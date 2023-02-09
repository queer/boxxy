use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::Duration;

use color_eyre::Result;
use haikunator::Haikunator;
use log::*;
use nix::sched::{clone, CloneFlags};
use nix::sys::wait::{waitpid, WaitStatus};
use nix::unistd::{chdir, chroot};
use owo_colors::colors::xterm::PinkSalmon;
use owo_colors::OwoColorize;
use rlimit::Resource;

use self::fs::{append_all, FsDriver};
use self::rule::{Rule, RuleMode, Rules};

pub mod fs;
pub mod rule;

pub struct Enclosure<'a> {
    pub command: &'a mut Command,
    fs: FsDriver,
    name: String,
    rules: Rules,
}

impl<'a> Enclosure<'a> {
    pub fn new(rules: Rules, command: &'a mut Command) -> Self {
        Self {
            command,
            fs: FsDriver::new(),
            name: Haikunator::default().haikunate(),
            rules,
        }
    }

    pub fn run(&mut self) -> Result<()> {
        // Set up the container: callback, stack, etc.
        let callback = || self.run_in_container().map(|_| 0).unwrap();

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

        // Set up ^C handling
        let name_clone = self.name.clone();
        let pid_clone = pid.as_raw();
        #[allow(unused_must_use)]
        ctrlc::set_handler(move || {
            nix::sys::signal::kill(
                nix::unistd::Pid::from_raw(pid_clone),
                nix::sys::signal::SIGTERM,
            );
            FsDriver::new().cleanup_root(&name_clone).unwrap();
        })?;

        // Wait for exit
        loop {
            match waitpid(pid, None) {
                Ok(WaitStatus::Exited(_pid, _status)) => {
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

        // Clean up!
        self.fs.cleanup_root(&self.name)?;

        Ok(())
    }

    fn run_in_container(&mut self) -> Result<()> {
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

            info!("applying rule '{}'", rule.name);

            let expanded_target = {
                let expanded = shellexpand::tilde(&rule.target).to_string();
                let canonicalised_path = Path::new(&expanded).canonicalize()?;
                canonicalised_path
            };

            let target_path = append_all(&container_root, vec![&expanded_target]).canonicalize()?;
            let target_path = target_path.as_path();

            let rewrite_path = shellexpand::tilde(&rule.rewrite).to_string();
            let rewrite_path = Path::new(&rewrite_path).canonicalize()?;

            match rule.mode {
                RuleMode::File => {
                    self.ensure_file(&rewrite_path)?;
                    self.ensure_file(target_path)?;
                    self.fs.bind_mount_rw(&rewrite_path, target_path)?;
                }
                RuleMode::Directory => {
                    self.ensure_directory(&rewrite_path)?;
                    self.ensure_directory(target_path)?;
                    self.fs.bind_mount_rw(&rewrite_path, target_path)?;
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
        debug!("remounting rootfs as ro!");
        self.fs.remount_ro(Path::new("/"))?;

        debug!(
            "chrooted to {}",
            self.fs.container_root(&self.name).display()
        );

        // Do the needful!
        debug!("running command: {:?}", self.command.get_program());
        info!(
            "{}",
            format!("boxed {:?} ♥", self.command.get_program())
                .if_supports_color(owo_colors::Stream::Stdout, |text| text.fg::<PinkSalmon>())
        );
        self.command.spawn()?.wait()?;

        Ok(())
    }

    fn ensure_file(&self, path: &Path) -> Result<()> {
        self.fs.touch_dir(path.parent().unwrap())?;
        self.fs.touch(path)?;

        Ok(())
    }

    fn ensure_directory(&self, path: &Path) -> Result<()> {
        self.fs.touch_dir(path)?;

        Ok(())
    }

    fn currently_in_context(&self, rule: &Rule) -> Result<bool> {
        if rule.context.is_empty() {
            return Ok(true);
        }

        for context in &rule.context {
            let expanded_context = shellexpand::tilde(&context).to_string();
            let expanded_context = Path::new(&expanded_context).canonicalize()?;

            let pwd = std::env::current_dir()?;

            if pwd.starts_with(&expanded_context) {
                return Ok(true);
            }
        }

        Ok(false)
    }
}
