use std::collections::HashMap;
use std::ffi::CString;
use std::path::Path;
use std::process::{exit, Command};
use std::thread;
use std::time::Duration;

use color_eyre::Result;
use haikunator::Haikunator;
use log::*;
use nix::sched::{clone, CloneFlags};
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
use nix::sys::{ptrace, signal};
use nix::unistd::{chdir, chroot, getgrouplist, getpid, Gid, Uid, User};
use owo_colors::colors::xterm::PinkSalmon;
use owo_colors::OwoColorize;
use rlimit::Resource;

use self::fs::{append_all, FsDriver};
use self::rule::{Rule, RuleMode, Rules};

pub mod fs;
mod linux;
pub mod rule;

pub struct Enclosure<'a> {
    command: &'a mut Command,
    fs: FsDriver,
    name: String,
    rules: Rules,
    immutable_root: bool,
    child_exit_status: i32,
}

pub struct Opts<'a> {
    pub rules: Rules,
    pub command: &'a mut Command,
    pub immutable_root: bool,
}

impl<'a> Enclosure<'a> {
    pub fn new(opts: Opts<'a>) -> Self {
        Self {
            command: opts.command,
            fs: FsDriver::new(),
            name: Haikunator::default().haikunate(),
            rules: opts.rules,
            immutable_root: opts.immutable_root,
            child_exit_status: -1,
        }
    }

    pub fn run(&mut self) -> Result<()> {
        // Set up the container: callback, stack, etc.
        let callback = || self.run_in_container().unwrap();

        let stack_size = match Resource::STACK.get() {
            Ok((soft, _hard)) => soft as usize,
            Err(_) => {
                // 8MB
                8 * 1024 * 1024
            }
        };

        let mut stack_vec = vec![0u8; stack_size];
        let stack: &mut [u8] = stack_vec.as_mut_slice();

        // Clone off the container process
        let pid = clone(
            Box::new(callback),
            stack,
            CloneFlags::CLONE_NEWNS | CloneFlags::CLONE_NEWUSER | CloneFlags::CLONE_NEWPID,
            Some(nix::sys::signal::Signal::SIGCHLD as i32),
        )?;
        if pid.as_raw() == -1 {
            return Err(std::io::Error::last_os_error().into());
        }

        // Await PTRACE_TRACEME from child
        waitpid(pid, Some(WaitPidFlag::WSTOPPED))?;
        debug!("child stopped!");

        // Map current UID + GID into the container so that things continue to
        // work as expected.

        // Get current UID + GID
        let uid = nix::unistd::geteuid();
        let gid = nix::unistd::getegid();

        // Call newuidmap + newgidmap

        // TODO: This is hacky. I don't like this.
        // It's... difficult... to map uids/gids properly. There is a proper
        // mechanism for doing so, but it's a part of the `shadow` package, and
        // I don't want to generate C bindings right now. Instead, this just
        // tries to map them over and over, removing broken uids/gids until it
        // happens to work.
        // This isn't optimal, but it works.
        if let Some(user) = User::from_uid(uid)? {
            let mut uid_map = HashMap::new();
            uid_map.insert(user.uid, user.uid);
            uid_map.insert(Uid::from_raw(0), Uid::from_raw(0));

            linux::map_uids(pid, &mut uid_map)?;

            let mut gid_map = HashMap::new();
            gid_map.insert(user.gid, user.gid);
            gid_map.insert(Gid::from_raw(0), Gid::from_raw(0));
            getgrouplist(&CString::new(user.name)?, gid)?
                .iter()
                .for_each(|gid| {
                    gid_map.insert(*gid, *gid);
                });

            linux::map_gids(pid, &mut gid_map)?;

            debug!("finished setting up uid/gid mapping");
        } else {
            unreachable!("it should be impossible to have a user that doesn't have your uid");
        }

        // Set up ^C handling
        let name_clone = self.name.clone();
        let pid_clone = pid.as_raw();
        #[allow(unused_must_use)]
        ctrlc::set_handler(move || {
            nix::sys::signal::kill(
                nix::unistd::Pid::from_raw(pid_clone),
                nix::sys::signal::SIGTERM,
            );
            FsDriver::new().cleanup_root(&name_clone);
            exit(1);
        })?;

        // Restart stopped child
        ptrace::detach(pid, None)?;

        // Wait for exit
        let mut exit_status: i32 = -1;
        loop {
            match waitpid(pid, None) {
                Ok(WaitStatus::Exited(_pid, status)) => {
                    exit_status = status;
                    break;
                }
                Err(nix::errno::Errno::ECHILD) => {
                    // We might need to wait to let stdout/err buffer
                    thread::sleep(Duration::from_millis(100));
                    break;
                }
                _ => thread::sleep(Duration::from_millis(100)),
            }
        }
        self.child_exit_status = exit_status;

        // Clean up!
        self.fs.cleanup_root(&self.name)?;

        // All done! Return the child's exit status
        debug!("exiting with status {}", self.child_exit_status);
        exit(self.child_exit_status);
    }

    fn run_in_container(&mut self) -> Result<isize> {
        // Mount root RW
        debug!("setup root");
        self.fs.setup_root(&self.name)?;
        let container_root = self.fs.container_root(&self.name);
        debug!("bind mount root rw");
        self.fs.bind_mount_rw(Path::new("/"), &container_root)?;

        // Apply all rules via bind mounts
        for rule in &self.rules.rules {
            debug!("processing rule '{}'", rule.name);

            if !self.currently_in_context(rule)? {
                debug!("not applying rule '{}' because of context", rule.name);
                continue;
            }

            if !self.applies_to_binary(rule)? {
                debug!("not applying rule '{}' because of binary", rule.name);
                continue;
            }

            info!("applying rule '{}'", rule.name);

            let expanded_target = self.fs.fully_expand_path(&rule.target)?;
            // Rewrite target path into the container
            let target_path =
                match append_all(&container_root, vec![&expanded_target]).canonicalize() {
                    Ok(path) => path,
                    Err(_) => {
                        // If the path doesn't exist, we'll create it
                        append_all(&container_root, vec![&expanded_target])
                    }
                };
            let target_path = target_path.as_path();
            let target_path = self.fs.maybe_resolve_symlink(target_path)?;

            let rewrite_path = self.fs.fully_expand_path(&rule.rewrite)?;

            match rule.mode {
                RuleMode::File => {
                    self.ensure_file(&rewrite_path)?;
                    self.ensure_file(&target_path)?;
                    self.fs.bind_mount_rw(&rewrite_path, &target_path)?;
                }
                RuleMode::Directory => {
                    self.ensure_directory(&rewrite_path)?;
                    self.ensure_directory(&target_path)?;
                    self.fs.bind_mount_rw(&rewrite_path, &target_path)?;
                }
            }

            info!("redirect: {} -> {}", rule.target, rule.rewrite);
            debug!("rewrote base bath {rewrite_path:?} => {target_path:?}");
        }

        // Chroot into container root
        let pwd = std::env::current_dir()?;
        chroot(&self.fs.container_root(&self.name))?;
        chdir(&pwd)?;

        // Remount rootfs as ro
        if self.immutable_root {
            debug!("remounting rootfs as ro!");
            self.fs.remount_ro(Path::new("/"))?;
        }

        debug!(
            "chrooted to {}",
            self.fs.container_root(&self.name).display()
        );

        // Initiate ptrace with the parent process
        ptrace::traceme()?;
        signal::kill(getpid(), signal::SIGSTOP)?;

        // Do the needful!
        debug!("running command: {:?}", self.command.get_program());
        info!(
            "{}",
            format!("boxed {:?} ♥", self.command.get_program())
                .if_supports_color(owo_colors::Stream::Stdout, |text| text.fg::<PinkSalmon>())
        );
        let result = self.command.spawn()?.wait()?;

        Ok(result.code().map(|c| c as isize).unwrap_or(0isize))
    }

    fn ensure_file(&self, path: &Path) -> Result<()> {
        if !path.exists() {
            if let Some(parent) = path.parent() {
                if !parent.exists() {
                    self.fs.touch_dir(parent)?;
                }
            }
            self.fs.touch(path)?;
        }

        Ok(())
    }

    fn ensure_directory(&self, path: &Path) -> Result<()> {
        if !path.exists() {
            self.fs.touch_dir(path)?;
        }

        Ok(())
    }

    fn currently_in_context(&self, rule: &Rule) -> Result<bool> {
        if rule.context.is_empty() {
            return Ok(true);
        }

        for context in &rule.context {
            debug!("{}: resolving context: {}", rule.name, context);
            let expanded_context = shellexpand::tilde(&context).to_string();
            let expanded_context = Path::new(&expanded_context).canonicalize()?;
            let resolved_context = self.fs.maybe_resolve_symlink(&expanded_context)?;

            let pwd = std::env::current_dir()?;

            debug!(
                "{}: {} <> {}",
                rule.name,
                pwd.display(),
                resolved_context.display()
            );

            if pwd.starts_with(&resolved_context) {
                return Ok(true);
            }
        }

        Ok(false)
    }

    fn applies_to_binary(&self, rule: &Rule) -> Result<bool> {
        if rule.only.is_empty() {
            return Ok(true);
        }

        let program = self.command.get_program();

        for binary in &rule.only {
            debug!("{}: resolving binary: {}", rule.name, binary);
            let expanded_binary = self.fs.fully_expand_path(binary)?;
            let resolved_binary = self.fs.maybe_resolve_symlink(&expanded_binary)?;

            if program == resolved_binary.file_name().unwrap() {
                return Ok(true);
            }
        }

        Ok(false)
    }
}
