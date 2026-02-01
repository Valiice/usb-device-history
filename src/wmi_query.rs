use serde::Deserialize;
use wmi::{COMLibrary, WMIConnection};

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
#[allow(non_camel_case_types)]
pub struct Win32_LogicalDisk {
    pub device_i_d: String,
    pub volume_name: Option<String>,
    pub description: Option<String>,
}

/// Query WMI for currently connected removable drives
pub async fn get_removable_drives() -> Vec<(String, Option<String>, Option<String>)> {
    tokio::task::spawn_blocking(|| get_removable_drives_sync())
        .await
        .unwrap_or_default()
}

fn get_removable_drives_sync() -> Vec<(String, Option<String>, Option<String>)> {
    let mut drives = Vec::new();

    println!("Querying WMI for removable drives...");

    if let Ok(com_con) = COMLibrary::new() {
        if let Ok(wmi_con) = WMIConnection::new(com_con) {
            let query = "SELECT DeviceID, VolumeName, Description FROM Win32_LogicalDisk WHERE DriveType = 2";
            if let Ok(results) = wmi_con.raw_query::<Win32_LogicalDisk>(query) {
                for disk in results {
                    drives.push((
                        disk.device_i_d,
                        disk.volume_name,
                        disk.description,
                    ));
                }
            }
        }
    }

    println!("  Found {} removable drives currently connected", drives.len());
    drives
}
