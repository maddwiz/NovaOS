#![no_std]

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PciRootSeed {
    pub segment_group: u16,
    pub bus_start: u8,
    pub bus_end: u8,
}

impl PciRootSeed {
    pub const fn new(segment_group: u16, bus_start: u8, bus_end: u8) -> Self {
        Self {
            segment_group,
            bus_start,
            bus_end,
        }
    }
}

pub const fn discovery_model() -> &'static str {
    "pci-root-seed-placeholder"
}

#[cfg(test)]
mod tests {
    use super::{PciRootSeed, discovery_model};

    #[test]
    fn pci_root_seed_placeholder_is_stable() {
        let seed = PciRootSeed::new(0, 0, 255);
        assert_eq!(discovery_model(), "pci-root-seed-placeholder");
        assert_eq!(seed.segment_group, 0);
        assert_eq!(seed.bus_end, 255);
    }
}
