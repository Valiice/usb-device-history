# USB Device History Scanner

A comprehensive Rust application that scans and displays detailed history of all USB devices ever connected to your Windows PC.

## Features

### Core Functionality
- **Concurrent Scanning**: Async/await architecture with parallel I/O operations for maximum performance
- **Native Windows APIs**: Direct Windows Event Log API calls (no PowerShell or wevtutil spawning)
- **Registry Scanning**: Queries both `USBSTOR` and `USB` registry paths for comprehensive device discovery
- **Installation Timestamps**: Parses `setupapi.dev.log` and Windows Event Logs to find when devices were first installed
- **Device Categorization**: Automatically categorizes devices into Storage, Input, Audio, Mobile, Hub, and Other
- **Color-Coded Output**: Terminal-based color-coded display with category icons
- **Smart Deduplication**: Removes duplicate entries (same device in multiple registry locations)
- **Vendor Database**: Built-in database of 580+ USB manufacturers (VID lookup)

### Data Sources (All queried concurrently)
1. **Windows Registry** - `HKLM\SYSTEM\CurrentControlSet\Enum\{USBSTOR,USB}` (async parallel scanning)
2. **setupapi.dev.log** - `C:\Windows\INF\setupapi.dev.log` for persistent installation timestamps
3. **Windows Event Logs** - Native Event Log API (`EvtOpenLog`, `EvtQuery`, `EvtNext`) for System and DriverFrameworks logs
4. **MountedDevices** - Drive letter mappings
5. **WMI Queries** - Currently connected removable drives

All data sources are queried in parallel using Tokio async runtime for optimal performance.

### Device Information Displayed
- VID:PID (Vendor ID : Product ID)
- Manufacturer name
- Device name and description
- Serial number
- Installation timestamp (when available)
- Drive letter (for storage devices)
- Device category with color-coded icons

## Performance

The application is optimized for speed using async/concurrent operations:
- **Parallel Registry Scanning** - Multiple registry paths scanned concurrently
- **Concurrent Data Gathering** - All data sources (registry, setupapi, event logs, WMI, mounted devices) queried in parallel
- **Thread Pool for I/O** - Blocking operations offloaded to Tokio thread pool
- **Native APIs** - Direct Windows Event Log API calls eliminate process spawning overhead

All I/O-bound operations run concurrently, making the tool significantly faster than sequential implementations.

## Requirements

- **OS**: Windows (uses Windows-specific APIs)
- **Rust**: 1.70.0 or later
- **Permissions**: Administrator privileges recommended for:
  - Full registry access
  - Reading setupapi.dev.log
  - Querying event logs

## Building

```bash
cargo build --release
```

## Running

```bash
.\target\release\usb-device-history.exe
```

Or with Cargo:

```bash
cargo run --release
```

### Command-Line Options

- `--verbose` or `-v`: Show all devices including system/composite interfaces (ROOT_HUB, &MI_ interfaces, etc.)

  By default, the program filters out system devices and composite interfaces to show only physical devices you've plugged in. Use verbose mode to see everything Windows has registered.

## Example Output

```
=== USB Device History Scanner ===

=== Scanning Registry ===
Scanning USB Storage...
  Found 2 devices
Scanning USB Devices...
  Found 80 devices

=== Gathering Additional Information ===
Parsing setupapi.dev.log for installation times...
  Found 3 installation timestamps from setupapi.dev.log
Querying Windows Event Logs for installation times...
  Found 1 installation timestamps

📅 Total installation timestamps found: 4
   - From setupapi.dev.log: 3
   - From event logs: 1
   - Matched timestamps to 11 devices

=== Device History (Categorized) ===

▸ 💾 Storage (2 devices)
────────────────────────────────────────────────────────────
Device #1
  💾 Storage
  VID:PID: 0781:5591 (SanDisk)
  Name: USB Mass Storage Device
  Serial: 0401f51759fee3a2a5379070fa6f14887b43cd053cccf99e35e4ba0ef498...
  Installed: 2026-01-31 23:41:31

Device #2
  💾 Storage
  Name: OnePlus Device Driver USB Device
  Serial: a&2e2cd3ff&0&4079c817&0

▸ 🎮 Input Device (5 devices)
────────────────────────────────────────────────────────────
Device #3
  🎮 Input Device
  VID:PID: 045E:028E (Microsoft)
  Name: Xbox 360 Controller for Windows
  Serial: 01

[...]

────────────────────────────────────────────────────────────
Total user devices found: 24
Total all devices (including system): 82
```

## Device Categories

- 💾 **Storage** - USB drives, external hard drives, card readers
- 🎮 **Input Device** - Keyboards, mice, game controllers
- 🎵 **Audio** - Headsets, microphones, sound devices
- 📱 **Mobile Device** - Phones, tablets
- 🔌 **USB Hub** - USB hubs and docking stations
- 🔧 **Other** - Everything else

## Architecture

The project follows Separation of Concerns (SoC) with async modular architecture:

- `main.rs` - Async orchestration and display logic (Tokio runtime)
- `device.rs` - Device structure and categorization
- `vendors.rs` - USB vendor database (580+ manufacturers, VID to name mapping)
- `registry.rs` - Async Windows registry queries with parallel path scanning
- `setupapi.rs` - Async setupapi.dev.log parser
- `eventlog.rs` - Native Windows Event Log API queries (EvtOpenLog, EvtQuery, EvtNext, EvtRender)
- `mounted.rs` - Async MountedDevices registry queries
- `wmi_query.rs` - Async WMI queries for connected drives

All modules use `tokio::spawn_blocking` for I/O operations and are orchestrated concurrently in `main.rs` using `tokio::join!`.

## Dependencies

- `tokio` - Async runtime with multi-threading support
- `futures` - Async utilities for concurrent operations
- `windows` - Native Windows API bindings (Event Log API)
- `winreg` - Windows registry access
- `wmi` - WMI queries
- `serde` - Serialization/deserialization
- `chrono` - Date/time handling
- `colored` - Terminal color output

## Notes

- Some devices may not have installation timestamps if the logs have been rotated or cleared
- The program filters out system devices (ROOT_HUB, composite interfaces, etc.)
- Devices are deduplicated using prefix-matching on serial numbers (Windows stores serials differently in USBSTOR vs USB paths)
- For best results, run as Administrator

## License

This project is open source.
