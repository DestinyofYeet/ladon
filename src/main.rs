use std::path::PathBuf;

use hydra::evaluator::EvalManager;
use hydra::db::DB;
use clap::Parser;
use tracing_subscriber;
use tracing::{Level, debug, warn, error, info};

mod hydra;

#[derive(Parser)]
struct Args {
    #[arg(short, long="data-dir", help="The data directory to use")]
    data_dir: PathBuf,
    
    #[arg(short='v', long, action = clap::ArgAction::Count, help="Sets the verbose level. More v's more output")]
    verbose: u8
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

    // let mut eval_manager = EvalManager::new(db).await;
    
    let schedule = [
        ("path:///home/ole/nixos",r#"nixosConfigurations."main".config.system.build.toplevel"#),
        ("path:///home/ole/nixos", r#"nixosConfigurations."wattson".config.system.build.toplevel"#),
        ("path:///home/ole/nixos", r#"nixosConfigurations."teapot".config.system.build.toplevel"#),
    ];

    let mut handles = Vec::new();

    // for (key, value) in schedule {
    //     let handle = eval_manager.schedule(
    //         key,
    //         value,
    //     ).await.unwrap();

    //     handles.push(handle);
    // };

    // for handle in handles {
    //     info!("Waiting for id {handle}");
    //     eval_manager.wait_handle(handle).await;
    // };

    // eval_manager.shutdown().await;
    

    // let result = eval_manager.wait_handle(handle).await;

    // let result = result.lock().await;

    // if result.is_ok() {
    //     let result = result.as_ref().unwrap();
    //     let duration = result.finished_at.duration_since(result.started_at);
    //     info!("Successfully built in {} seconds", duration.as_secs());
    // } else {
    //     let error = result.as_ref().err().unwrap();
    //     error!("Evaluation failed because: {}", error)
    // }
}
