use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};

use color_eyre::Result;
use log::*;
use nix::mount::{mount, MsFlags};

pub struct FsDriver;

#[allow(unused)]
impl FsDriver {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {}
    }

    pub fn all_containers_root(&self) -> PathBuf {
        PathBuf::from("/tmp/boxxy-containers")
    }

    pub fn container_root(&self, name: &str) -> PathBuf {
        append_all(&self.all_containers_root(), vec![name])
    }

    pub fn setup_root(&self, name: &str) -> Result<()> {
        fs::create_dir_all(self.container_root(name))?;
        Ok(())
    }

    pub fn cleanup_root(&self, name: &str) -> Result<()> {
        fs::remove_dir_all(self.container_root(name))?;
        Ok(())
    }

    pub fn bind_mount_ro(&self, src: &Path, target: &Path) -> Result<()> {
        // ro bindmount is a complicated procedure: https://unix.stackexchange.com/a/128388
        // tldr: You first do a normal bindmount, then remount bind+ro
        self.bind_mount(src, target, MsFlags::MS_BIND)?;
        self.remount_ro(target)?;
        Ok(())
    }

    pub fn remount_ro(&self, target: &Path) -> Result<()> {
        debug!("remount {target:?} as ro");
        mount::<Path, Path, str, str>(
            None,
            target,
            Some(""),
            MsFlags::MS_REMOUNT | MsFlags::MS_BIND | MsFlags::MS_RDONLY,
            Some(""),
        )?;
        Ok(())
    }

    pub fn bind_mount_rw(&self, src: &Path, target: &Path) -> Result<()> {
        self.bind_mount(src, target, MsFlags::MS_BIND)
    }

    fn bind_mount(&self, src: &Path, target: &Path, flags: MsFlags) -> Result<()> {
        debug!("bind mount {src:?} onto {target:?}");
        mount(
            Some(src),
            target,
            Some(""),
            MsFlags::MS_REC | flags,
            Some(""),
        )?;
        Ok(())
    }

    pub fn touch(&self, path: &Path) -> Result<()> {
        match OpenOptions::new().create(true).write(true).open(path) {
            Ok(_) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }

    pub fn touch_dir(&self, path: &Path) -> Result<()> {
        match fs::create_dir_all(path) {
            Ok(_) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }

    pub fn touch_dir_sync(&self, path: &Path) -> Result<()> {
        match fs::create_dir_all(path) {
            Ok(_) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
}

pub fn append_all(buf: &Path, parts: Vec<&str>) -> PathBuf {
    let mut buf = buf.to_path_buf();
    for part in parts {
        let part = match part.strip_prefix('/') {
            Some(p) => p,
            None => part,
        };
        buf.push(part);
    }
    buf
}
