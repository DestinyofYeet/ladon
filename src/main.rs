use ladon::state;

use std::sync::Arc;

#[cfg(feature = "ssr")]
use clap::Parser;

#[cfg(feature = "ssr")]
#[derive(Parser)]
struct Args {
    #[arg(short, long = "data-dir", help = "The data directory to use")]
    data_dir: std::path::PathBuf,
    #[arg(short='v', long, action = clap::ArgAction::Count, help="Sets the verbose level. More v's more output")]
    verbose: u8,
}

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use ladon::hydracore;
    use tokio::sync::Mutex;
    use tracing::{error, Level};
    use tracing_subscriber;
    let args = Args::parse();

    let logger = tracing_subscriber::fmt();

    let logger = match args.verbose {
        0 => logger.with_max_level(Level::INFO),
        1 => logger.with_max_level(Level::DEBUG),
        _ => logger.with_max_level(Level::TRACE),
    };

    logger.init();

    let path = args.data_dir.join("db.sqlite");

    let db = hydracore::DB::new(path.to_str().unwrap()).await;

    if db.is_err() {
        error!("Failed to create database: {}", db.err().unwrap());
        return;
    }

    let db = db.unwrap();

    let coordinator = hydracore::Coordinator::new(db);

    let state = Arc::new(state::State {
        coordinator: Mutex::new(coordinator),
    });

    let result = state
        .clone()
        .coordinator
        .lock()
        .await
        .start_jobsets_timer(state.clone())
        .await;

    if result.is_err() {
        error!("Failed to start jobsets timer: {}", result.err().unwrap());
    }

    use axum::Router;
    use ladon::app::*;
    use leptos::logging::log;
    use leptos::prelude::*;
    use leptos_axum::{generate_route_list, LeptosRoutes};

    let conf = get_configuration(None).unwrap();
    let addr = conf.leptos_options.site_addr;
    let leptos_options = conf.leptos_options;
    // Generate the list of routes in your Leptos App
    let routes = generate_route_list(App);

    let app = Router::new()
        .leptos_routes_with_context(
            &leptos_options,
            routes,
            move || provide_context(state.clone()),
            {
                let leptos_options = leptos_options.clone();
                move || shell(leptos_options.clone())
            },
        )
        .fallback(leptos_axum::file_and_error_handler(shell))
        .with_state(leptos_options);

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    log!("listening on http://{}", &addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

#[cfg(not(feature = "ssr"))]
pub fn main() {
    // no client-side main function
    // unless we want this to work with e.g., Trunk for pure client-side testing
    // see lib.rs for hydration function instead
}
