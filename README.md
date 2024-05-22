# amdgpu-profile-switcher
`amdgpu-profile-switcher` is a simple tool that automatically switches profiles for AMDGPU, not a overclock/undervolt tool.  
The tool switches between perf level (`power_dpm_force_performance_level`) and power_profile (`pp_power_profile_mode`) depending on the process program name. (requires root privileges)  

## Usage
```
# amdgpu-profile-swicther [options ..]
```

```
FLAGS:
   --procs
       Dump current all process names.
   --check-config
       Check config file.
   --generate-config
       Output config file to stdout.
   --help
       Print help information.
```

If you want to specify a config file, set the path to the file in `APS_CONFIG_PATH`.  
The default config file paths are `/etc/amdgpu-profile-switcher.ron` or `/etc/xdg/amdgpu-profile-switcher.ron` or under `XDG_CONFIG_DIRS`.  

## Installation
### Debian
```
$ git clone https://github.com/Umio-Yasuno/amdgpu-profile-switcher
$ cd amdgpu-profile-switcher
$ cargo deb
$ sudo dpkg -i ./target/debian/amdgpu-profile-switcher.*deb
$ amdgpu-profile-switcher --generate-config | sudo tee /etc/xdg/amdgpu-profile-switcher.ron
```

### systemd
```
sudo systemctl enable amdgpu-profile-switcher
sudo systemctl start amdgpu-profile-switcher
```

## Config example
```rust
// Config entries that are earlier take priority.
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
            entries: [
                (name: "glxgears", profile: Some("BOOTUP_DEFAULT")),
            ],
        ),
    ],
)
```

## Reference
 * <https://www.kernel.org/doc/html/latest/gpu/amdgpu/thermal.html#gpu-sysfs-power-state-interfaces>
