use std::{ffi::OsStr, path::PathBuf};

use clap::Parser;
use log::info;
use misc_conf::apache::Apache;

use crate::cmd::configs::ProxyConfig;

pub mod configs;
mod logging;

#[derive(Debug, Parser)]
#[command(about = "ProxyParser is a tool to parse nginx and apache config files")]
pub struct Cli {
    #[arg(short, long)]
    pub verbose: Option<u8>,

    #[arg(short, long, help = "The path to the config files")]
    pub starting_dir: PathBuf,

    #[arg(
        short,
        long,
        help = "The extension of the files to parse",
        default_value = "conf"
    )]
    pub extension: Option<String>,
}

pub fn exec() {
    let args: Cli = Cli::parse();
    logging::init_logger(args.verbose);

    info!("Starting ProxyParser");

    let starting_dir = args.starting_dir;

    let extension: &OsStr = args.extension.as_deref().unwrap_or("conf").as_ref();

    use walkdir::WalkDir;

    for entry in WalkDir::new(&starting_dir)
        .into_iter()
        .filter_map(Result::ok)
    {
        if entry
            .path()
            .extension()
            .map_or(false, |ext| ext == extension)
        {
            process(entry.path().to_path_buf());
        }
    }
}

fn process(file_path: PathBuf) -> Vec<ProxyConfig> {
    info!("Processing file: {:?}", file_path);
    use misc_conf::ast::*;

    let data = std::fs::read(file_path).expect("unable to read file");

    let mut configs = Vec::new();
    if let Ok(res) = Directive::<Apache>::parse(&data) {
        for directive in res {
            let virtual_host = ProxyConfig::from(directive);
            configs.push(virtual_host);
        }
    }

    configs
}
