use winreg::enums::HKEY_LOCAL_MACHINE;
use winreg::RegKey;
use crate::device::UsbDevice;
use futures::future::join_all;

const REGISTRY_PATHS: &[(&str, &str)] = &[
    ("USB Storage", r"SYSTEM\CurrentControlSet\Enum\USBSTOR"),
    ("USB Devices", r"SYSTEM\CurrentControlSet\Enum\USB"),
];

/// Collect USB devices from a specific registry path
async fn collect_devices_from_path(category: &str, registry_path: &str) -> Vec<UsbDevice> {
    let category = category.to_string();
    let registry_path = registry_path.to_string();

    // Run blocking registry operations in a separate thread
    tokio::task::spawn_blocking(move || {
        let mut devices = Vec::new();

        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        let Ok(root_key) = hklm.open_subkey(&registry_path) else {
            return devices;
        };

        for device_type in root_key.enum_keys().filter_map(Result::ok) {
            let Ok(device_type_key) = root_key.open_subkey(&device_type) else {
                continue;
            };

            for serial in device_type_key.enum_keys().filter_map(Result::ok) {
                let Ok(serial_key) = device_type_key.open_subkey(&serial) else {
                    continue;
                };

                devices.push(UsbDevice::from_registry(
                    category.clone(),
                    device_type.clone(),
                    serial,
                    &serial_key,
                ));
            }
        }

        devices
    })
    .await
    .unwrap_or_default()
}

/// Collect all USB devices from registry concurrently
pub async fn collect_devices() -> Vec<UsbDevice> {
    // Create tasks for each registry path
    let futures: Vec<_> = REGISTRY_PATHS
        .iter()
        .map(|(category, path)| {
            let category = *category;
            let path = *path;
            async move {
                println!("Scanning {}...", category);
                let devices = collect_devices_from_path(category, path).await;
                println!("  Found {} devices", devices.len());
                devices
            }
        })
        .collect();

    // Execute all queries concurrently
    let results = join_all(futures).await;

    // Flatten results into single vec
    results.into_iter().flatten().collect()
}
