use std::fmt;
use chrono::{DateTime, Utc};
use colored::*;
use winreg::RegKey;
use crate::vendors::{parse_vid_pid, lookup_vendor};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DeviceCategory {
    Storage,
    Input,
    Audio,
    Mobile,
    Hub,
    Other,
}

impl DeviceCategory {
    pub fn as_str(&self) -> &str {
        match self {
            DeviceCategory::Storage => "💾 Storage",
            DeviceCategory::Input => "🎮 Input Device",
            DeviceCategory::Audio => "🎵 Audio",
            DeviceCategory::Mobile => "📱 Mobile Device",
            DeviceCategory::Hub => "🔌 USB Hub",
            DeviceCategory::Other => "🔧 Other",
        }
    }

    pub fn color(&self) -> Color {
        match self {
            DeviceCategory::Storage => Color::Yellow,
            DeviceCategory::Input => Color::Cyan,
            DeviceCategory::Audio => Color::Magenta,
            DeviceCategory::Mobile => Color::Blue,
            DeviceCategory::Hub => Color::BrightBlack,
            DeviceCategory::Other => Color::White,
        }
    }
}

#[derive(Debug, Clone)]
pub struct UsbDevice {
    #[allow(dead_code)]
    category: String,
    pub vendor_product: String,
    pub serial_number: String,
    pub friendly_name: Option<String>,
    pub device_desc: Option<String>,
    #[allow(dead_code)]
    class: Option<String>,
    pub vid: Option<String>,
    pub pid: Option<String>,
    pub vendor_name: Option<String>,
    pub install_time: Option<DateTime<Utc>>,
    pub drive_letter: Option<String>,
    pub device_category: DeviceCategory,
}

impl UsbDevice {
    pub fn from_registry(category: String, device_type: String, serial: String, key: &RegKey) -> Self {
        let (vid, pid) = parse_vid_pid(&device_type);
        let vendor_name = vid.as_ref().and_then(|v| lookup_vendor(v));
        let friendly_name: Option<String> = key.get_value("FriendlyName").ok();
        let device_desc: Option<String> = key.get_value("DeviceDesc").ok();
        let class: Option<String> = key.get_value("Class").ok();

        // Determine device category
        let device_category = Self::categorize(&category, &device_type, &friendly_name, &device_desc, &class);

        Self {
            category,
            vendor_product: device_type,
            serial_number: serial,
            friendly_name,
            device_desc,
            class,
            vid,
            pid,
            vendor_name,
            install_time: None,
            drive_letter: None,
            device_category,
        }
    }

    fn categorize(
        category: &str,
        device_type: &str,
        friendly_name: &Option<String>,
        device_desc: &Option<String>,
        class: &Option<String>,
    ) -> DeviceCategory {
        let full_text = format!(
            "{} {} {} {} {}",
            category,
            device_type,
            friendly_name.as_deref().unwrap_or(""),
            device_desc.as_deref().unwrap_or(""),
            class.as_deref().unwrap_or("")
        ).to_lowercase();

        // Storage devices
        if full_text.contains("storage")
            || full_text.contains("disk")
            || full_text.contains("drive")
            || full_text.contains("usbstor")
            || full_text.contains("mass storage")
        {
            return DeviceCategory::Storage;
        }

        // Audio devices
        if full_text.contains("audio")
            || full_text.contains("sound")
            || full_text.contains("microphone")
            || full_text.contains("headset")
            || full_text.contains("speaker")
            || full_text.contains("shure")
            || full_text.contains("g735")  // Logitech G735 headset
            || full_text.contains("046d:0ad8")  // Logitech G735 VID:PID
        {
            return DeviceCategory::Audio;
        }

        // Input devices
        if full_text.contains("keyboard")
            || full_text.contains("mouse")
            || full_text.contains("controller")
            || full_text.contains("gamepad")
            || full_text.contains("hid")
            || full_text.contains("input")
            || full_text.contains("razer")
            || full_text.contains("steelseries")
            || full_text.contains("logitech")
            || full_text.contains("xbox")
        {
            return DeviceCategory::Input;
        }

        // Mobile devices
        if full_text.contains("mobile")
            || full_text.contains("phone")
            || full_text.contains("iphone")
            || full_text.contains("android")
            || full_text.contains("mtp")
            || full_text.contains("apple")
            || full_text.contains("oneplus")
        {
            return DeviceCategory::Mobile;
        }

        // USB Hubs
        if full_text.contains("hub") {
            return DeviceCategory::Hub;
        }

        DeviceCategory::Other
    }
}

impl fmt::Display for UsbDevice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Device category with icon
        writeln!(f, "  {}", self.device_category.as_str().color(self.device_category.color()).bold())?;

        // VID:PID with vendor
        if let (Some(vid), Some(pid)) = (&self.vid, &self.pid) {
            write!(f, "  {}: ", "VID:PID".bright_black())?;
            write!(f, "{vid}:{pid}")?;
            if let Some(vendor) = &self.vendor_name {
                writeln!(f, " ({})", vendor.cyan())?;
            } else {
                writeln!(f)?;
            }
        }

        // Device name
        let display_name = self.friendly_name.as_ref()
            .or(self.device_desc.as_ref())
            .map(|s| clean_device_name(s));

        if let Some(name) = display_name {
            writeln!(f, "  {}: {}", "Name".bright_black(), name.bright_white().bold())?;
        }

        // Serial number (truncated if too long)
        let serial = if self.serial_number.len() > 60 {
            format!("{}...", &self.serial_number[..60])
        } else {
            self.serial_number.clone()
        };
        writeln!(f, "  {}: {}", "Serial".bright_black(), serial.white())?;

        // Installation time
        if let Some(time) = &self.install_time {
            writeln!(f, "  {}: {}", "Installed".bright_black(),
                time.format("%Y-%m-%d %H:%M:%S").to_string().green())?;
        }

        // Drive letter (for storage devices)
        if let Some(drive) = &self.drive_letter {
            writeln!(f, "  {}: {}", "Drive".bright_black(), drive.yellow().bold())?;
        }

        Ok(())
    }
}

fn clean_device_name(name: &str) -> String {
    if let Some(idx) = name.rfind(';') {
        name[idx + 1..].trim().to_string()
    } else {
        name.to_string()
    }
}
