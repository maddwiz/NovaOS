#![cfg_attr(target_os = "uefi", no_std)]
#![cfg_attr(target_os = "uefi", no_main)]

mod report;

use report::BootObservation;

#[cfg(target_os = "uefi")]
use uefi::prelude::*;
#[cfg(target_os = "uefi")]
use uefi::{boot, println, system};

#[cfg(target_os = "uefi")]
#[entry]
fn efi_main() -> Status {
    let _ = uefi::helpers::init();

    let observation = BootObservation::collect();
    let structured_report_path = observation.persist_report();

    println!("NovaOS Spark observe");
    println!("firmware_vendor={}", system::firmware_vendor());
    println!("firmware_revision={}", observation.firmware_revision);
    println!("config_tables={}", observation.config_table_count);
    println!("memory_map_entries={}", observation.memory_map_entries);
    println!("memory_map_desc_size={}", observation.memory_map_desc_size);
    println!("conventional_pages={}", observation.conventional_pages);
    println!(
        "loaded_image_path_known={}",
        observation.loaded_image_path_known
    );
    println!("display_seed_count={}", observation.display_paths.len());
    println!("storage_seed_count={}", observation.storage_seeds.len());
    println!("network_seed_count={}", observation.network_seeds.len());
    println!(
        "accel_seed_draft_count={}",
        observation.accel_seed_drafts.len()
    );
    println!(
        "storage_filesystem_handles={}",
        observation.storage_filesystem_handles
    );
    println!(
        "storage_block_handles={}",
        observation.storage_block_handles
    );
    println!("network_handles={}", observation.network_handles);
    if let Some(path) = observation.loaded_image_path.as_deref() {
        println!("loaded_image_path={}", path);
    } else {
        println!("loaded_image_path=unknown");
    }
    if let Some(path) = structured_report_path.as_deref() {
        println!("structured_report_path={}", path);
    } else {
        println!("structured_report_path=write_failed");
    }
    println!("secure_boot_known={}", observation.flags.secure_boot_known);
    if let Some(enabled) = observation.secure_boot_enabled {
        println!("secure_boot_enabled={}", enabled);
    } else {
        println!("secure_boot_enabled=unknown");
    }
    if let Some(setup_mode) = observation.setup_mode {
        println!("setup_mode={}", setup_mode);
    } else {
        println!("setup_mode=unknown");
    }
    if let Some(ptr) = observation.table_presence.acpi_rsdp {
        println!("acpi_rsdp={:#x}", ptr);
    } else {
        println!("acpi_rsdp=absent");
    }
    if let Some(ptr) = observation.table_presence.dtb {
        println!("dtb={:#x}", ptr);
    } else {
        println!("dtb=absent");
    }
    if let Some(ptr) = observation.table_presence.smbios {
        println!("smbios={:#x}", ptr);
    } else {
        println!("smbios=absent");
    }
    if let Some(fb) = observation.framebuffer {
        println!(
            "framebuffer base={:#x} {}x{} stride={} format={}",
            fb.base, fb.width, fb.height, fb.stride, fb.pixel_format
        );
    } else {
        println!("framebuffer=absent");
    }
    println!("structured_report_begin");
    for line in observation.structured_report().lines() {
        println!("{}", line);
    }
    println!("structured_report_end");

    boot::stall(3_000_000);
    Status::SUCCESS
}

#[cfg(not(target_os = "uefi"))]
fn main() {
    let observation = BootObservation::collect();
    println!("NovaOS Spark observe");
    println!("{}", observation);
}
