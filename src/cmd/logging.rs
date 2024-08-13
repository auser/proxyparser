use log::LevelFilter;

pub fn init_logger(verbose: Option<u8>) {
    let log_level = match verbose {
        Some(0) => LevelFilter::Info,
        Some(1) => LevelFilter::Debug,
        Some(2) => LevelFilter::Trace,
        _ => LevelFilter::Info,
    };

    env_logger::builder().filter_level(log_level).init();
}
