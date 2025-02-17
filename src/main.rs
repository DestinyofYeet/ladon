use evaluator::evaluator::Evaluator;
use clap::Parser;
use tracing_subscriber;
use tracing::{Level, debug, warn, error, info};

#[derive(Parser)]
struct Args {
    #[arg(short, long="data-dir", help="The data directory to use")]
    data_dir: String,
    
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
    
    let mut eval = Evaluator::new(
        // "git+https://git.ole.blue/ole/nix-config",
        "path:///home/ole/nixos",
        // "hydraJobs"
        r#"nixosConfigurations."main".config.system.build.toplevel"#
    );

    let result = eval.start().await;

    if result.is_ok() {
        let result = result.unwrap();
        let duration = result.finished_at.duration_since(result.started_at);
        info!("Successfully built in {} seconds", duration.as_secs());
    } else {
        let error = result.err().unwrap();
        error!("Evaluation failed because: {}", error)
    }
}
