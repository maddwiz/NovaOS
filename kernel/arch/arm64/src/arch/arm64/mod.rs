pub mod allocator;
pub mod exceptions;
pub mod mmu;

pub fn architecture_name() -> &'static str {
    "arm64"
}
