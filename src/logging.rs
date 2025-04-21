use chrono::Local;
use color_eyre::Result;
use fern::Dispatch;
use log::LevelFilter;
use std::{fs::{self, OpenOptions}, path::PathBuf};

const LOG_DIR: &str = "logs";

pub fn setup_logging() -> Result<()> {

    fs::create_dir_all(LOG_DIR)?;

    let log_file_name = format!(
        "astro-swarm-{}.log",
        Local::now().format("%Y-%m-%d_%H-%M-%S")
    );

    let full_log_path = PathBuf::from(LOG_DIR).join(log_file_name);

    let log_file = OpenOptions::new()
        .create(true)
        .write(true)
        .append(true)
        .open(&full_log_path)?;

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

    log::info!(
        "File logging initialized. Log file: {}",
        full_log_path.display()
    );
    
    Ok(())
}
