use std::{ffi::OsStr, path::PathBuf};

use clap::Parser;
use log::info;
use misc_conf::apache::Apache;

use crate::cmd::configs::ProxyConfig;

pub mod configs;
mod interact;
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

    #[arg(short, long, help = "Interactive mode")]
    pub interactive: bool,

    #[arg(short, long, help = "Print the config commands")]
    pub print_commands: bool,
}

pub fn exec() {
    let args: Cli = Cli::parse();
    logging::init_logger(args.verbose);

    info!("Starting ProxyParser");

    let starting_dir = args.starting_dir;

    let extension: &OsStr = args.extension.as_deref().unwrap_or("conf").as_ref();

    use walkdir::WalkDir;

    let mut configs = ProxyConfig::default();
    for entry in WalkDir::new(&starting_dir)
        .into_iter()
        .filter_map(Result::ok)
    {
        if entry
            .path()
            .extension()
            .map_or(false, |ext| ext == extension)
        {
            let pc = process(entry.path().to_path_buf());
            pc.virtual_hosts.into_iter().for_each(|virtual_host| {
                configs.add_virtual_host(virtual_host);
            });
        }
    }

    // if args.interactive {
    //     let _ = interact::exec(configs);
    // }

    if args.print_commands {
        print_commands(configs);
    }
}

fn process(file_path: PathBuf) -> ProxyConfig {
    info!("Processing file: {:?}", file_path);
    use misc_conf::ast::*;

    let data = std::fs::read(file_path).expect("unable to read file");

    let mut configs = ProxyConfig::default();
    if let Ok(res) = Directive::<Apache>::parse(&data) {
        for directive in res {
            let pc = ProxyConfig::from(directive);
            pc.virtual_hosts.into_iter().for_each(|virtual_host| {
                configs.add_virtual_host(virtual_host);
            });
        }
    }

    configs
}

fn print_commands(configs: ProxyConfig) {
    for virtual_host in configs.virtual_hosts {
        println!("{}", virtual_host.to_etcd_config());
    }
}
