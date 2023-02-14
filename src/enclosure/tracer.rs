use std::cell::RefCell;
use std::collections::HashMap;

use byteorder::{LittleEndian, WriteBytesExt};
use color_eyre::Result;
use log::*;
use nix::sys::ptrace;
use nix::sys::signal::Signal;
use nix::sys::wait::{waitpid, WaitStatus};
use nix::unistd::Pid;

pub struct Tracer {
    children: HashMap<Pid, ChildProcess>,
}

impl Tracer {
    pub fn new(pid: Pid) -> Self {
        debug!("starting new tracer for root pid {pid}");
        let mut children = HashMap::new();
        let mut root_child = ChildProcess::new(pid, None);
        root_child.state = ChildProcessState::Running;
        children.insert(pid, root_child);
        Self { children }
    }

    pub fn run(&mut self) -> Result<()> {
        debug!("starting to run!");
        while !self.children.is_empty() {
            let mut pids = self.children.keys().cloned().collect::<Vec<_>>();
            pids.sort();
            for pid in pids {
                self.wait_on_child(pid)?;
            }
        }

        Ok(())
    }

    fn wait_on_child(&mut self, pid: Pid) -> Result<()> {
        let status = waitpid(pid, Some(nix::sys::wait::WaitPidFlag::WNOHANG))?;
        match status {
            WaitStatus::Exited(pid, status) => {
                debug!("process {pid} exited with status {status}");
                self.remove_child(pid)?;
            }
            WaitStatus::PtraceEvent(pid, signal, event) => {
                let child = self.children.get_mut(&pid).unwrap();
                child.last_signal = Some(signal);
                match event {
                    libc::PTRACE_EVENT_CLONE
                    | libc::PTRACE_EVENT_FORK
                    | libc::PTRACE_EVENT_VFORK => {
                        let child_pid = ptrace::getevent(pid)?;
                        let child_pid = Pid::from_raw(child_pid as i32);
                        self.children
                            .insert(child_pid, ChildProcess::new(child_pid, Some(pid)));
                        debug!("process {pid} spawned {child_pid}");
                        ptrace::syscall(pid, signal)?;
                    }
                    libc::PTRACE_EVENT_EXEC => {
                        debug!("process {pid} exec'd");
                        ptrace::syscall(pid, signal)?;
                    }
                    libc::PTRACE_EVENT_EXIT => {
                        debug!("process {pid} exited");
                        if let Some(child) = self.children.get(&pid) {
                            if child.parent.is_none() {
                                ptrace::detach(pid, None)?;
                                self.handle_root_exit()?;
                                return Ok(());
                            }
                        }
                        self.remove_child(pid)?;
                    }
                    _ => {}
                }
            }
            WaitStatus::PtraceSyscall(pid) => {
                let child = self.children.get_mut(&pid).unwrap();
                child.last_signal = None;
                match &child.state {
                    ChildProcessState::Running => {
                        debug!("process {pid} entered syscall");
                        child.state = ChildProcessState::EnteringSyscall;
                        self.handle_syscall_enter(pid)?;
                        ptrace::syscall(pid, None)?;
                    }
                    ChildProcessState::EnteringSyscall => {
                        debug!("process {pid} exited syscall");
                        child.state = ChildProcessState::ExitingSyscall;
                        self.handle_syscall_exit(pid)?;
                        ptrace::syscall(pid, None)?;
                    }
                    ChildProcessState::ExitingSyscall => {
                        debug!("process {pid} returned to running");
                        child.state = ChildProcessState::Running;
                        ptrace::syscall(pid, None)?;
                    }
                    _ => {}
                }
            }
            WaitStatus::Signaled(pid, signal, _core_dumped) => {
                debug!("process {pid} signalled with {signal}");
                let child = self.children.get_mut(&pid).unwrap();
                child.last_signal = Some(signal);
                match signal {
                    Signal::SIGTRAP => match child.state {
                        ChildProcessState::Created => {
                            debug!("transition created => running");
                            child.state = ChildProcessState::Running;
                            ptrace::syscall(pid, None)?;
                        }
                        ChildProcessState::Running => {
                            debug!("ptrace event");
                            child.state = ChildProcessState::PtraceEvent;
                            ptrace::syscall(pid, None)?;
                        }
                        _ => {}
                    },
                    Signal::SIGTERM | Signal::SIGKILL => {
                        debug!("process {pid} signalled with {signal}");
                        self.remove_child(pid)?;
                    }
                    _ => {
                        debug!("process {pid} signalled with {signal}");
                        ptrace::syscall(pid, child.last_signal)?;
                    }
                }
            }
            WaitStatus::Stopped(pid, signal) => {
                let child = self.children.get_mut(&pid).unwrap();
                debug!(
                    "{} {pid} stopped with {signal}",
                    if child.parent.is_none() {
                        "root"
                    } else {
                        "child"
                    }
                );
                child.last_signal = None;
                match signal {
                    Signal::SIGTRAP | Signal::SIGSTOP => match child.state {
                        ChildProcessState::Created => {
                            debug!("transition created => running");
                            child.state = ChildProcessState::Running;
                            ptrace::syscall(pid, None)?;
                        }
                        ChildProcessState::Running => {
                            debug!("ptrace event");
                            ptrace::syscall(pid, child.last_signal)?;
                        }
                        _ => {}
                    },
                    _ => {
                        self.remove_child(pid)?;
                        debug!("process {pid} stopped with {signal}");
                    }
                }
            }
            _ => {}
        }

        if let Some(child) = self.children.get_mut(&pid) {
            child.clear_register_cache();
        }

        Ok(())
    }

    fn remove_child(&mut self, pid: Pid) -> Result<()> {
        debug!("! removing child {pid}");
        let child = self.children.remove(&pid);
        ptrace::detach(pid, None)?;

        if let Some(child) = child {
            if child.parent.is_none() {
                self.handle_root_exit()?;
            }
        }

        Ok(())
    }

    fn handle_root_exit(&mut self) -> Result<()> {
        debug!("!!! parent died, stopping all children!");

        let children = self.children.clone();
        let children = children.values();
        debug!("cleaning up {} children!", children.len());
        for child in children {
            ptrace::detach(child.pid, Signal::SIGTERM)?;
            self.children.remove(&child.pid);
            debug!("removed child {}", child.pid);
        }

        Ok(())
    }

    fn handle_syscall_enter(&mut self, pid: Pid) -> Result<()> {
        super::syscall::handle_syscall(self, pid)?;
        Ok(())
    }

    fn handle_syscall_exit(&self, pid: Pid) -> Result<()> {
        let child = self.children.get(&pid).unwrap();
        let regs = child.get_registers()?;
        debug!(
            "child {pid} exited syscall {:?}",
            syscall_numbers::native::sys_call_name(regs.orig_rax as i64)
        );
        Ok(())
    }

    pub fn get_child(&self, pid: Pid) -> Option<&ChildProcess> {
        self.children.get(&pid)
    }
}

pub type PtraceRegisters = libc::user_regs_struct;

#[derive(Debug, Clone)]
pub struct ChildProcess {
    #[allow(unused)]
    pid: Pid,
    state: ChildProcessState,
    last_signal: Option<Signal>,
    parent: Option<Pid>,
    register_cache: RefCell<HashMap<StringRegister, String>>,
}

impl ChildProcess {
    fn new(pid: Pid, parent: Option<Pid>) -> Self {
        Self {
            pid,
            state: ChildProcessState::Created,
            last_signal: None,
            parent,
            register_cache: RefCell::new(HashMap::new()),
        }
    }

    pub fn get_registers(&self) -> Result<PtraceRegisters> {
        ptrace::getregs(self.pid).map_err(|e| e.into())
    }

    pub fn clear_register_cache(&self) {
        self.register_cache.borrow_mut().clear();
    }

    pub fn read_string(&self, register: &StringRegister, addr: *mut u64) -> Result<String> {
        if let Some(cached_str) = self.register_cache.borrow().get(register) {
            return Ok(cached_str.clone());
        }

        let mut buf = vec![];
        let mut addr = addr;
        loop {
            let c = ptrace::read(self.pid, addr as *mut _)?;
            if c == 0 {
                break;
            }
            buf.write_u64::<LittleEndian>(c as u64)?;
            if buf.len() >= libc::PATH_MAX as usize {
                let zero = buf.iter().position(|c| *c == 0);
                if let Some(idx) = zero {
                    buf.truncate(idx);
                }
                break;
            }

            let zero = buf.iter().position(|c| *c == 0);
            if let Some(idx) = zero {
                buf.truncate(idx);
                break;
            }

            // Safety: We're just iterating a C-style string, and exit
            // condition is checked. Unfortunately, we can't know the length of
            // the string ahead of time.
            addr = unsafe { addr.add(1) };
        }

        match String::from_utf8(buf.clone()) {
            Ok(s) => {
                let mut register_cache = self.register_cache.borrow_mut();
                register_cache.insert(*register, s.clone());
                Ok(s)
            }
            err @ Err(_) => err.map_err(|e| e.into()),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ChildProcessState {
    Created,
    Running,
    EnteringSyscall,
    ExitingSyscall,
    PtraceEvent,
}

#[allow(unused)]
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy, PartialOrd, Ord)]
pub enum StringRegister {
    Rdi,
    Rsi,
    Rdx,
    Rcx,
    R8,
    R9,
}
