use shadowdiff_server::{build_router, build_state, config::AppConfig};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = AppConfig::from_env()?;
    let addr = config.bind_addr;
    let state = build_state(config).await?;
    let app = build_router(state);
    let listener = TcpListener::bind(addr).await?;
    println!("shadowdiff-server listening on http://{addr}");
    axum::serve(listener, app).await?;
    Ok(())
}
