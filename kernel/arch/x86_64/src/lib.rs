#![no_std]

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X86_64ArchPlaceholder {
    pub supports_uefi: bool,
    pub supports_pci: bool,
}

impl X86_64ArchPlaceholder {
    pub const fn new() -> Self {
        Self {
            supports_uefi: true,
            supports_pci: true,
        }
    }
}

pub const fn arch_name() -> &'static str {
    "x86_64-placeholder"
}

#[cfg(test)]
mod tests {
    use super::{X86_64ArchPlaceholder, arch_name};

    #[test]
    fn placeholder_arch_reports_expected_boot_primitives() {
        let placeholder = X86_64ArchPlaceholder::new();
        assert_eq!(arch_name(), "x86_64-placeholder");
        assert!(placeholder.supports_uefi);
        assert!(placeholder.supports_pci);
    }
}
