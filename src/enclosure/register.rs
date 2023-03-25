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
pub(crate) use syscall_number_from_user_regs;

#[cfg(target_arch = "x86_64")]
string_registers! {
    Rdi,
    Rsi,
    Rdx,
    Rcx,
    R8,
    R9,
}
