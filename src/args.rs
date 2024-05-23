const HELP_MSG: &str = concat!(
    env!("CARGO_PKG_NAME"), " ", env!("CARGO_PKG_VERSION"), "\n",
    env!("CARGO_PKG_HOMEPAGE"), "\n",
    "\n",
    "USAGE:\n",
    "    # <", env!("CARGO_PKG_NAME"), "> [commands] [options ..]\n",
    "\n",
    "COMMANDS:\n",
    "    add\n",
    "        Add the config entry to the config file.\n",
    "        `--pci, --name` must be specified. (`--perf_level, --profile` are optional)\n",
    "FLAGS:\n",
    "    --procs\n",
    "        Dump all current process names.\n",
    "    --check-config\n",
    "        Check the config file.\n",
    "    --generate-config\n",
    "        Output the config file to stdout.\n",
    "    --profiles\n",
    "        Dump all supported power profiles.\n",
    "    --help\n",
    "        Print help information.\n",
    "ENV:\n",
    "    APS_CONFIG_PATH\n",
    "        Specify the config file path.\n",
);

use crate::config::ConfigEntry;
use libdrm_amdgpu_sys::PCI;

#[derive(Default)]
pub enum SubCommand {
    AddEntry((PCI::BUS_INFO, ConfigEntry)),
    #[default]
    Nop,
}

#[derive(Default)]
pub enum AppMode {
    #[default]
    Run,
    DumpProcs,
    CheckConfig,
    GenerateConfig,
    DumpSupportedPowerProfile,
}

#[derive(Default)]
pub struct MainOpt {
    pub sub_command: SubCommand,
    pub app_mode: AppMode,
}

impl MainOpt {
    fn parse_add_subcommand(&mut self) {
        let mut args = std::env::args().skip(2);
        let mut pci = String::new();
        let mut entry = ConfigEntry::default();

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--pci" => pci = args
                    .next()
                    .unwrap_or_else(|| panic!("`--pci <String>` is missing.")),
                "--name" => entry.name = args
                    .next()
                    .map(|arg| arg.to_string())
                    .unwrap_or_else(|| panic!("`--name <String>` is missing.")),
                "--perf_level" => entry.perf_level = args
                    .next()
                    .map(|arg| arg.to_string())
                    .or_else(|| panic!("`--perf_level <String>` is missing.")),
                "--profile" => entry.profile = args
                    .next()
                    .map(|arg| arg.to_string())
                    .or_else(|| panic!("`--profile_level <String>` is missing.")),
                _ => panic!("Unknown Option for add: {arg:?}"),
            }
        }

        if pci.is_empty() {
            panic!("<String> for `--pci` is empty.");
        }

        if entry.name.is_empty() {
            panic!("<String> for `--name` is empty.");
        }

        if entry.perf_level.is_none() && entry.profile.is_none() {
            eprintln!("Warn: Both `perf_level` and `profile` are empty.");
        }

        let pci: PCI::BUS_INFO = pci.parse().unwrap_or_else(|e| {
            panic!("Error: {e:?} ({pci:?})");
        });

        // valid
        let _ = entry.parse().unwrap();

        self.sub_command = SubCommand::AddEntry((pci, entry));
    }

    pub fn parse() -> Self {
        let mut args = std::env::args().skip(1).peekable();
        let mut opt = Self::default();

        if let Some(first_arg) = args.peek() {
            match first_arg.as_str() {
                "add" => {
                    opt.parse_add_subcommand();
                    return opt;
                },
                _ => {},
            }
        }

        for arg in args {
            match arg.as_str() {
                "--procs" => opt.app_mode = AppMode::DumpProcs,
                "--check-config" => opt.app_mode = AppMode::CheckConfig,
                "--generate-config" => opt.app_mode = AppMode::GenerateConfig,
                "--profiles" => opt.app_mode = AppMode::DumpSupportedPowerProfile,
                "--help" => {
                    println!("{HELP_MSG}");
                    std::process::exit(0);
                },
                _ => panic!("Unknown Option: {arg:?}"),
            }
        }

        opt
    }
}
