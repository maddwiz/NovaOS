use crate::bootinfo::NovaBootInfoV1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PageTablePlan {
    pub kernel_base: u64,
    pub kernel_size: u64,
    pub user_base: u64,
    pub user_size: u64,
}

impl PageTablePlan {
    pub const fn empty() -> Self {
        Self {
            kernel_base: 0,
            kernel_size: 0,
            user_base: 0,
            user_size: 0,
        }
    }

    pub fn from_boot_info(boot_info: &NovaBootInfoV1) -> Self {
        let memory = boot_info.memory();
        Self {
            kernel_base: memory.kernel_window_base,
            kernel_size: memory.kernel_window_size,
            user_base: memory.user_window_base,
            user_size: memory.user_window_size,
        }
    }
}
