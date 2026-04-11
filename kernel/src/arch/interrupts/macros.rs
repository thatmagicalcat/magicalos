#[macro_export]
macro_rules! push_scratch {
    () => {
        "// Push scratch registers
        push rax
        push rcx
        push rdx
        push rdi
        push rsi
        push r8
        push r9
        push r10
        push r11
    "
    };
}

#[macro_export]
macro_rules! pop_scratch {
    () => {
        "// Pop scratch registers
        pop r11
        pop r10
        pop r9
        pop r8
        pop rsi
        pop rdi
        pop rdx
        pop rcx
        pop rax
    "
    };
}

#[macro_export]
macro_rules! push_preserved {
    () => {
        "// Push preserved registers
        push rbx
        push rbp
        push r12
        push r13
        push r14
        push r15
    "
    };
}

#[macro_export]
macro_rules! pop_preserved {
    () => {
        "// Pop preserved registers
        pop r15
        pop r14
        pop r13
        pop r12
        pop rbp
        pop rbx
    "
    };
}

// CPU pushes 5 regisers (rip, cs, rflags, rsp, ss) = 40 bytes on exception.
// We push 9 more = 72 bytes
// total 112 bytes which is divisible by 16 so no algnment needed

#[macro_export]
macro_rules! exception_handler {
    [ $name:ident ] => {{
        #[unsafe(naked)]
        extern "C" fn wrapper() -> ! {
            naked_asm! {
                $crate::push_scratch!(),

                "lea rdi, [rsp + 72]", // 9 registers pushed
                "call {handler}",

                $crate::pop_scratch!(),
                "iretq",

                handler = sym $name,
            }
        }

        wrapper
    }}
}

#[macro_export]
macro_rules! exception_handler_with_error_code {
    ($name: ident) => {{
        #[unsafe(naked)]
        extern "C" fn wrapper() -> ! {
            naked_asm! {
                "pop rsi",      // error code

                $crate::push_scratch!(),

                "lea rdi, [rsp + 72]", // 9 registers pushed
                "call {handler}",

                $crate::pop_scratch!(),

                "iretq",

                handler = sym $name,
            }
        }
        wrapper
    }};
}
