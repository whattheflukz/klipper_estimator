// use std::error::Error;

// use lib_klipper::glam::DVec3;
use lib_klipper::planner::{Planner, PrinterLimits};

use clap::Parser;
use once_cell::sync::OnceCell;
// use serde::Deserialize;
#[macro_use]
extern crate lazy_static;

mod cmd;
mod cfg;

#[derive(Parser, Debug)]
#[clap(version = env!("VERGEN_GIT_SEMVER_LIGHTWEIGHT"), author = "Lasse Dalegaard <dalegaard@gmail.com>")]
pub struct Opts {
    #[clap(long = "config_moonraker_url")]
    config_moonraker: Option<String>,

    #[clap(long = "config_file")]
    config_filename: Option<String>,

    #[clap(subcommand)]
    cmd: SubCommand,

    #[clap(skip)]
    config: OnceCell<PrinterLimits>,
}

impl Opts {
    fn printer_limits(&self, config_overrides: Option<Vec<String>>) -> &PrinterLimits {
        let cl: cfg::config_loader::Loader = cfg::config_loader::Loader::default();
        match self.config.get() {
            Some(limits) => limits,
            None => match cl.load_config(self.config_filename.as_ref(), self.config_moonraker.as_ref(), config_overrides) {
                Ok(limits) => {
                    let _ = self.config.set(limits);
                    self.config.get().unwrap()
                }
                Err(e) => {
                    eprintln!("Failed to load printer configuration: {:?}", e);
                    std::process::exit(1);
                }
            },
        }
    }

    fn make_planner(&self, config_overrides: Option<Vec<String>>) -> Planner {
        Planner::from_limits(self.printer_limits(config_overrides).clone())
    }
}

#[derive(Parser, Debug)]
enum SubCommand {
    Estimate(cmd::estimate::EstimateCmd),
    DumpMoves(cmd::estimate::DumpMovesCmd),
    PostProcess(cmd::post_process::PostProcessCmd),
    DumpConfig(cmd::dump_config::DumpConfigCmd),
}

impl SubCommand {
    fn run(&self, opts: &Opts) {
        match self {
            Self::Estimate(i) => i.run(opts),
            Self::DumpMoves(i) => i.run(opts),
            Self::PostProcess(i) => i.run(opts),
            Self::DumpConfig(i) => i.run(opts),
        }
    }
}

fn main() {
    let opts = Opts::parse();
    opts.cmd.run(&opts);
}
