use std::collections::HashMap;
use winreg::enums::HKEY_LOCAL_MACHINE;
use winreg::RegKey;

/// Query MountedDevices from registry
pub async fn get_mounted_devices() -> HashMap<String, String> {
    tokio::task::spawn_blocking(|| get_mounted_devices_sync())
        .await
        .unwrap_or_default()
}

fn get_mounted_devices_sync() -> HashMap<String, String> {
    let mut mounted = HashMap::new();

    println!("Querying MountedDevices...");

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    if let Ok(key) = hklm.open_subkey(r"SYSTEM\MountedDevices") {
        for value_name in key.enum_values().filter_map(Result::ok) {
            let name = value_name.0;
            if name.starts_with("\\DosDevices\\") {
                if let Some(drive_letter) = name.strip_prefix("\\DosDevices\\") {
                    if let Ok(data) = key.get_raw_value(&name) {
                        let device_path = String::from_utf8_lossy(&data.bytes).to_string();
                        if device_path.contains("USBSTOR") || device_path.contains("USB") {
                            mounted.insert(device_path.clone(), drive_letter.to_string());
                        }
                    }
                }
            }
        }
    }

    println!("  Found {} mounted USB devices", mounted.len());
    mounted
}
