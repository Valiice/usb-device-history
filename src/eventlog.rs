use std::collections::HashMap;
use chrono::{DateTime, Utc, NaiveDateTime};
use windows::Win32::System::EventLog::*;
use windows::core::PWSTR;

/// Query Windows Event Logs natively for USB device installation timestamps
/// No PowerShell or wevtutil spawning - uses Windows API directly
pub async fn get_install_timestamps() -> HashMap<String, DateTime<Utc>> {
    let mut timestamps = HashMap::new();

    println!("Querying Windows Event Logs for installation times...");

    // Try to get timestamps from DriverFrameworks log (more reliable)
    if let Ok(driver_times) = query_driver_frameworks_log().await {
        timestamps.extend(driver_times);
    }

    // Also try System log as fallback
    if let Ok(system_times) = query_system_log().await {
        for (key, time) in system_times {
            timestamps.entry(key).or_insert(time);
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

/// Query the DriverFrameworks-UserMode operational log
async fn query_driver_frameworks_log() -> Result<HashMap<String, DateTime<Utc>>, String> {
    tokio::task::spawn_blocking(|| {
        query_driver_frameworks_log_sync()
    })
    .await
    .map_err(|e| e.to_string())?
}

fn query_driver_frameworks_log_sync() -> Result<HashMap<String, DateTime<Utc>>, String> {
    let mut timestamps = HashMap::new();

    let channel = "Microsoft-Windows-DriverFrameworks-UserMode/Operational";

    // Open event log channel
    let channel_wide: Vec<u16> = channel.encode_utf16().chain(std::iter::once(0)).collect();
    let h_log = unsafe {
        EvtOpenLog(
            None,
            PWSTR(channel_wide.as_ptr() as *mut u16),
            0x1, // EvtOpenChannelPath
        )
    }
    .map_err(|e| format!("Failed to open DriverFrameworks log: {}", e))?;

    // Query for recent events
    let query = "*[System/EventID=2003]"; // Device installation events
    let query_wide: Vec<u16> = query.encode_utf16().chain(std::iter::once(0)).collect();

    let h_results = unsafe {
        EvtQuery(
            None,
            PWSTR(channel_wide.as_ptr() as *mut u16),
            PWSTR(query_wide.as_ptr() as *mut u16),
            0x200, // EvtQueryReverseDirection (newest first)
        )
    }
    .map_err(|e| {
        unsafe { let _ = EvtClose(h_log); }
        format!("Failed to query events: {}", e)
    })?;

    // Read events
    let mut event_handles: Vec<isize> = vec![0; 100];
    let mut returned = 0u32;

    unsafe {
        if EvtNext(
            h_results,
            event_handles.as_mut_slice(),
            3000, // 3 second timeout
            0,
            &mut returned,
        ).is_ok() {
            for i in 0..returned as usize {
                let h_event = EVT_HANDLE(event_handles[i]);
                if let Some((vid_pid, time)) = parse_event(&h_event) {
                    timestamps.insert(vid_pid, time);
                }
                let _ = EvtClose(h_event);
            }
        }

        let _ = EvtClose(h_results);
        let _ = EvtClose(h_log);
    }

    Ok(timestamps)
}

/// Query the System log for USB events
async fn query_system_log() -> Result<HashMap<String, DateTime<Utc>>, String> {
    tokio::task::spawn_blocking(|| {
        query_system_log_sync()
    })
    .await
    .map_err(|e| e.to_string())?
}

fn query_system_log_sync() -> Result<HashMap<String, DateTime<Utc>>, String> {
    let mut timestamps = HashMap::new();

    let channel = "System";

    // Open event log channel
    let channel_wide: Vec<u16> = channel.encode_utf16().chain(std::iter::once(0)).collect();
    let h_log = unsafe {
        EvtOpenLog(
            None,
            PWSTR(channel_wide.as_ptr() as *mut u16),
            0x1,
        )
    }
    .map_err(|e| format!("Failed to open System log: {}", e))?;

    // Query for USB-related events
    let query = "*[System/Provider[@Name='Microsoft-Windows-Kernel-PnP'] and System/EventID=400]";
    let query_wide: Vec<u16> = query.encode_utf16().chain(std::iter::once(0)).collect();

    let h_results = unsafe {
        EvtQuery(
            None,
            PWSTR(channel_wide.as_ptr() as *mut u16),
            PWSTR(query_wide.as_ptr() as *mut u16),
            0x200,
        )
    }
    .map_err(|e| {
        unsafe { let _ = EvtClose(h_log); }
        format!("Failed to query System events: {}", e)
    })?;

    // Read events
    let mut event_handles: Vec<isize> = vec![0; 200];
    let mut returned = 0u32;

    unsafe {
        if EvtNext(
            h_results,
            event_handles.as_mut_slice(),
            3000,
            0,
            &mut returned,
        ).is_ok() {
            for i in 0..returned as usize {
                let h_event = EVT_HANDLE(event_handles[i]);
                if let Some((vid_pid, time)) = parse_event(&h_event) {
                    timestamps.entry(vid_pid).or_insert(time);
                }
                let _ = EvtClose(h_event);
            }
        }

        let _ = EvtClose(h_results);
        let _ = EvtClose(h_log);
    }

    Ok(timestamps)
}

/// Parse an event to extract VID/PID and timestamp
fn parse_event(h_event: &EVT_HANDLE) -> Option<(String, DateTime<Utc>)> {
    unsafe {
        // Get event as XML
        let mut buffer = vec![0u8; 8192];
        let mut used = 0u32;
        let mut property_count = 0u32;

        if EvtRender(
            None,
            *h_event,
            1, // EvtRenderEventXml
            buffer.len() as u32,
            Some(buffer.as_mut_ptr() as *mut _),
            &mut used,
            &mut property_count,
        ).is_err() {
            return None;
        }

        // Convert to string (UTF-16)
        let xml_slice = std::slice::from_raw_parts(
            buffer.as_ptr() as *const u16,
            (used as usize) / 2,
        );
        let xml = String::from_utf16_lossy(xml_slice);

        // Extract VID/PID from XML
        let vid_pid = extract_vid_pid_from_xml(&xml)?;

        // Extract timestamp
        let timestamp = extract_timestamp_from_xml(&xml)?;

        Some((vid_pid, timestamp))
    }
}

/// Extract VID_XXXX&PID_XXXX from event XML
fn extract_vid_pid_from_xml(xml: &str) -> Option<String> {
    // Look for VID_ pattern
    if let Some(start) = xml.find("VID_") {
        let after_vid = &xml[start..];

        // Find PID_ after VID_
        if let Some(pid_pos) = after_vid.find("PID_") {
            if after_vid.len() >= 8 {
                let vid = &after_vid[4..8]; // 4 chars after "VID_"

                if after_vid.len() >= pid_pos + 8 {
                    let after_pid = &after_vid[pid_pos..];
                    let pid = &after_pid[4..8]; // 4 chars after "PID_"
                    return Some(format!("VID_{}&PID_{}", vid, pid));
                }
            }
        }
    }

    // Also check for USBSTOR references
    if xml.contains("USBSTOR") {
        return Some("USBSTOR".to_string());
    }

    None
}

/// Extract timestamp from event XML
fn extract_timestamp_from_xml(xml: &str) -> Option<DateTime<Utc>> {
    // Look for SystemTime attribute
    if let Some(start) = xml.find("SystemTime='") {
        let after = &xml[start + 12..];
        if let Some(end) = after.find('\'') {
            let time_str = &after[..end];

            // Parse ISO 8601 format: 2026-01-31T23:41:31.538Z
            if let Ok(dt) = DateTime::parse_from_rfc3339(time_str) {
                return Some(dt.with_timezone(&Utc));
            }

            // Try without Z
            let time_with_z = format!("{}Z", time_str.trim_end_matches('Z'));
            if let Ok(dt) = DateTime::parse_from_rfc3339(&time_with_z) {
                return Some(dt.with_timezone(&Utc));
            }

            // Try parsing as naive datetime then convert to UTC
            if let Ok(naive) = NaiveDateTime::parse_from_str(
                &time_str.replace('T', " ").split('.').next().unwrap_or(""),
                "%Y-%m-%d %H:%M:%S"
            ) {
                return Some(DateTime::from_naive_utc_and_offset(naive, Utc));
            }
        }
    }

    None
}
