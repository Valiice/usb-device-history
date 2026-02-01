use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use chrono::{DateTime, NaiveDateTime, Utc};

/// Parse setupapi.dev.log for USB device installation timestamps
pub async fn parse_setupapi_log() -> HashMap<String, DateTime<Utc>> {
    tokio::task::spawn_blocking(|| parse_setupapi_log_sync())
        .await
        .unwrap_or_default()
}

fn parse_setupapi_log_sync() -> HashMap<String, DateTime<Utc>> {
    let mut timestamps = HashMap::new();

    println!("Parsing setupapi.dev.log for installation times...");

    let log_path = r"C:\Windows\INF\setupapi.dev.log";

    let file = match File::open(log_path) {
        Ok(f) => f,
        Err(_) => {
            println!("  ⚠️  Could not open setupapi.dev.log");
            return timestamps;
        }
    };

    let reader = BufReader::new(file);
    let mut current_device: Option<String> = None;

    for line in reader.lines().filter_map(Result::ok) {
        let line = line.trim();

        // Look for device install section headers
        // Format: >>>  [Device Install (Hardware initiated) - USB\VID_XXXX&PID_XXXX\...]
        if line.starts_with(">>>") && line.contains("Device Install") {
            if line.contains("USB\\VID_") {
                current_device = extract_device_id(&line);
            } else if line.contains("USBSTOR") {
                current_device = Some("USBSTOR".to_string());
            }
        }

        // Look for timestamp lines
        // Format: >>>  Section start 2026/01/31 23:41:31.538
        if line.contains("Section start") {
            if let Some(timestamp) = parse_timestamp(&line) {
                // If we have a device, store its timestamp
                if let Some(device) = &current_device {
                    timestamps.insert(device.clone(), timestamp);
                    current_device = None; // Reset for next device
                }
            }
        }
    }

    println!("  Found {} installation timestamps from setupapi.dev.log", timestamps.len());
    timestamps
}

fn extract_device_id(line: &str) -> Option<String> {
    // Extract VID_XXXX&PID_XXXX from the line
    if let Some(start) = line.find("VID_") {
        // Find the end (either backslash or closing bracket)
        let after_vid = &line[start..];
        if let Some(end) = after_vid.find(|c| c == '\\' || c == ']' || c == '&' && !after_vid[..20].contains("PID_")) {
            // Get VID_XXXX&PID_XXXX portion
            let device_str = &after_vid[..end];
            if device_str.contains("PID_") {
                // Extract just VID_XXXX&PID_XXXX
                if let Some(pid_end) = device_str.find("&PID_") {
                    if let Some(after_pid) = device_str.get(pid_end + 5..pid_end + 9) {
                        let vid = &device_str[4..8];  // After "VID_"
                        return Some(format!("VID_{}&PID_{}", vid, after_pid));
                    }
                }
            }
        }
    }
    None
}

fn parse_timestamp(line: &str) -> Option<DateTime<Utc>> {
    // Format: ">>>  Section start 2026/01/31 23:41:31.538"
    if let Some(start_idx) = line.find("Section start") {
        let after_start = &line[start_idx + 13..].trim();

        // Parse format: YYYY/MM/DD HH:MM:SS.mmm
        // We'll ignore milliseconds for simplicity
        let parts: Vec<&str> = after_start.split_whitespace().collect();
        if parts.len() >= 2 {
            let date = parts[0];
            let time = parts[1].split('.').next()?;

            let datetime_str = format!("{} {}", date.replace('/', "-"), time);

            if let Ok(naive_dt) = NaiveDateTime::parse_from_str(&datetime_str, "%Y-%m-%d %H:%M:%S") {
                return Some(DateTime::from_naive_utc_and_offset(naive_dt, Utc));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_device_id() {
        let line = ">>>  [Device Install (Hardware initiated) - USB\\VID_0781&PID_5591\\123456]";
        assert_eq!(extract_device_id(line), Some("VID_0781&PID_5591".to_string()));
    }

    #[test]
    fn test_parse_timestamp() {
        let line = ">>>  Section start 2026/01/31 23:41:31.538";
        let result = parse_timestamp(line);
        assert!(result.is_some());
    }
}
