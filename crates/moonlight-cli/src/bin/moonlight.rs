#[tokio::main]
async fn main() -> anyhow::Result<()> {
    moonlight_cli::run_cli().await
}
