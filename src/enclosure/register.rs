macro_rules! string_registers {
    ($($t:tt)*) => {
        #[allow(unused)]
        #[derive(Debug, Hash, PartialEq, Eq, Clone, Copy, PartialOrd, Ord)]
        pub enum StringRegister {
            $($t)*
        }
    };
}

#[cfg(target_arch = "x86_64")]
macro_rules! syscall_number_from_user_regs {
    ($regs: ident) => {
        $regs.orig_rax
    };
}

#[cfg(target_arch = "riscv64")]
macro_rules! syscall_number_from_user_regs {
    ($regs: ident) => {
        $regs.a7
    };
}

#[cfg(target_arch = "x86_64")]
string_registers! {
    Rdi,
    Rsi,
    Rdx,
    Rcx,
    R8,
    R9,
}

#[cfg(target_arch = "riscv64")]
string_registers! {
    A0,
    A1,
    A2,
    A3,
    A4,
    A5
}

#[cfg(target_arch = "x86_64")]
macro_rules! get_register_from_regs {
    ($string_register: expr, $registers: ident) => {
        match $string_register {
            StringRegister::Rdi => $registers.rdi,
            StringRegister::Rsi => $registers.rsi,
            StringRegister::Rdx => $registers.rdx,
            StringRegister::Rcx => $registers.rcx,
            StringRegister::R8 => $registers.r8,
            StringRegister::R9 => $registers.r9,
        }
    };
}

#[cfg(target_arch = "riscv64")]
macro_rules! get_register_from_regs {
    ($string_register: expr, $registers: ident) => {
        match $string_register {
            StringRegister::A0 => $registers.a0,
            StringRegister::A1 => $registers.a1,
            StringRegister::A2 => $registers.a2,
            StringRegister::A3 => $registers.a3,
            StringRegister::A4 => $registers.a4,
            StringRegister::A5 => $registers.a5,
        }
    };
}

pub(crate) use get_register_from_regs;
pub(crate) use syscall_number_from_user_regs;
