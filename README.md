# amdgpu-profile-switcher
`amdgpu-profile-switcher` is a simple tool that automatically switches profiles for AMDGPU, and it also allows for advanced settings such as overclocking, undervolting, and fan control.  
The tool switches between perf level (`power_dpm_force_performance_level`) and power_profile (`pp_power_profile_mode`) depending on the process program name. (requires root privileges)  

## Usage
```
# amdgpu-profile-swicther [options ..]
```

```
# Add the config entry
amdgpu-profile-switcher add --pci 0000:08:00.0 --name glxgears --profile "BOOTUP_DEFAULT"
# or
amdgpu-profile-switcher add -i 0 --name glxgears --profile "BOOTUP_DEFAULT"
```

```
COMMANDS:
    add
        Add the config entry to the config file.
        `--pci <String>` or `-i/--index <usize>` and --name <String>` must be specified.
        (`--perf_level, --profile` are optional)
FLAGS:
    --procs
        Dump all current process names.
    --check-config
        Check the config file.
    --generate-config
        Output the config file to stdout.
    --profiles
        Dump all supported power profiles.
    --help
        Print help information.
```

If you want to specify a config file, set the path to the file in `APS_CONFIG_PATH`.  
The default config file paths are `/etc/amdgpu-profile-switcher.ron` or `/etc/xdg/amdgpu-profile-switcher.ron` or under `XDG_CONFIG_DIRS`.  

## Installation
### Manually
```
$ git clone https://github.com/Umio-Yasuno/amdgpu-profile-switcher
$ cd amdgpu-profile-switcher
$ cargo build --release
$ sudo cp ./target/release/amdgpu-profile-switcher /usr/bin/
$ sudo cp ./debian/amdgpu-profile-switcher.service /etc/systemd/system/
$ amdgpu-profile-switcher --generate-config | sudo tee /etc/xdg/amdgpu-profile-switcher.ron
$ sudo systemctl enable amdgpu-profile-switcher
$ sudo systemctl start amdgpu-profile-switcher
```

### Debian/Ubuntu (.deb)
```
$ git clone https://github.com/Umio-Yasuno/amdgpu-profile-switcher
$ cd amdgpu-profile-switcher
$ cargo deb
$ sudo dpkg -i ./target/debian/amdgpu-profile-switcher.*deb
$ amdgpu-profile-switcher --generate-config | sudo tee /etc/xdg/amdgpu-profile-switcher.ron
$ sudo systemctl enable amdgpu-profile-switcher
$ sudo systemctl start amdgpu-profile-switcher
```

## Config example
```rust
/*
    perf_level: [
        "auto",
        "low",
        "high",
        "manual",
        "profile_standard",
        "profile_peak",
        "profile_min_sclk",
        "profile_min_mclk",
        "perf_determinism",
    ],
    profile: [
        "BOOTUP_DEFAULT",
        "3D_FULL_SCREEN",
        "POWER_SAVING",
        "VIDEO",
        "VR",
        "COMPUTE",
        "CUSTOM",
        "WINDOW_3D",
        "CAPPED",
        "UNCAPPED",
    ],
*/
(
    config_devices: [
        (
            pci: "0000:03:00.0",
            _device_name: Some("AMD Radeon RX 9060 XT"),
            default_power_cap_watt: Some(180),
            _power_cap_watt_range: Some((126, 200)),
            default_perf_level: None,
            default_profile: None,
            default_fan_target_temperature: Some(70),
            _fan_target_temperature_range: Some((25, 110)),
            default_fan_minimum_pwm: Some(20),
            _fan_minimum_pwm_range: Some((20, 100)),
            sclk_offset: Some(-500),
            _sclk_offset_range: Some((-500, 1000)),
            vddgfx_offset: Some(-70),
            _vddgfx_offset_range: Some((-200, 0)),
            fan_zero_rpm: Some(true),
            acoustic_target_rpm_threshold: Some(2400),
            _acoustic_target_rpm_threshold_range: Some((500, 3500)),
            entries: [
                (
                    name: "glxgears",
                    perf_level: None,
                    profile: Some("BOOTUP_DEFAULT"),
                    power_cap_watt: None,
                    fan_target_temperature: None,
                    fan_minimum_pwm: None,
                    acoustic_target_rpm_threshold: None,
                ),
            ],
        ),
    ],
)
```

## Tips
 * If you want to apply a single setting to Steam games launched via Wine/Proton, I recommend adding an entry for "steam.exe".

## Reference
 * <https://www.kernel.org/doc/html/latest/gpu/amdgpu/thermal.html#gpu-sysfs-power-state-interfaces>
