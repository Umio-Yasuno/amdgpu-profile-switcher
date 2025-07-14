use std::path::PathBuf;
use std::fs;

use libdrm_amdgpu_sys::PCI;
use libdrm_amdgpu_sys::AMDGPU::{self, PowerCap, PowerProfile};

pub struct AmdgpuDevice {
    pub pci_bus: PCI::BUS_INFO,
    pub sysfs_path: PathBuf,
    pub hwmon_path: PathBuf,
    pub device_id: u32,
    pub revision_id: u32,
    pub device_name: String,
    pub power_profile_path: PathBuf,
    pub dpm_perf_level_path: PathBuf,
    pub power_cap: Option<PowerCap>,
    pub fan_target_temperature: Option<FanTargetTemp>,
    pub fan_minimum_pwm: Option<FanMinPwm>,
    pub sclk_offset: Option<SclkOffset>, // RDNA 4
    pub vddgfx_offset: Option<VddgfxOffset>, // RDNA 2/3/4
    pub fan_zero_rpm: Option<bool>,
}

impl AmdgpuDevice {
    pub fn get_from_pci_bus(pci_bus: PCI::BUS_INFO) -> Option<Self> {
        let sysfs_path = pci_bus.get_sysfs_path();
        let power_profile_path = sysfs_path.join("pp_power_profile_mode");
        let dpm_perf_level_path = sysfs_path.join("power_dpm_force_performance_level");

        if !power_profile_path.exists() || !dpm_perf_level_path.exists() {
            return None;
        }

        let [device_id, revision_id] = {
            let [did, rid] = ["device", "revision"]
                .map(|s| std::fs::read_to_string(sysfs_path.join(s)).ok());

            [did?, rid?]
                .map(|s|
                    u32::from_str_radix(s.trim_start_matches("0x").trim_end(), 16).unwrap()
                )
        };
        let device_name = AMDGPU::find_device_name(device_id, revision_id)
            .unwrap_or(AMDGPU::DEFAULT_DEVICE_NAME.to_string());
        let hwmon_path = pci_bus.get_hwmon_path()?;
        let power_cap = PowerCap::from_hwmon_path(&hwmon_path);
        let fan_target_temperature = FanTargetTemp::from_sysfs_path(&sysfs_path);
        let fan_minimum_pwm = FanMinPwm::from_sysfs_path(&sysfs_path);
        let pp_od_clk_voltage = std::fs::read_to_string(sysfs_path.join("pp_od_clk_voltage"));
        let (sclk_offset, vddgfx_offset) = if let Ok(s) = pp_od_clk_voltage {
            (SclkOffset::from_str(&s), VddgfxOffset::from_str(&s))
        } else {
            (None, None)
        };
        let fan_zero_rpm = FanZeroRpm::from_sysfs_path(&sysfs_path);

        Some(Self {
            pci_bus,
            sysfs_path,
            hwmon_path,
            device_id,
            revision_id,
            device_name,
            power_profile_path,
            dpm_perf_level_path,
            power_cap,
            fan_target_temperature,
            fan_minimum_pwm,
            sclk_offset,
            vddgfx_offset,
            fan_zero_rpm,
        })
    }

    pub fn _update(&mut self) {
        self.power_cap = PowerCap::from_hwmon_path(&self.hwmon_path);
        self.fan_target_temperature = FanTargetTemp::from_sysfs_path(&self.sysfs_path);
        self.fan_minimum_pwm = FanMinPwm::from_sysfs_path(&self.sysfs_path);
        let pp_od_clk_voltage = std::fs::read_to_string(self.sysfs_path.join("pp_od_clk_voltage"));
        if let Ok(s) = pp_od_clk_voltage {
            self.sclk_offset = SclkOffset::from_str(&s);
            self.vddgfx_offset = VddgfxOffset::from_str(&s);
        }
    }

    pub fn check_permissions(&self) -> bool {
        [&self.power_profile_path, &self.dpm_perf_level_path]
            .iter()
            .any(|path| {
                fs::OpenOptions::new().read(true).write(true).open(path).is_ok()
            })
    }

    pub fn get_all_supported_power_profile(&self) -> Vec<PowerProfile> {
        PowerProfile::get_all_supported_profiles_from_sysfs(&self.sysfs_path)
    }
}

#[derive(Debug, Clone)]
pub struct FanZeroRpm;

impl FanZeroRpm {
    pub fn from_sysfs_path<P: Into<PathBuf>>(path: P) -> Option<bool> {
        let current: bool;

        {
            let path = path.into().join("gpu_od/fan_ctrl/fan_zero_rpm_enable");
            let s = std::fs::read_to_string(path).ok()?;
            let mut lines = s.lines();

            lines.find(|l| l.starts_with("FAN_ZERO_RPM_ENABLE:"));
            current = lines.next()? == "1";
        }

        Some(current)
    }
}

#[derive(Debug, Clone)]
pub struct SclkOffset {
    pub current: i32,
    pub range: Option<[i32; 2]>,
}

impl SclkOffset {
    pub fn from_str(s: &str) -> Option<Self> {
        fn parse_mhz(s: &str) -> Option<i32> {
            let len = s.len();
            s.get(..len-3)?.parse::<i32>().ok()
        }

        let mut lines = s.lines();
        let _ = lines.find(|l| l.ends_with("OD_SCLK_OFFSET:"))?;
        let current = lines.next().and_then(parse_mhz)?;
        let s_range = lines.find(|l| l.starts_with("SCLK_OFFSET:"))?;
        let range = {
            let mut split = s_range
                .trim_start_matches("SCLK_OFFSET:")
                .split_whitespace();
            if let [Some(min), Some(max)] = [split.next(), split.next()]
                .map(|v| v.and_then(parse_mhz))
            {
                Some([min, max])
            } else {
                None
            }
        };

        Some(Self {
            current,
            range,
        })
    }

    pub fn _from_sysfs_path<P: Into<PathBuf>>(path: P) -> Option<Self> {
        let s = std::fs::read_to_string(path.into().join("pp_od_clk_voltage")).ok()?;

        Self::from_str(&s)
    }
}

#[derive(Debug, Clone)]
pub struct VddgfxOffset {
    pub current: i32,
    pub range: Option<[i32; 2]>,
}

impl VddgfxOffset {
    pub fn from_str(s: &str) -> Option<Self> {
        fn parse_mv(s: &str) -> Option<i32> {
            let len = s.len();
            s.get(..len-2)?.parse::<i32>().ok()
        }

        let mut lines = s.lines();
        let _ = lines.find(|l| l.ends_with("OD_VDDGFX_OFFSET:"))?;
        let current = lines.next().and_then(parse_mv)?;
        let s_range = lines.find(|l| l.starts_with("VDDGFX_OFFSET:"))?;
        let range = {
            let mut split = s_range
                .trim_start_matches("VDDGFX_OFFSET:")
                .split_whitespace();
            if let [Some(min), Some(max)] = [split.next(), split.next()]
                .map(|v| v.and_then(parse_mv))
            {
                Some([min, max])
            } else {
                None
            }
        };

        Some(Self {
            current,
            range,
        })
    }

    pub fn _from_sysfs_path<P: Into<PathBuf>>(path: P) -> Option<Self> {
        let s = std::fs::read_to_string(path.into().join("pp_od_clk_voltage")).ok()?;

        Self::from_str(&s)
    }
}

#[derive(Debug, Clone)]
pub struct FanTargetTemp {
    pub target_temp: u32,
    pub temp_range: [u32; 2],
}

impl FanTargetTemp {
    pub fn from_sysfs_path<P: Into<PathBuf>>(path: P) -> Option<Self> {
        let target_temp: Option<u32>;
        let temp_range: Option<[u32; 2]>;

        {
            let path = path.into().join("gpu_od/fan_ctrl/fan_target_temperature");
            let s = std::fs::read_to_string(path).ok()?;
            let mut lines = s.lines();

            lines.find(|l| l.starts_with("FAN_TARGET_TEMPERATURE:"));
            target_temp = lines.next().and_then(|s| s.parse().ok());
            lines.find(|l| l.starts_with("OD_RANGE:"));
            temp_range = lines.next().and_then(|s| {
                let (min, max) = s
                    .trim_start_matches("TARGET_TEMPERATURE: ")
                    .split_once(" ")?;
                let [min, max] = [min, max].map(|s| s.parse::<u32>().ok());

                Some([min?, max?])
            });
        }

        Some(Self {
            target_temp: target_temp?,
            temp_range: temp_range?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct FanMinPwm {
    pub minimum_pwm: u32,
    pub pwm_range: [u32; 2],
}

impl FanMinPwm {
    pub fn from_sysfs_path<P: Into<PathBuf>>(path: P) -> Option<Self> {
        let minimum_pwm: Option<u32>;
        let pwm_range: Option<[u32; 2]>;

        {
            let path = path.into().join("gpu_od/fan_ctrl/fan_minimum_pwm");
            let s = std::fs::read_to_string(path).ok()?;
            let mut lines = s.lines();

            lines.find(|l| l.starts_with("FAN_MINIMUM_PWM:"));
            minimum_pwm = lines.next().and_then(|s| s.parse().ok());
            lines.find(|l| l.starts_with("OD_RANGE:"));
            pwm_range = lines.next().and_then(|s| {
                let (min, max) = s
                    .trim_start_matches("MINIMUM_PWM: ")
                    .split_once(" ")?;
                let [min, max] = [min, max].map(|s| s.parse::<u32>().ok());

                Some([min?, max?])
            });
        }

        Some(Self {
            minimum_pwm: minimum_pwm?,
            pwm_range: pwm_range?,
        })
    }
}
