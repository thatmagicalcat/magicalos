#[macro_export]
macro_rules! exception_handler {
    [ $name:ident ] => {{
        #[unsafe(naked)]
        extern "C" fn wrapper() -> ! {
            unsafe {
                naked_asm! {
                    "mov rdi, rsp",
                    "sub rsp, 8", // 16 bytes alignment
                    "call {handler}",

                    handler = sym $name,
                }
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
            unsafe {
                naked_asm! {
                    "pop rsi", // error code
                    "mov rdi, rsp", // stack frame ptr
                    "sub rsp, 8", // 16 bytes alignment
                    "call {handler}",

                    handler = sym $name,
                }
            }
        }
        wrapper
    }};
}
