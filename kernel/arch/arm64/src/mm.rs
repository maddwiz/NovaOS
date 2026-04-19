use crate::bootinfo::NovaBootInfoV1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MemoryWindow {
    pub base: u64,
    pub size: u64,
}

impl MemoryWindow {
    pub const fn empty() -> Self {
        Self { base: 0, size: 0 }
    }
}

pub fn kernel_window(boot_info: &NovaBootInfoV1) -> MemoryWindow {
    let memory = boot_info.memory();
    MemoryWindow {
        base: memory.kernel_window_base,
        size: memory.kernel_window_size,
    }
}

pub fn user_window(boot_info: &NovaBootInfoV1) -> MemoryWindow {
    let memory = boot_info.memory();
    MemoryWindow {
        base: memory.user_window_base,
        size: memory.user_window_size,
    }
}
