use std::collections::HashMap;

use crate::enclosure::register::StringRegister;

lazy_static::lazy_static! {
    pub static ref SYSCALL_REGISTERS: HashMap<i64, StringRegister> = {
        let mut m = HashMap::new();
        // read/write
        m.insert(libc::SYS_read, StringRegister::Rdi);
        m.insert(libc::SYS_write, StringRegister::Rdi);

        // open/openat/creat
        m.insert(libc::SYS_openat, StringRegister::Rsi);
        m.insert(libc::SYS_open, StringRegister::Rdi);
        m.insert(libc::SYS_creat, StringRegister::Rdi);

        // close
        m.insert(libc::SYS_close, StringRegister::Rdi);

        // unlink/unlinkat
        m.insert(libc::SYS_unlinkat, StringRegister::Rsi);
        m.insert(libc::SYS_unlink, StringRegister::Rdi);

        // stat/fstat/lstat
        m.insert(libc::SYS_stat, StringRegister::Rdi);
        m.insert(libc::SYS_fstat, StringRegister::Rdi);
        m.insert(libc::SYS_lstat, StringRegister::Rdi);
        // statx
        m.insert(libc::SYS_statx, StringRegister::Rdi);
        // newfstatat
        m.insert(libc::SYS_newfstatat, StringRegister::Rdi);

        // lseek
        m.insert(libc::SYS_lseek, StringRegister::Rdi);

        // pread64/pwrite64/preadv/pwritev
        m.insert(libc::SYS_pread64, StringRegister::Rdi);
        m.insert(libc::SYS_pwrite64, StringRegister::Rdi);
        m.insert(libc::SYS_preadv, StringRegister::Rdi);
        m.insert(libc::SYS_pwritev, StringRegister::Rdi);

        // access/faccessat/faccessat2
        m.insert(libc::SYS_access, StringRegister::Rdi);
        m.insert(libc::SYS_faccessat, StringRegister::Rsi);
        m.insert(libc::SYS_faccessat2, StringRegister::Rsi);

        // dup/dup2/dup3
        m.insert(libc::SYS_dup, StringRegister::Rdi);
        m.insert(libc::SYS_dup2, StringRegister::Rdi);
        m.insert(libc::SYS_dup3, StringRegister::Rdi);

        // sendfile
        m.insert(libc::SYS_sendfile, StringRegister::Rdi);

        // fcntl
        m.insert(libc::SYS_fcntl, StringRegister::Rdi);

        // fsync/fdatasync
        m.insert(libc::SYS_fsync, StringRegister::Rdi);
        m.insert(libc::SYS_fdatasync, StringRegister::Rdi);

        // truncate/ftruncate
        m.insert(libc::SYS_truncate, StringRegister::Rdi);
        m.insert(libc::SYS_ftruncate, StringRegister::Rdi);

        // getdents/getdents64
        m.insert(libc::SYS_getdents, StringRegister::Rdi);
        m.insert(libc::SYS_getdents64, StringRegister::Rdi);

        // chdir/fchdir
        m.insert(libc::SYS_chdir, StringRegister::Rdi);
        m.insert(libc::SYS_fchdir, StringRegister::Rdi);

        // rename/renameat
        m.insert(libc::SYS_rename, StringRegister::Rdi);
        m.insert(libc::SYS_renameat, StringRegister::Rsi);

        // mkdir/rmdir/mkdirat
        m.insert(libc::SYS_mkdir, StringRegister::Rdi);
        m.insert(libc::SYS_rmdir, StringRegister::Rdi);
        m.insert(libc::SYS_mkdirat, StringRegister::Rsi);

        // link/unlink/symlink/readlink/linkat/symlinkat/unlinkat
        m.insert(libc::SYS_link, StringRegister::Rsi);
        m.insert(libc::SYS_unlink, StringRegister::Rdi);
        m.insert(libc::SYS_symlink, StringRegister::Rdi);
        m.insert(libc::SYS_readlink, StringRegister::Rdi);
        m.insert(libc::SYS_linkat, StringRegister::Rsi);
        m.insert(libc::SYS_symlinkat, StringRegister::Rsi);
        m.insert(libc::SYS_unlinkat, StringRegister::Rdi);

        // chmod/fchmod/chown/fchown/lchown
        m.insert(libc::SYS_chmod, StringRegister::Rdi);
        m.insert(libc::SYS_fchmod, StringRegister::Rdi);
        m.insert(libc::SYS_chown, StringRegister::Rdi);
        m.insert(libc::SYS_fchown, StringRegister::Rdi);
        m.insert(libc::SYS_lchown, StringRegister::Rdi);
        // fchownat/fchmodat
        m.insert(libc::SYS_fchownat, StringRegister::Rsi);
        m.insert(libc::SYS_fchmodat, StringRegister::Rsi);

        // mknod/mknodat
        m.insert(libc::SYS_mknod, StringRegister::Rdi);
        m.insert(libc::SYS_mknodat, StringRegister::Rsi);

        // pivot_root
        m.insert(libc::SYS_pivot_root, StringRegister::Rdi);

        // chroot
        m.insert(libc::SYS_chroot, StringRegister::Rdi);

        // mount/umount2
        m.insert(libc::SYS_mount, StringRegister::Rdi);
        m.insert(libc::SYS_umount2, StringRegister::Rdi);

        // swapon/swapoff
        m.insert(libc::SYS_swapon, StringRegister::Rdi);
        m.insert(libc::SYS_swapoff, StringRegister::Rdi);

        // readahead
        m.insert(libc::SYS_readahead, StringRegister::Rdi);

        // setxattr/lsetxattr/fsetxattr/getxattr/lgetxattr/fgetxattr/listxattr/llistxattr/flistxattr/removexattr/lremovexattr/fremovexattr
        m.insert(libc::SYS_setxattr, StringRegister::Rdi);
        m.insert(libc::SYS_lsetxattr, StringRegister::Rdi);
        m.insert(libc::SYS_fsetxattr, StringRegister::Rdi);
        m.insert(libc::SYS_getxattr, StringRegister::Rdi);
        m.insert(libc::SYS_lgetxattr, StringRegister::Rdi);
        m.insert(libc::SYS_fgetxattr, StringRegister::Rdi);
        m.insert(libc::SYS_listxattr, StringRegister::Rdi);
        m.insert(libc::SYS_llistxattr, StringRegister::Rdi);
        m.insert(libc::SYS_flistxattr, StringRegister::Rdi);
        m.insert(libc::SYS_removexattr, StringRegister::Rdi);
        m.insert(libc::SYS_lremovexattr, StringRegister::Rdi);
        m.insert(libc::SYS_fremovexattr, StringRegister::Rdi);

        // fadvise64
        m.insert(libc::SYS_fadvise64, StringRegister::Rdi);

        // futimesat/utimensat
        m.insert(libc::SYS_futimesat, StringRegister::Rdi);
        m.insert(libc::SYS_utimensat, StringRegister::Rdi);

        // splice/tee
        m.insert(libc::SYS_splice, StringRegister::Rdi);
        m.insert(libc::SYS_tee, StringRegister::Rdi);

        // sync_file_range
        m.insert(libc::SYS_sync_file_range, StringRegister::Rdi);

        // vmsplice
        m.insert(libc::SYS_vmsplice, StringRegister::Rdi);

        // fallocate
        m.insert(libc::SYS_fallocate, StringRegister::Rdi);

        // inotify_init1/fanotify_init/fanonotify_mark
        m.insert(libc::SYS_inotify_init1, StringRegister::Rdi);
        m.insert(libc::SYS_fanotify_init, StringRegister::Rdi);
        m.insert(libc::SYS_fanotify_mark, StringRegister::Rdi);

        // name_to_handle_at/open_by_handle_at
        m.insert(libc::SYS_name_to_handle_at, StringRegister::Rdi);
        m.insert(libc::SYS_open_by_handle_at, StringRegister::Rdi);

        // syncfs
        m.insert(libc::SYS_syncfs, StringRegister::Rdi);

        m
    };
}
