# amdgpu-profile-switcher
`amdgpu-profile-switcher` is a simple tool that automatically switches profiles for AMDGPU.  
The tool switches between perf level (`power_dpm_force_performance_level`) and power_profile (`pp_power_profile_mode`) depending on the process program name. (requires root privileges)  

## Usage
```
# amdgpu-profile-swicther [options ..]
```

```
FLAGS:
   --procs
       Dump current all process names.
   --help
       Print help information.
```

If you want to specify a config file, set the path to the file in `APS_CONFIG_PATH`.  

## Config example
```
(
    config_devices: [
        (
            // AMDGPU device list can be get using `amdgpu_top --list` or `ls /sys/bus/pci/drivers/amdgpu/`
            pci: "0000:03:00.0",
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
            entries: [
                (name: "MonsterHunterWorld.exe", perf_level: Some("profile_standard")),
                (name: "mgsvtpp.exe", profile: Some("3D_FULL_SCREEN")),
            ],
        )
    ],
)
```

## Reference
 * <https://www.kernel.org/doc/html/latest/gpu/amdgpu/thermal.html#gpu-sysfs-power-state-interfaces>

## TODO
 *  hot reloading of the config file
