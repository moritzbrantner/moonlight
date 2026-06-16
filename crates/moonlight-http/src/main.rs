mod args;

use args::ProxyArgs;
use clap::Parser;
use moonlight_core::config::{load_optional_config, AppConfig};
use moonlight_http::{build_router, build_state};
use tokio::net::TcpListener;
use tracing_subscriber::{EnvFilter, FmtSubscriber};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();
    let args = ProxyArgs::parse();
    let file_config = load_optional_config(args.config.as_deref(), args.no_config)?;
    let mut config = AppConfig::defaults();
    config.apply_shared_config(&file_config);
    if let Some(http) = &file_config.http {
        config.apply_http_config(http)?;
    }
    let config = args.apply_to(config)?;
    let addr = config.bind_addr;
    let state = build_state(config).await?;
    let app = build_router(state);
    let listener = TcpListener::bind(addr).await?;
    tracing::info!(%addr, "moonlight-http listening");
    axum::serve(listener, app).await?;
    Ok(())
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new("moonlight_http=info,moonlight_core=info,tower_http=info")
    });
    let subscriber = FmtSubscriber::builder().with_env_filter(filter).finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("tracing subscriber initializes once");
}
