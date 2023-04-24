use cfg_if::cfg_if;
use color_eyre::Result;
use nix::unistd::Pid;
use std::{fs, path::PathBuf};

use super::{
    register::{get_register_from_regs, syscall_number_from_user_regs, StringRegister},
    tracer::{ChildProcess, PtraceRegisters, Tracer},
};

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

#[derive(Debug, Clone)]
pub struct Syscall {
    pub name: String,
    pub number: u64,
    pub path: Option<PathBuf>,
}

pub fn handle_syscall(tracer: &Tracer, pid: Pid) -> Result<Option<Syscall>> {
    let child = match tracer.get_child(pid) {
        Some(child) => child,
        None => unreachable!(
            "should never get a child from the tracer that the tracer doesn't know about"
        ),
    };
    let registers = child.get_registers()?;
    let syscall_no = syscall_number_from_user_regs!(registers);
    if let Some(syscall_name) = syscall_numbers::native::sys_call_name(syscall_no.try_into()?) {
        let path = get_path_from_syscall(child, syscall_no, &mut registers.clone())?;
        let syscall = Syscall {
            name: syscall_name.to_string(),
            number: syscall_no,
            path,
        };

        Ok(Some(syscall))
    } else {
        Ok(None)
    }
}

fn get_path_from_syscall(
    child: &ChildProcess,
    syscall_no: u64,
    registers: &mut PtraceRegisters,
) -> Result<Option<PathBuf>> {
    if let Some(register) = SYSCALL_REGISTERS.get(&(syscall_no as i64)) {
        let path_ptr = get_register_from_regs!(register, registers);
        let path = match child.read_string(register, path_ptr as *mut _) {
            Ok(path) => PathBuf::from(path),
            Err(_) => match get_fd_path(child.pid(), path_ptr as i32) {
                Ok(Some(path)) => path,
                Ok(None) => return Ok(None),
                Err(_) => return Ok(None),
            },
        };

        Ok(Some(path))
    } else {
        Ok(None)
    }
}

cfg_if! {
    if #[cfg(target_arch = "x86_64")] {
        mod x86_64;
        pub use x86_64::*;
    } else if #[cfg(target_arch = "riscv64")] {
        mod riscv64;
        pub use riscv64::*;
    } else {
        compile_error!("The current architecture is unsupported!");
    }
}
