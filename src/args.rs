const HELP_MSG: &str = concat!(
    env!("CARGO_PKG_NAME"), " ", env!("CARGO_PKG_VERSION"), "\n",
    env!("CARGO_PKG_HOMEPAGE"), "\n",
    "\n",
    "USAGE:\n",
    "    # <", env!("CARGO_PKG_NAME"), "> [options ..]\n",
    "\n",
    "FLAGS:\n",
    "    --procs\n",
    "        Dump current all process names.\n",
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
    pub app_mode: AppMode,
}

impl MainOpt {
    pub fn parse() -> Self {
        let args = &std::env::args().skip(1).collect::<Vec<String>>();
        let mut opt = Self::default();

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
