use chrono::Local;
use color_eyre::Result;
use fern::Dispatch;
use log::LevelFilter;
use std::fs::OpenOptions;

pub fn setup_logging() -> Result<()> {
    let log_file_name = format!(
        "astro-swarm-{}.log",
        Local::now().format("%Y-%m-%d_%H-%M-%S")
    );
    let log_file = OpenOptions::new()
        .create(true)
        .write(true)
        .append(true)
        .open(log_file_name)?;

    Dispatch::new()
        .level(LevelFilter::Trace)
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{}][{}][{}] {}",
                Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.target(),
                message
            ))
        })
        .chain(log_file)
        .apply()?;

    log::info!("TUI file logging initialized");
    Ok(())
}
