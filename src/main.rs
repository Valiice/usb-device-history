mod vendors;
mod device;
mod registry;
mod eventlog;
mod mounted;
mod wmi_query;
mod setupapi;

use std::collections::HashMap;
use std::error::Error;
use std::io::{self, Write};
use colored::*;
use device::{UsbDevice, DeviceCategory};

async fn collect_all_devices() -> Vec<UsbDevice> {
    println!("{}", "=== Scanning Registry ===".bright_cyan().bold());
    let mut devices = registry::collect_devices().await;
    println!();

    println!("{}", "=== Gathering Additional Information ===".bright_cyan().bold());

    // Run setupapi, event logs, mounted devices, and WMI queries concurrently
    let (setupapi_times, event_times, _mounted, removable) = tokio::join!(
        setupapi::parse_setupapi_log(),
        eventlog::get_install_timestamps(),
        mounted::get_mounted_devices(),
        wmi_query::get_removable_drives()
    );

    // Show combined timestamp results
    let total_timestamps = setupapi_times.len() + event_times.len();
    if total_timestamps > 0 {
        println!("\n{} {}", "📅".yellow(), format!("Total installation timestamps found: {}", total_timestamps).bright_white());
        println!("   - From setupapi.dev.log: {}", setupapi_times.len().to_string().green());
        println!("   - From event logs: {}", event_times.len().to_string().green());
    }

    // Match timestamps to devices
    let mut matched_count = 0;
    for device in &mut devices {
        if let (Some(vid), Some(pid)) = (&device.vid, &device.pid) {
            let vid_pid_key = format!("VID_{}&PID_{}", vid, pid);

            // Try setupapi first (more accurate)
            if let Some(timestamp) = setupapi_times.get(&vid_pid_key) {
                device.install_time = Some(*timestamp);
                matched_count += 1;
            }
            // Fallback to event logs
            else if let Some(timestamp) = event_times.get(&vid_pid_key) {
                device.install_time = Some(*timestamp);
                matched_count += 1;
            }
        }
    }

    if matched_count > 0 {
        println!("   - Matched timestamps to {} devices", matched_count.to_string().green());
    }
    println!();

    if !removable.is_empty() {
        println!("{}", "=== Currently Connected Removable Drives ===".bright_green().bold());
        for (drive, volume, desc) in removable {
            println!("  {} {} - {} ({})",
                "💾".yellow(),
                drive.yellow().bold(),
                volume.as_deref().unwrap_or("No Label").bright_white(),
                desc.as_deref().unwrap_or("Unknown").white()
            );
        }
        println!();
    }

    devices
}

fn display_devices(devices: &[UsbDevice], verbose: bool) {
    if devices.is_empty() {
        println!("{}", "No USB devices found in registry.".red());
        return;
    }

    // Filter out system devices first (unless verbose mode)
    let (pre_filtered, filtered_devices): (Vec<&UsbDevice>, Vec<&UsbDevice>) = if verbose {
        (devices.iter().collect(), Vec::new())
    } else {
        let filtered: Vec<&UsbDevice> = devices
            .iter()
            .filter(|d| {
                !d.vendor_product.contains("ROOT_HUB")
                    && !d.vendor_product.contains("VID_0000")
                    && !d.vendor_product.contains("&MI_")
                    && !d.vendor_product.contains("&LAMPARRAY")
            })
            .collect();
        let system: Vec<&UsbDevice> = devices
            .iter()
            .filter(|d| {
                d.vendor_product.contains("ROOT_HUB")
                    || d.vendor_product.contains("VID_0000")
                    || d.vendor_product.contains("&MI_")
                    || d.vendor_product.contains("&LAMPARRAY")
            })
            .collect();
        (filtered, system)
    };

    let filtered_count = filtered_devices.len();
    if filtered_count > 0 {
        println!("{} {}",
            "ℹ️".bright_black(),
            format!("Filtered out {} system/composite devices (use --verbose to show all)",
                filtered_count).bright_black());
        println!();
    }

    // Deduplicate by serial number, merging best info from duplicates
    // Note: Windows stores serials differently in USB vs USBSTOR registry,
    // so we need to match by prefix for longer serials
    let mut devices_vec: Vec<UsbDevice> = pre_filtered.iter().map(|d| (*d).clone()).collect();

    // Helper function to check if serials match (by prefix)
    let serials_match = |serial1: &str, serial2: &str| -> bool {
        const MIN_PREFIX_LEN: usize = 20;
        if serial1.len() >= MIN_PREFIX_LEN && serial2.len() >= MIN_PREFIX_LEN {
            serial1.starts_with(serial2) || serial2.starts_with(serial1)
        } else {
            serial1 == serial2
        }
    };

    // Helper to score friendly name quality (higher = better)
    let name_quality = |device: &UsbDevice| -> usize {
        let name = device.friendly_name.as_ref()
            .or(device.device_desc.as_ref())
            .map(|s| s.as_str())
            .unwrap_or("");

        // Prefer names with vendor/product info over generic names
        if name.contains("Mass Storage") || name.contains("USB Device") && name.len() < 20 {
            1 // Generic
        } else {
            name.len() // More specific names tend to be longer
        }
    };

    // Merge duplicates: keep VID/PID from one, but take better name from either
    let mut i = 0;
    while i < devices_vec.len() {
        let mut j = i + 1;
        while j < devices_vec.len() {
            if serials_match(&devices_vec[i].serial_number, &devices_vec[j].serial_number) {
                // Found duplicate - merge the better info
                let (keep_idx, merge_idx) = if devices_vec[i].vid.is_some() {
                    (i, j) // Keep i (has VID/PID), merge from j
                } else {
                    (j, i) // Keep j (has VID/PID), merge from i
                };

                // If the other device has a better name, use it
                if name_quality(&devices_vec[merge_idx]) > name_quality(&devices_vec[keep_idx]) {
                    devices_vec[keep_idx].friendly_name = devices_vec[merge_idx].friendly_name.clone();
                    devices_vec[keep_idx].device_desc = devices_vec[merge_idx].device_desc.clone();
                }

                // Use the longer serial
                if devices_vec[merge_idx].serial_number.len() > devices_vec[keep_idx].serial_number.len() {
                    devices_vec[keep_idx].serial_number = devices_vec[merge_idx].serial_number.clone();
                }

                // Remove the duplicate
                devices_vec.remove(merge_idx);
                continue;
            }
            j += 1;
        }
        i += 1;
    }

    let filtered: Vec<&UsbDevice> = devices_vec.iter().collect();

    if filtered.is_empty() {
        println!("{}", "No user devices found (only system devices detected).".yellow());
        return;
    }

    // Group devices by category
    let mut categorized: HashMap<DeviceCategory, Vec<&UsbDevice>> = HashMap::new();
    for device in &filtered {
        categorized.entry(device.device_category.clone())
            .or_insert_with(Vec::new)
            .push(device);
    }

    println!("{}", "=== Device History (Categorized) ===".bright_cyan().bold());
    println!();

    // Display in category order
    let category_order = [
        DeviceCategory::Storage,
        DeviceCategory::Input,
        DeviceCategory::Audio,
        DeviceCategory::Mobile,
        DeviceCategory::Hub,
        DeviceCategory::Other,
    ];

    let mut device_number = 1;
    for category in &category_order {
        if let Some(category_devices) = categorized.get(category) {
            if !category_devices.is_empty() {
                // Category header
                println!("{} {} {}",
                    "▸".color(category.color()).bold(),
                    category.as_str().color(category.color()).bold(),
                    format!("({} devices)", category_devices.len()).bright_black()
                );
                println!("{}", "─".repeat(60).bright_black());

                // Display devices in this category
                for device in category_devices {
                    println!("{} {}", "Device".bright_black(), format!("#{}", device_number).cyan().bold());
                    print!("{device}");
                    device_number += 1;
                }
                println!();
            }
        }
    }

    println!("{}", "─".repeat(60).bright_black());
    println!("{} {}", "Total user devices found:".bright_white().bold(), filtered.len().to_string().green().bold());
    println!("{} {}", "Total all devices (including system):".white(), devices.len().to_string().bright_black());

    // Show filtered devices in verbose mode
    if verbose && !filtered_devices.is_empty() {
        println!();
        println!("{}", "=== Filtered System/Composite Devices (--verbose) ===".bright_black().bold());
        println!();
        for (idx, device) in filtered_devices.iter().enumerate() {
            println!("{} {}", "Filtered".bright_black(), format!("#{}", idx + 1).bright_black());

            if let (Some(vid), Some(pid)) = (&device.vid, &device.pid) {
                print!("  {}: ", "VID:PID".bright_black());
                print!("{vid}:{pid}");
                if let Some(vendor) = &device.vendor_name {
                    println!(" ({})", vendor.bright_black());
                } else {
                    println!();
                }
            }

            println!("  {}: {}", "Type".bright_black(), device.vendor_product.bright_black());
            println!("  {}: {}", "Serial".bright_black(), device.serial_number.bright_black());
            println!();
        }
    }
}

fn pause() {
    print!("\n{}", "Press Enter to exit...".bright_black());
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Parse command-line arguments
    let args: Vec<String> = std::env::args().collect();
    let verbose = args.iter().any(|arg| arg == "--verbose" || arg == "-v");

    println!("{}", "=== USB Device History Scanner ===".bright_magenta().bold());
    println!();

    let devices = collect_all_devices().await;
    display_devices(&devices, verbose);

    pause();
    Ok(())
}
