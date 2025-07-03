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
    pub _device_name: Option<String>,
    pub default_power_cap_watt: Option<u32>,
    pub _min_power_cap_watt: Option<u32>,
    pub _max_power_cap_watt: Option<u32>,
    pub default_perf_level: DpmForcedLevel,
    pub default_profile: PowerProfile,
    pub entries: Vec<ParsedConfigEntry>,
}

impl ParsedConfigPerDevice {
    pub fn names(&self) -> Vec<String> {
        self.entries.iter().map(|e| e.name.clone()).collect()
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ParsedConfigEntry {
    pub name: String,
    pub perf_level: Option<DpmForcedLevel>,
    pub profile: Option<PowerProfile>,
    pub power_cap_watt: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub config_devices: Vec<ConfigPerDevice>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConfigPerDevice {
    pub pci: String,
    pub _device_name: Option<String>,
    pub default_power_cap_watt: Option<u32>,
    pub _min_power_cap_watt: Option<u32>,
    pub _max_power_cap_watt: Option<u32>,
    pub default_perf_level: Option<String>,
    pub default_profile: Option<String>,
    pub entries: Vec<ConfigEntry>,
}

#[derive(Default, Debug, Clone, Deserialize, Serialize)]
pub struct ConfigEntry {
    pub name: String,
    pub perf_level: Option<String>,
    pub profile: Option<String>,
    pub power_cap_watt: Option<u32>,
}

#[derive(Debug, Clone)]
pub enum ParseConfigError {
    DevicesIsEmpty,
    InvalidPci(String),
    PciIsEmpty,
    EntryNameIsEmpty,
    InvalidPerfLevel(String),
    InvalidProfile(String),
}

impl Config {
    pub fn parse(&self) -> Result<ParsedConfig, ParseConfigError> {
        if self.config_devices.is_empty() {
            return Err(ParseConfigError::DevicesIsEmpty);
        }

        let config_devices: Result<Vec<_>, _> = self.config_devices
            .iter()
            .map(|device| device.parse())
            .collect();

        Ok(ParsedConfig { config_devices: config_devices? })
    }
}

impl ConfigPerDevice {
    fn parse_default_perf_level(&self) -> Result<DpmForcedLevel, ParseConfigError> {
        let default_perf_level = if let Some(ref s) = self.default_perf_level {
            if let Some(perf_level) = perf_level_from_str(s) {
                perf_level
            } else {
                return Err(ParseConfigError::InvalidPerfLevel(s.to_string()));
            }
        } else {
            DpmForcedLevel::Auto
        };

        Ok(default_perf_level)
    }

    fn parse_default_power_profile(&self) -> Result<PowerProfile, ParseConfigError> {
        let default_profile = if let Some(ref s) = self.default_profile {
            if let Some(profile) = power_profile_from_str(s) {
                profile
            } else {
                return Err(ParseConfigError::InvalidProfile(s.to_string()));
            }
        } else {
            PowerProfile::BOOTUP_DEFAULT
        };

        Ok(default_profile)
    }

    fn parse(&self) -> Result<ParsedConfigPerDevice, ParseConfigError> {
        if self.pci.is_empty() {
            return Err(ParseConfigError::PciIsEmpty);
        }

        let pci: PCI::BUS_INFO = self.pci.parse().map_err(|_| ParseConfigError::InvalidPci(self.pci.to_string()))?;

        if self.entries.is_empty() {
            eprintln!("`entries` for {pci} is empty.");
        }

        let default_perf_level = self.parse_default_perf_level()?;
        let default_profile = self.parse_default_power_profile()?;
        let entries: Result<Vec<ParsedConfigEntry>, ParseConfigError> = self.entries.iter().map(|entry| entry.parse()).collect();

        Ok(ParsedConfigPerDevice {
            pci,
            _device_name: None,
            default_power_cap_watt: None,
            _min_power_cap_watt: None,
            _max_power_cap_watt: None,
            default_perf_level,
            default_profile,
            entries: entries?,
        })
    }
}

impl ConfigEntry {
    fn parse_perf_level(&self) -> Result<Option<DpmForcedLevel>, ParseConfigError> {
        let perf_level = if let Some(ref s) = self.perf_level {
            if let Some(perf_level) = perf_level_from_str(s) {
                Some(perf_level)
            } else {
                return Err(ParseConfigError::InvalidPerfLevel(s.to_string()));
            }
        } else {
            None
        };

        Ok(perf_level)
    }

    fn parse_power_profile(&self) -> Result<Option<PowerProfile>, ParseConfigError> {
        let profile = if let Some(ref s) = self.profile {
            if let Some(profile) = power_profile_from_str(s) {
                Some(profile)
            } else {
                return Err(ParseConfigError::InvalidProfile(s.to_string()));
            }
        } else {
            None
        };

        Ok(profile)
    }

    pub fn parse(&self) -> Result<ParsedConfigEntry, ParseConfigError> {
        if self.name.is_empty() {
            return Err(ParseConfigError::EntryNameIsEmpty);
        }

        let name = self.name.clone();
        let perf_level = self.parse_perf_level()?;
        let profile = self.parse_power_profile()?;
        let power_cap_watt = self.power_cap_watt;

        Ok(ParsedConfigEntry { name, perf_level, profile, power_cap_watt })
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
        // "profile_exit" => DpmForcedLevel::ProfileExit,
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
