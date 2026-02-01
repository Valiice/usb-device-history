# USB Device History Scanner

A fast Rust application that scans and displays detailed history of all USB devices ever connected to your Windows PC.

## Features

- **Fast Concurrent Scanning** - Async I/O with parallel data gathering
- **Native Windows APIs** - Direct Event Log API calls (no PowerShell spawning)
- **Complete Device History** - Registry, setupapi.dev.log, Event Logs, WMI, and MountedDevices
- **Installation Timestamps** - Tracks when devices were first connected
- **Smart Categorization** - Auto-categorizes into Storage, Input, Audio, Mobile, Hub, Other
- **Vendor Database** - Built-in database of 580+ USB manufacturers
- **Color-Coded Output** - Easy-to-read terminal display with category icons
- **Deduplication** - Removes duplicate entries across registry paths

## Requirements

- **OS**: Windows (uses Windows-specific APIs)
- **Rust**: 1.70.0 or later
- **Permissions**: Run as Administrator for full access to event logs and registry

## Installation

```bash
cargo build --release
```

## Usage

```bash
# Run the scanner
.\target\release\usb-device-history.exe

# Or with Cargo
cargo run --release

# Show all devices including system interfaces
cargo run --release -- --verbose
```

### Command-Line Options

- `--verbose` / `-v` - Show all devices including system/composite interfaces (ROOT_HUB, &MI_ interfaces, etc.)

## Example Output

```
=== USB Device History Scanner ===

=== Device History (Categorized) ===

▸ 💾 Storage (2 devices)
────────────────────────────────────────────────────────────
Device #1
  💾 Storage
  VID:PID: 0781:5591 (SanDisk)
  Name: USB Mass Storage Device
  Serial: 0401f51759fee3a2a5379070fa6f14887b43cd053cccf99e35e4ba0ef498...
  Installed: 2026-01-31 23:41:31

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
```

## Performance

All data sources (registry, setupapi.dev.log, Event Logs, WMI, MountedDevices) are queried concurrently using Tokio async runtime. Native Windows Event Log API eliminates process spawning overhead for faster, more reliable results.

## Notes

- Run as Administrator for complete event log and registry access
- Some devices may not have timestamps if logs have been rotated
- System devices are filtered by default (use `--verbose` to show all)

## License

MIT License
