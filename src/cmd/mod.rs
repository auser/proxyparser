use std::{collections::HashMap, ffi::OsStr, path::PathBuf};

use clap::Parser;
use log::{debug, info};
use misc_conf::apache::Apache;

use crate::{
    cmd::configs::{ProxyConfig, VirtualHostBuilder},
    error::ParserResult,
};

pub mod configs;
mod interact;
mod logging;

#[derive(Debug, Parser)]
#[command(about = "ProxyParser is a tool to parse nginx and apache config files")]
pub struct Cli {
    #[arg(short, long, default_value = "0")]
    pub verbose: Option<u8>,

    #[arg(
        value_name = "starting_dir",
        help = "The path to the config files",
        index = 1
    )]
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

    #[arg(short('m'), long, help = "Print the middleware commands for traefik")]
    pub print_middleware_commands: bool,

    #[arg(short, long, help = "Print the config commands")]
    pub print_commands: bool,

    #[arg(
        short,
        long,
        help = "Type of the config to parse",
        default_value = "etcd"
    )]
    pub config_type: String,
}

pub fn exec() -> ParserResult<()> {
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
            debug!("Processing file: {:?}", entry.path());
            let pc = process(entry.path().to_path_buf())?;
            pc.virtual_hosts.into_iter().for_each(|virtual_host| {
                configs.add_virtual_host(virtual_host);
            });
        }
    }

    // if args.interactive {
    //     let _ = interact::exec(configs);
    // }

    if args.print_middleware_commands {
        print_middleware_commands(&configs);
        println!("\n");
    }

    if args.print_commands {
        print_commands(&configs, &args.config_type);
    }

    Ok(())
}

fn process(file_path: PathBuf) -> ParserResult<ProxyConfig> {
    info!("Processing file: {:?}", file_path);
    let extension: &OsStr = file_path.extension().unwrap_or(OsStr::new("conf"));
    let mut configs = ProxyConfig::default();

    if extension == "conf" {
        process_apache(file_path, &mut configs)?;
    } else if extension == "xlsx" {
        process_xlsx(file_path, &mut configs)?;
    }

    Ok(configs)
}

fn process_apache(file_path: PathBuf, configs: &mut ProxyConfig) -> ParserResult<()> {
    info!("Processing file: {:?}", file_path);
    use misc_conf::ast::*;

    let data = std::fs::read(file_path).expect("unable to read file");
    if let Ok(res) = Directive::<Apache>::parse(&data) {
        for directive in res {
            let pc = ProxyConfig::from(directive);
            pc.virtual_hosts.into_iter().for_each(|virtual_host| {
                configs.add_virtual_host(virtual_host);
            });
        }
    }

    Ok(())
}

fn process_xlsx(file_path: PathBuf, configs: &mut ProxyConfig) -> ParserResult<()> {
    info!("Processing file: {:?}", file_path);
    use calamine::{open_workbook, Reader, Xlsx};
    let mut workbook: Xlsx<_> = open_workbook(file_path).expect("unable to read file");

    let sheet = workbook
        .worksheet_range("Sheet1")
        .expect("unable to read file");

    let headers = sheet.rows().next().unwrap();
    let headers = headers
        .iter()
        .map(|h| h.to_string().to_lowercase())
        .collect::<Vec<String>>();

    for row in sheet.rows() {
        let row_map = HashMap::new();
        let row_values = row
            .iter()
            .zip(headers.iter())
            // .map(|(h, header)| json!({ header: h.to_string() }))
            .fold(row_map, |mut row_map, (h, header)| {
                let header = header.trim().to_string().to_lowercase();
                let value = h.to_string().to_lowercase();
                row_map.insert(header, value);
                row_map
            });

        if row_values.contains_key("needed for traefik") {
            let needed_for_traefik = row_values
                .get("needed for traefik")
                .expect("missing needed for traefik");
            if needed_for_traefik.to_lowercase() == "y" {
                let hostname = row_values.get("host name").unwrap().trim().to_string();
                let host = row_values
                    .get("blue webproxy ip")
                    .unwrap()
                    .trim()
                    .to_string();
                let virtual_host = VirtualHostBuilder::default()
                    .host(host.clone())
                    .server_name(hostname.clone())
                    .build();
                // dbg!("virtual_host {:#?}", virtual_host);
                configs.add_virtual_host(virtual_host);
            }
        }

        // let virtual_host = VirtualHostBuilder::default()
        //     .host(row_values.get("Host").unwrap().to_string())
        //     .server_name(row_values.get("Server Name").unwrap().to_string())
        //     .document_root(row_values.get("Document Root").unwrap().to_string())
        //     .build();

        // configs.add_virtual_host(virtual_host);
    }

    Ok(())
}

fn print_commands(configs: &ProxyConfig, config_type: &str) {
    let mut json_configs = Vec::new();
    for virtual_host in &configs.virtual_hosts {
        match config_type {
            "etcd" => println!("{}\n", virtual_host.to_etcd_config()),
            "json" => {
                let json_config = virtual_host.to_json_config();
                if let Some(json_config) = json_config {
                    json_configs.push(json_config);
                }
            }
            _ => println!("Unknown config type"),
        }
    }
    if config_type == "json" {
        let j = serde_json::json!(json_configs);
        println!("{}", j.to_string());
    }
}

fn print_middleware_commands(_configs: &ProxyConfig) {
    println!("etcdctl put traefik.http.middlewares.secured.chain.middlewares https-only");
    println!("etcdctl put traefik/http/middlewares/https-only/redirectScheme/scheme https");
    println!("etcdctl put traefik/http/middlewares/follow-redirects/redirectregex/permanent true");
    println!("etcdctl put traefik.http.middlewares.secured.chain.middlewares https-only");
    println!("etcdctl put traefik/http/middlewares/https-only/redirectScheme/scheme https");
    println!("etcdctl put traefik/http/middlewares/https-only/redirectScheme/permanent true");
    println!("etcdctl put traefik/http/middlewares/https-only/redirectScheme/port 443");

    println!("etcdctl put traefik/http/middlewares/enable-headers/headers/accessControlAllowMethods \"GET, POST, OPTIONS, PUT, DELETE\"");
    println!("etcdctl put traefik/http/middlewares/enable-headers/headers/accessControlAllowHeaders \"Content-Type, Content-Length, Accept-Encoding, X-CSRF-Token, Authorization, accept, origin, Cache-Control, X-Requested-With\"");
    println!("etcdctl put traefik/http/middlewares/enable-headers/headers/accessControlAllowOriginList \"*\"");
    println!(
        "etcdctl put traefik/http/middlewares/enable-headers/headers/accessControlMaxAge \"3600\""
    );
    println!("etcdctl put traefik/http/middlewares/enable-headers/headers/addVaryHeader true");
}
