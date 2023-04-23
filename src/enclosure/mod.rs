use std::collections::{HashMap, HashSet};
use std::ffi::CString;
use std::fs::{read_to_string, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::exit;
use std::sync::mpsc::channel;
use std::thread;
use std::time::Duration;

use color_eyre::Result;
use dotenv_parser::parse_dotenv;
use haikunator::Haikunator;
use log::*;
use nix::mount::{umount2, MntFlags};
use nix::sched::{clone, CloneFlags};
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
use nix::sys::{ptrace, signal};
use nix::unistd::{chdir, chroot, getgrouplist, getpid, pivot_root, Gid, Pid, User};
use owo_colors::colors::xterm::PinkSalmon;
use owo_colors::OwoColorize;
use rlimit::Resource;

use crate::config::BoxxyConfig;
use crate::enclosure::tracer::Tracer;

use self::fs::{append_all, FsDriver};
use self::rule::{Rule, RuleMode};

pub mod fs;
mod linux;
mod register;
pub mod rule;
mod syscall;
mod tracer;

pub struct Enclosure {
    config: BoxxyConfig,
    fs: FsDriver,
    name: String,
    child_exit_status: i32,
    created_files: Vec<PathBuf>,
    created_directories: Vec<PathBuf>,
}

impl Enclosure {
    pub fn new(config: BoxxyConfig) -> Self {
        Self {
            config,
            fs: FsDriver::new(),
            name: Haikunator::default().haikunate(),
            child_exit_status: -1,
            created_files: vec![],
            created_directories: vec![],
        }
    }

    pub fn run(&mut self) -> Result<()> {
        // Prepare the filesystem
        let applicable_rules = &self
            .config
            .rules
            .get_all_applicable_rules(self.config.command.get_program(), &self.fs)?;
        self.set_up_temporary_files(applicable_rules)?;

        // Set up the container: callback, stack, etc.
        let callback = || self.run_in_container(applicable_rules).unwrap();

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
            CloneFlags::CLONE_NEWNS | CloneFlags::CLONE_NEWUSER,
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

        // Restart stopped child if not tracing
        if self.config.trace {
            self.run_with_tracing(pid)?;
        } else {
            ptrace::detach(pid, None)?;
            self.run_without_tracing(pid)?;
        }

        Ok(())
    }

    #[allow(unreachable_code)]
    fn run_with_tracing(&mut self, pid: Pid) -> Result<()> {
        Tracer::flag(pid)?;
        let (tx, rx) = channel();

        debug!("restarting child and starting tracer!");
        ptrace::syscall(pid, None)?;
        Tracer::new(pid).run(tx)?;
        debug!("tracing finished!");

        match waitpid(pid, None)? {
            WaitStatus::Exited(_pid, status) => {
                self.child_exit_status = status;
            }
            _ => unreachable!("child should have exited!"),
        }

        let mut buffer = String::new();
        let mut seen_paths = HashSet::new();
        let mut counter = 0;
        {
            use std::fmt::Write;
            while let Ok(syscall) = rx.recv() {
                if let Some(path) = syscall.path {
                    let container_root = self.fs.container_root(&self.name);

                    if path.starts_with(&container_root) && !seen_paths.contains(&path) {
                        writeln!(buffer, "/{}", path.strip_prefix(&container_root)?.display())?;
                        seen_paths.insert(path);
                        counter += 1;
                    }
                }
            }
            writeln!(buffer, "# total: {counter}")?;
        }

        let mut file = File::create("./boxxy-report.txt")?;
        file.write_all(buffer.as_bytes())?;
        info!("wrote trace report to boxxy-report.txt");

        exit(self.child_exit_status);
    }

    fn run_without_tracing(&mut self, pid: Pid) -> Result<()> {
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
        self.clean_up_container()?;

        // All done! Return the child's exit status
        debug!("exiting with status {}", self.child_exit_status);
        exit(self.child_exit_status);
    }

    fn set_up_temporary_files(&mut self, applicable_rules: &[Rule]) -> Result<Vec<PathBuf>> {
        for rule in applicable_rules {
            debug!("processing path creation for rule '{}'", rule.name);

            let expanded_target = self.fs.fully_expand_path(&rule.target)?;
            let target_path = self.fs.maybe_resolve_symlink(&expanded_target)?;

            let rewrite_path = self.fs.fully_expand_path(&rule.rewrite)?;

            debug!("ensuring path: {target_path:?}");
            debug!("rewriting to: {rewrite_path:?}");

            match rule.mode {
                RuleMode::File => {
                    self.ensure_file(&rewrite_path)?;
                    if self.ensure_file(&target_path)? {
                        self.created_files.push(target_path.clone());
                    }
                }
                RuleMode::Directory => {
                    self.ensure_directory(&rewrite_path)?;
                    if self.ensure_directory(&target_path)? {
                        self.created_directories.push(target_path.clone());
                    }
                }
            }

            debug!("rewrote base bath {rewrite_path:?} => {target_path:?}");
        }

        Ok(vec![])
    }

    fn set_up_container(&mut self, applicable_rules: &[Rule]) -> Result<()> {
        // Load .env vars
        if self.config.dotenv {
            debug!("dotenv enabled!");
            if let Ok(dotenv_file) = dotenv::dotenv() {
                debug!("found dotenv path: {dotenv_file:?}");
                info!("loading env vars from {}", dotenv_file.display());
                // TODO: bleh error handling
                let dotenv = parse_dotenv(&read_to_string(dotenv_file)?).unwrap();
                for (key, value) in dotenv.iter() {
                    self.config.command.env(key, value);
                    debug!("loaded env var: {}=********", key);
                }
                info!("loaded {} env vars", dotenv.len());
            }
        }

        // Load env vars from applicable rules
        for rule in applicable_rules {
            for (key, value) in rule.env.iter() {
                self.config.command.env(key, value);
                debug!("loaded env var: {}=********", key);
            }
            if !rule.env.is_empty() {
                info!(
                    "loaded {} env vars from rule '{}'",
                    rule.env.len(),
                    rule.name
                );
            }
        }

        // Mount root RW
        debug!("setup root");
        self.fs.setup_root(&self.name)?;
        let container_root = self.fs.container_root(&self.name);
        debug!("bind mount root rw");
        self.fs.bind_mount_rw(Path::new("/"), &container_root)?;

        // Apply all rules via bind mounts
        info!("applying {} rules", applicable_rules.len());
        for rule in applicable_rules {
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
            let target_path = self.fs.maybe_resolve_symlink(&target_path)?;

            let rewrite_path = self.fs.fully_expand_path(&rule.rewrite)?;

            debug!("source exists: {}", rewrite_path.exists());
            debug!("target exists: {}", target_path.exists());

            // If the target file doesn't exist, we have to create it in order to bind mount over it.
            match rule.mode {
                RuleMode::File => {
                    if !target_path.exists() {
                        debug!("creating file: {target_path:?}");
                        self.ensure_file(&target_path)?;
                        self.created_files.push(target_path.clone());
                    }
                    self.fs.bind_mount_rw(&rewrite_path, &target_path)?;
                }
                RuleMode::Directory => {
                    if !target_path.exists() {
                        debug!("creating directory: {target_path:?}");
                        self.ensure_directory(&target_path)?;
                        self.created_files.push(target_path.clone());
                    }
                    self.fs.bind_mount_rw(&rewrite_path, &target_path)?;
                }
            }

            debug!("rewrote base bath {rewrite_path:?} => {target_path:?}");
        }

        Ok(())
    }

    fn clean_up_container(&mut self) -> Result<()> {
        debug!(
            "{}",
            format!(
                "cleaning up {} path(s) ♥",
                self.created_directories.len() + self.created_files.len()
            )
            .if_supports_color(owo_colors::Stream::Stdout, |text| text.fg::<PinkSalmon>())
        );
        for file in &self.created_files {
            debug!("removing temporary file {}", file.display());
            std::fs::remove_file(file)?;
        }
        for dir in &self.created_directories {
            debug!("removing temporary directory {}", dir.display());
            std::fs::remove_dir(dir)?;
        }

        Ok(())
    }

    fn run_in_container(&mut self, applicable_rules: &[Rule]) -> Result<isize> {
        self.set_up_container(applicable_rules)?;

        let pwd = std::env::current_dir()?;

        if self.config.trace {
            chroot(&self.fs.container_root(&self.name))?;
            chdir(&pwd)?;
        } else {
            chdir(&self.fs.container_root(&self.name))?;
            pivot_root(".", ".")?;
            umount2(".", MntFlags::MNT_DETACH)?;
            chdir(&pwd)?;
        }

        // Remount rootfs as ro
        if self.config.immutable_root {
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
        debug!("running command: {:?}", self.config.command.get_program());
        info!(
            "{}",
            format!("boxed {:?} ♥", self.config.command.get_program())
                .if_supports_color(owo_colors::Stream::Stdout, |text| text.fg::<PinkSalmon>())
        );
        let result = self.config.command.spawn()?.wait()?;

        debug!("command exited with status: {:?}", result);

        Ok(result.code().map(|c| c as isize).unwrap_or(0isize))
    }

    fn ensure_file(&self, path: &Path) -> Result<bool> {
        if !path.exists() {
            if let Some(parent) = path.parent() {
                if !parent.exists() {
                    self.fs.touch_dir(parent)?;
                }
            }
            self.fs.touch(path)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn ensure_directory(&self, path: &Path) -> Result<bool> {
        if !path.exists() {
            self.fs.touch_dir(path)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}
