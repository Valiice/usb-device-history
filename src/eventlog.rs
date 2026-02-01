use std::collections::HashMap;
use std::process::Command;
use chrono::{DateTime, Utc};

/// Query Windows Event Log for USB device installation timestamps
pub fn get_install_timestamps() -> HashMap<String, DateTime<Utc>> {
    let mut timestamps = HashMap::new();

    println!("Querying Windows Event Logs for installation times...");

    // Try multiple methods to get USB event data

    // Method 1: DriverFrameworks log (requires admin)
    if let Some(ts) = try_driver_frameworks_log() {
        timestamps.extend(ts);
    }

    // Method 2: System log via PowerShell (more accessible)
    if timestamps.is_empty() {
        if let Some(ts) = try_system_log_powershell() {
            timestamps.extend(ts);
        }
    }

    if timestamps.is_empty() {
        println!("  ⚠️  No installation timestamps found.");
        println!("  Tip: Run as Administrator for full event log access.");
    } else {
        println!("  Found {} installation timestamps", timestamps.len());
    }

    timestamps
}

fn try_driver_frameworks_log() -> Option<HashMap<String, DateTime<Utc>>> {
    let output = Command::new("wevtutil")
        .args([
            "qe",
            "Microsoft-Windows-DriverFrameworks-UserMode/Operational",
            "/f:text",
            "/c:100",
            "/rd:true"
        ])
        .output()
        .ok()?;

    if !output.status.success() || output.stdout.is_empty() {
        return None;
    }

    let mut timestamps = HashMap::new();
    let text = String::from_utf8_lossy(&output.stdout);
    let mut current_time: Option<DateTime<Utc>> = None;

    for line in text.lines() {
        let line = line.trim();

        // Try multiple date formats
        if line.starts_with("Date:") || line.contains("TimeCreated") {
            if let Some(date_str) = extract_date(line) {
                if let Ok(dt) = DateTime::parse_from_rfc3339(&date_str) {
                    current_time = Some(dt.with_timezone(&Utc));
                }
            }
        }

        if (line.contains("USB") || line.contains("USBSTOR")) && current_time.is_some() {
            if let Some(device_id) = extract_device_id(line) {
                timestamps.insert(device_id, current_time.unwrap());
                current_time = None;
            }
        }
    }

    if timestamps.is_empty() {
        None
    } else {
        Some(timestamps)
    }
}

fn try_system_log_powershell() -> Option<HashMap<String, DateTime<Utc>>> {
    let script = r#"
        Get-EventLog -LogName System -Newest 200 -ErrorAction SilentlyContinue |
        Where-Object { $_.Message -match 'USB|USBSTOR' } |
        Select-Object -First 10 |
        ForEach-Object {
            "$($_.TimeGenerated.ToString('o'))|$($_.Message.Split([Environment]::NewLine)[0])"
        }
    "#;

    let output = Command::new("powershell")
        .args(["-NoProfile", "-Command", script])
        .output()
        .ok()?;

    if !output.status.success() || output.stdout.is_empty() {
        return None;
    }

    let mut timestamps = HashMap::new();
    let text = String::from_utf8_lossy(&output.stdout);

    for line in text.lines() {
        if let Some((time_str, message)) = line.split_once('|') {
            if let Ok(dt) = DateTime::parse_from_rfc3339(time_str.trim()) {
                if let Some(device_id) = extract_device_id(message) {
                    timestamps.insert(device_id, dt.with_timezone(&Utc));
                }
            }
        }
    }

    if timestamps.is_empty() {
        None
    } else {
        Some(timestamps)
    }
}

fn extract_date(line: &str) -> Option<String> {
    // Try to extract date from various formats
    if let Some(stripped) = line.strip_prefix("Date:") {
        return Some(stripped.trim().to_string());
    }

    // Look for TimeCreated format
    if line.contains("TimeCreated") {
        // Parse XML-style: <TimeCreated SystemTime='2026-01-31T...'/>
        if let Some(start) = line.find("SystemTime='") {
            if let Some(end) = line[start + 12..].find('\'') {
                return Some(line[start + 12..start + 12 + end].to_string());
            }
        }
    }

    None
}

fn extract_device_id(line: &str) -> Option<String> {
    // Try to extract VID/PID
    if let Some(start) = line.find("VID_") {
        if let Some(end) = line[start..].find(|c: char| c.is_whitespace() || c == '\\' || c == '&') {
            return Some(line[start..start + end].to_string());
        }
    }

    // Try to extract device instance ID
    if line.contains("USBSTOR") {
        return Some("USBSTOR_Device".to_string());
    }

    None
}
