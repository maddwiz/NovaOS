use crate::bootinfo::NovaBootInfoV1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FrameAllocatorPlan {
    pub usable_base: u64,
    pub usable_limit: u64,
    pub reserved_bytes: u64,
}

impl FrameAllocatorPlan {
    pub const fn empty() -> Self {
        Self {
            usable_base: 0,
            usable_limit: 0,
            reserved_bytes: 0,
        }
    }

    pub fn from_boot_info(boot_info: &NovaBootInfoV1) -> Self {
        let memory = boot_info.memory();
        Self {
            usable_base: memory.usable_base,
            usable_limit: memory.usable_limit,
            reserved_bytes: memory.reserved_bytes,
        }
    }
}
