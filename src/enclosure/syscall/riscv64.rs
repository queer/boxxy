use std::collections::HashMap;

use crate::enclosure::register::StringRegister;

lazy_static::lazy_static! {
    pub static ref SYSCALL_REGISTERS: HashMap<i64, StringRegister> = {
        let mut m = HashMap::new();
        // read/write
        m.insert(libc::SYS_read, StringRegister::A0);
        m.insert(libc::SYS_write, StringRegister::A0);

        // openat
        m.insert(libc::SYS_openat, StringRegister::A1);

        // close
        m.insert(libc::SYS_close, StringRegister::A0);

        // unlinkat
        m.insert(libc::SYS_unlinkat, StringRegister::A1);

        // fstat
        m.insert(libc::SYS_fstat, StringRegister::A0);
        // statx
        m.insert(libc::SYS_statx, StringRegister::A0);
        // newfstatat
        m.insert(libc::SYS_newfstatat, StringRegister::A0);

        // lseek
        m.insert(libc::SYS_lseek, StringRegister::A0);

        // pread64/pwrite64/preadv/pwritev
        m.insert(libc::SYS_pread64, StringRegister::A0);
        m.insert(libc::SYS_pwrite64, StringRegister::A0);
        m.insert(libc::SYS_preadv, StringRegister::A0);
        m.insert(libc::SYS_pwritev, StringRegister::A0);

        // faccessat/faccessat2
        m.insert(libc::SYS_faccessat, StringRegister::A1);
        m.insert(libc::SYS_faccessat2, StringRegister::A1);

        // dup/dup3
        m.insert(libc::SYS_dup, StringRegister::A0);
        m.insert(libc::SYS_dup3, StringRegister::A0);

        // sendfile
        m.insert(libc::SYS_sendfile, StringRegister::A0);

        // fcntl
        m.insert(libc::SYS_fcntl, StringRegister::A0);

        // fsync/fdatasync
        m.insert(libc::SYS_fsync, StringRegister::A0);
        m.insert(libc::SYS_fdatasync, StringRegister::A0);

        // truncate/ftruncate
        m.insert(libc::SYS_truncate, StringRegister::A0);
        m.insert(libc::SYS_ftruncate, StringRegister::A0);

        // getdents64
        m.insert(libc::SYS_getdents64, StringRegister::A0);

        // chdir/fchdir
        m.insert(libc::SYS_chdir, StringRegister::A0);
        m.insert(libc::SYS_fchdir, StringRegister::A0);

        // renameat2
        // TODO: add renameat2 to x86_64
        m.insert(libc::SYS_renameat2, StringRegister::A1);

        // mkdirat
        m.insert(libc::SYS_mkdirat, StringRegister::A1);

        // linkat/symlinkat/unlinkat
        m.insert(libc::SYS_linkat, StringRegister::A1);
        m.insert(libc::SYS_symlinkat, StringRegister::A1);
        m.insert(libc::SYS_unlinkat, StringRegister::A0);

        // fchmod/fchown
        m.insert(libc::SYS_fchmod, StringRegister::A0);
        m.insert(libc::SYS_fchown, StringRegister::A0);

        // fchownat/fchmodat
        m.insert(libc::SYS_fchownat, StringRegister::A1);
        m.insert(libc::SYS_fchmodat, StringRegister::A1);

        // mknodat
        m.insert(libc::SYS_mknodat, StringRegister::A1);

        // pivot_root
        m.insert(libc::SYS_pivot_root, StringRegister::A0);

        // chroot
        m.insert(libc::SYS_chroot, StringRegister::A0);

        // mount/umount2
        m.insert(libc::SYS_mount, StringRegister::A0);
        m.insert(libc::SYS_umount2, StringRegister::A0);

        // swapon/swapoff
        m.insert(libc::SYS_swapon, StringRegister::A0);
        m.insert(libc::SYS_swapoff, StringRegister::A0);

        // readahead
        m.insert(libc::SYS_readahead, StringRegister::A0);

        // setxattr/lsetxattr/fsetxattr/getxattr/lgetxattr/fgetxattr/listxattr/llistxattr/flistxattr/removexattr/lremovexattr/fremovexattr
        m.insert(libc::SYS_setxattr, StringRegister::A0);
        m.insert(libc::SYS_lsetxattr, StringRegister::A0);
        m.insert(libc::SYS_fsetxattr, StringRegister::A0);
        m.insert(libc::SYS_getxattr, StringRegister::A0);
        m.insert(libc::SYS_lgetxattr, StringRegister::A0);
        m.insert(libc::SYS_fgetxattr, StringRegister::A0);
        m.insert(libc::SYS_listxattr, StringRegister::A0);
        m.insert(libc::SYS_llistxattr, StringRegister::A0);
        m.insert(libc::SYS_flistxattr, StringRegister::A0);
        m.insert(libc::SYS_removexattr, StringRegister::A0);
        m.insert(libc::SYS_lremovexattr, StringRegister::A0);
        m.insert(libc::SYS_fremovexattr, StringRegister::A0);

        // fadvise64
        m.insert(libc::SYS_fadvise64, StringRegister::A0);

        // utimensat
        m.insert(libc::SYS_utimensat, StringRegister::A0);

        // splice/tee
        m.insert(libc::SYS_splice, StringRegister::A0);
        m.insert(libc::SYS_tee, StringRegister::A0);

        // sync_file_range
        m.insert(libc::SYS_sync_file_range, StringRegister::A0);

        // vmsplice
        m.insert(libc::SYS_vmsplice, StringRegister::A0);

        // fallocate
        m.insert(libc::SYS_fallocate, StringRegister::A0);

        // inotify_init1/fanotify_init/fanonotify_mark
        m.insert(libc::SYS_inotify_init1, StringRegister::A0);
        m.insert(libc::SYS_fanotify_init, StringRegister::A0);
        m.insert(libc::SYS_fanotify_mark, StringRegister::A0);

        // name_to_handle_at/open_by_handle_at
        m.insert(libc::SYS_name_to_handle_at, StringRegister::A0);
        m.insert(libc::SYS_open_by_handle_at, StringRegister::A0);

        // syncfs
        m.insert(libc::SYS_syncfs, StringRegister::A0);

        m
    };
}
