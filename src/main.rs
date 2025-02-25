use std::ffi::c_schar;
use std::path::PathBuf;

use clap::Parser;
use hydra::db::DB;
use hydra::evaluator::Coordinator;
use tracing::{debug, error, info, warn, Level};
use tracing_subscriber;

mod hydra;

#[derive(Parser)]
struct Args {
    #[arg(short, long = "data-dir", help = "The data directory to use")]
    data_dir: PathBuf,

    #[arg(short='v', long, action = clap::ArgAction::Count, help="Sets the verbose level. More v's more output")]
    verbose: u8,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let logger = tracing_subscriber::fmt();

    let logger = match args.verbose {
        0 => logger.with_max_level(Level::INFO),
        1 => logger.with_max_level(Level::DEBUG),
        _ => logger.with_max_level(Level::TRACE),
    };

    logger.init();

    // debug!("data_dir path is: {}", args.data_dir.to_str().unwrap());

    let path = args.data_dir.join("db.sqlite");

    let db = DB::new(path.to_str().unwrap()).await;

    if db.is_err() {
        error!("Failed to create database: {}", db.err().unwrap());
        return;
    }

    let db = db.unwrap();

    let mut coordinator = Coordinator::new();

    let schedule = vec![
        // r#"path:///home/ole/nixos#nixosConfigurations."main".config.system.build.toplevel"#,
        // r#"path:///home/ole/nixos#nixosConfigurations."wattson".config.system.build.toplevel"#,
        // r#"path:///home/ole/nixos#nixosConfigurations."teapot".config.system.build.toplevel"#,
        "path:///home/ole/nixos#hydraJobs",
        "github:DestinyofYeet/add_replay_gain#hydraJobs",
    ];

    for uri in schedule.iter() {
        coordinator.schedule(uri).await;
    }

    coordinator.shutdown().await;
}
