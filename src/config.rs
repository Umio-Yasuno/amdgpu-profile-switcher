use libdrm_amdgpu_sys::{AMDGPU, PCI};
use AMDGPU::{PowerProfile, DpmForcedLevel};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct ParsedConfig {
    pub config_devices: Vec<ParsedConfigPerDevice>,
}

#[derive(Debug, Clone)]
pub struct ParsedConfigPerDevice {
    pub pci: PCI::BUS_INFO,
    pub entries: Vec<ParsedConfigEntry>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ParsedConfigEntry {
    pub name: String,
    pub perf_level: Option<DpmForcedLevel>,
    pub profile: Option<PowerProfile>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub config_devices: Vec<ConfigPerDevice>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConfigPerDevice {
    pub pci: String,
    pub entries: Vec<ConfigEntry>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConfigEntry {
    pub name: String,
    pub perf_level: Option<String>,
    pub profile: Option<String>,
}

impl Config {
    pub fn parse(&self) -> ParsedConfig {
        if self.config_devices.is_empty() {
            panic!("`config_devices` is empty.");
        }

        let config_devices = self.config_devices.iter().map(|device| device.parse()).collect();

        ParsedConfig { config_devices }
    }
}

impl ConfigPerDevice {
    fn parse(&self) -> ParsedConfigPerDevice {
        let pci: PCI::BUS_INFO = self.pci.parse().unwrap_or_else(|_| panic!("Parse Error: {:?}", self.pci));

        if self.entries.is_empty() {
            panic!("`entries` for {pci} is empty.");
        }

        let entries = self.entries.iter().map(|entry| entry.parse(&pci)).collect();

        ParsedConfigPerDevice { pci, entries }
    }
}

impl ConfigEntry {
    fn parse(&self, pci: &PCI::BUS_INFO) -> ParsedConfigEntry {
        if self.name.is_empty() {
            panic!("`name` for {pci} is empty.")
        }

        let name = self.name.clone();
        let perf_level = self.perf_level.as_ref().and_then(|s| {
            let perf_level = perf_level_from_str(s);

            if perf_level.is_none() {
                panic!("`perf_level` for {pci} ({:?}) is invalid.", self);
            }

            perf_level
        });
        let profile = self.profile.as_ref().and_then(|s| {
            let profile = power_profile_from_str(s);

            if profile.is_none() {
                panic!("`profile` for {pci} ({:?}) is invalid.", self);
            }

            profile
        });

        ParsedConfigEntry { name, perf_level, profile }
    }
}

fn perf_level_from_str(s: &str) -> Option<DpmForcedLevel> {
    let perf_level = match s {
        "auto" => DpmForcedLevel::Auto,
        "low" => DpmForcedLevel::Low,
        "high" => DpmForcedLevel::High,
        "manual" => DpmForcedLevel::Manual,
        "profile_standard" => DpmForcedLevel::ProfileStandard,
        "profile_peak" => DpmForcedLevel::ProfilePeak,
        "profile_min_sclk" => DpmForcedLevel::ProfileMinSclk,
        "profile_min_mclk" => DpmForcedLevel::ProfileMinMclk,
        "profile_exit" => DpmForcedLevel::ProfileExit,
        "perf_determinism" => DpmForcedLevel::PerfDeterminism,
        _ => return None,
    };

    Some(perf_level)
}

fn power_profile_from_str(s: &str) -> Option<PowerProfile> {
    let pp = match s {
        "BOOTUP_DEFAULT" => PowerProfile::BOOTUP_DEFAULT,
        "3D_FULL_SCREEN" => PowerProfile::FULLSCREEN3D,
        "POWER_SAVING" => PowerProfile::POWERSAVING,
        "VIDEO" => PowerProfile::VIDEO,
        "VR" => PowerProfile::VR,
        "COMPUTE" => PowerProfile::COMPUTE,
        "CUSTOM" => PowerProfile::CUSTOM,
        "WINDOW_3D" => PowerProfile::WINDOW3D,
        "CAPPED" => PowerProfile::CAPPED,
        "UNCAPPED" => PowerProfile::UNCAPPED,
        _ => return None,
    };

    Some(pp)
}
