#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]

#[cfg(target_os = "none")]
use core::hint::spin_loop;
#[cfg(target_os = "none")]
use core::panic::PanicInfo;
#[cfg(target_os = "none")]
use nova_rt::{NovaBootstrapTaskContextV1, resolve_bootstrap_task_context};

#[cfg(target_os = "none")]
#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    loop {
        spin_loop();
    }
}

#[cfg(target_os = "none")]
#[unsafe(no_mangle)]
pub extern "C" fn _start(context: *const NovaBootstrapTaskContextV1) -> ! {
    let _context_valid = resolve_bootstrap_task_context(context).is_some();
    loop {
        spin_loop();
    }
}

#[cfg(not(target_os = "none"))]
fn main() {
    println!("{}", intentd_payload_identity());
}

pub fn intentd_payload_identity() -> &'static str {
    "NovaOS intentd payload"
}
