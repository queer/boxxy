use cfg_if::cfg_if;
use color_eyre::Result;
use nix::unistd::Pid;
use std::{fs, path::PathBuf};

#[allow(unused)]
fn get_fd_path(pid: Pid, fd: i32) -> Result<Option<PathBuf>> {
    let fd_path = format!("/proc/{pid}/fd/{fd}");
    match fs::read_link(fd_path) {
        Ok(path) => {
            if path.starts_with("pipe:[") {
                Ok(None)
            } else {
                Ok(Some(path))
            }
        }
        Err(_) => Ok(None),
    }
}

cfg_if! {
    if #[cfg(target_arch = "x86_64")] {
        mod x86_64;
        pub use x86_64::*;
    } else {
        compile_error!("The current architecture is unsupported!");
    }
}
