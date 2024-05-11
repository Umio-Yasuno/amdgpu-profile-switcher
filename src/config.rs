use libdrm_amdgpu_sys::{AMDGPU, PCI};
use AMDGPU::StablePstateFlag;
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

#[derive(Debug, Clone)]
pub struct ParsedConfigEntry {
    pub name: String,
    pub pstate: StablePstateFlag,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub config_devices: Vec<ConfigPerDevice>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConfigPerDevice {
    pub pci: Option<String>,
    pub entries: Vec<ConfigEntry>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConfigEntry {
    pub name: String,
    pub pstate: String,
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
        let pci = self.pci.as_ref().expect("`pci` is `None`.");
        let pci: PCI::BUS_INFO = pci.parse().unwrap_or_else(|_| panic!("Parse Error: {:?}", pci));

        if self.entries.is_empty() {
            panic!("`entries` for {pci} is empty.");
        }

        let entries = self.entries.iter().map(|entry| {
            if entry.name.is_empty() {
                panic!("`name` for {pci} is empty.")
            }

            let name = entry.name.clone();
            let pstate = stable_pstate_flag_from_str(&entry.pstate).unwrap_or_else(|| panic!("`pstate` for {pci} ({entry:?}) is invalid. (Must be one of: None, Standard, Peak, MinSclk, MinMclk)"));

            ParsedConfigEntry { name, pstate }
        }).collect();

        ParsedConfigPerDevice { pci, entries }
    }
}

fn stable_pstate_flag_from_str(s: &str) -> Option<StablePstateFlag> {
    let flag = match s {
        "None" => StablePstateFlag::NONE,
        "Standard" => StablePstateFlag::STANDARD,
        "MinSclk" => StablePstateFlag::MIN_SCLK,
        "MinMclk" => StablePstateFlag::MIN_MCLK,
        "Peak" => StablePstateFlag::PEAK,
        _ => return None,
    };

    Some(flag)
}
