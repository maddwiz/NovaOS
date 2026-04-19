#![no_std]

#[cfg(test)]
extern crate alloc;

pub mod bootinfo;
pub mod handoff;

pub use bootinfo::{NovaBootInfoV1, NovaBootInfoV2};
pub use handoff::{
    build_plan, handoff, prepare_transfer, stage1_entry, KernelEntry, KernelImage, Stage1Config,
    Stage1Entry, Stage1Input, Stage1Plan, Stage1Status, Stage1Transfer, Stage1TransferStatus,
};

pub fn stage1_identity() -> &'static str {
    "NovaOS stage1"
}
