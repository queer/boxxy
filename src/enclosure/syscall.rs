use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use color_eyre::eyre::eyre;
use color_eyre::Result;
use log::*;
use nix::unistd::Pid;

use super::tracer::{ChildProcess, PtraceRegisters, StringRegister, Tracer};

lazy_static::lazy_static! {
    static ref SYSCALL_REGISTERS: HashMap<i64, StringRegister> = {
        let mut m = HashMap::new();
        // open/openat
        m.insert(libc::SYS_openat, StringRegister::Rsi);
        m.insert(libc::SYS_open, StringRegister::Rdi);

        // unlink/unlinkat
        m.insert(libc::SYS_unlinkat, StringRegister::Rsi);
        m.insert(libc::SYS_unlink, StringRegister::Rdi);

        // newfstatat
        m.insert(libc::SYS_newfstatat, StringRegister::Rsi);

        // faccessat2
        m.insert(libc::SYS_faccessat2, StringRegister::Rsi);

        m
    };
}

#[derive(Debug, Clone)]
pub struct Syscall {
    pub name: String,
    pub number: u64,
}

pub fn handle_syscall(tracer: &Tracer, pid: Pid) -> Result<Option<Syscall>> {
    let child = match tracer.get_child(pid) {
        Some(child) => child,
        None => unreachable!(
            "should never get a child from the tracer that the tracer doesn't know about"
        ),
    };
    let registers = child.get_registers()?;
    let syscall_no = registers.orig_rax;
    if let Some(syscall_name) = syscall_numbers::native::sys_call_name(syscall_no.try_into()?) {
        let syscall = Syscall {
            name: syscall_name.to_string(),
            number: syscall_no,
        };

        if let Some(path) = get_path_from_syscall(child, syscall_no, &mut registers.clone())? {
            info!("{}({})", syscall.name, path);
        } else {
            warn!("{} has unknown path!?", syscall.name);
        }

        Ok(Some(syscall))
    } else {
        Ok(None)
    }
}

fn get_path_from_syscall(
    child: &ChildProcess,
    syscall_no: u64,
    registers: &mut PtraceRegisters,
) -> Result<Option<String>> {
    if let Some(register) = SYSCALL_REGISTERS.get(&(syscall_no as i64)) {
        let path_ptr = match register {
            StringRegister::Rdi => registers.rdi,
            StringRegister::Rsi => registers.rsi,
            StringRegister::Rdx => registers.rdx,
            StringRegister::Rcx => registers.rcx,
            StringRegister::R8 => registers.r8,
            StringRegister::R9 => registers.r9,
        };
        let path = child.read_string(register, path_ptr as *mut _)?;

        Ok(Some(path))
    } else {
        warn!(
            "unable to handle syscall {syscall_no} ({:?})!",
            syscall_numbers::native::sys_call_name(syscall_no.try_into()?)
        );
        Ok(None)
    }
}

#[allow(unused)]
fn get_fd_path(pid: Pid, fd: i32) -> Result<PathBuf> {
    let fd_path = format!("/proc/{pid}/fd/{fd}");
    let out_path = fs::read_link(fd_path)?;
    if out_path.starts_with("pipe:[") {
        Err(eyre!("cannot rewrite pipes"))
    } else {
        Ok(out_path)
    }
}
