use winreg::enums::HKEY_LOCAL_MACHINE;
use winreg::RegKey;
use crate::device::UsbDevice;

const REGISTRY_PATHS: &[(&str, &str)] = &[
    ("USB Storage", r"SYSTEM\CurrentControlSet\Enum\USBSTOR"),
    ("USB Devices", r"SYSTEM\CurrentControlSet\Enum\USB"),
];

/// Collect USB devices from a specific registry path
fn collect_devices_from_path(category: &str, registry_path: &str) -> Vec<UsbDevice> {
    let mut devices = Vec::new();

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let Ok(root_key) = hklm.open_subkey(registry_path) else {
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
                category.to_string(),
                device_type.clone(),
                serial,
                &serial_key,
            ));
        }
    }

    devices
}

/// Collect all USB devices from registry
pub fn collect_devices() -> Vec<UsbDevice> {
    let mut all_devices = Vec::new();

    for (category, path) in REGISTRY_PATHS {
        println!("Scanning {}...", category);
        let devices = collect_devices_from_path(category, path);
        println!("  Found {} devices", devices.len());
        all_devices.extend(devices);
    }

    all_devices
}
