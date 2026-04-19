use crate::console::{ConsoleSink, LogLevel};

pub fn log_and_halt<C: ConsoleSink>(console: &mut C, reason: &str) -> ! {
    console.log(LogLevel::Error, "kernel halted");
    console.log(LogLevel::Error, reason);
    halt()
}

pub fn halt() -> ! {
    loop {
        core::hint::spin_loop();
    }
}
