#[tokio::main]
async fn main() -> std::process::ExitCode {
    match moonlight_cli::run_cli().await {
        Ok(code) => code,
        Err(error) => {
            eprintln!("Error: {error:#}");
            error
                .downcast_ref::<moonlight_cli::ExitCodeError>()
                .map(|error| std::process::ExitCode::from(error.code()))
                .unwrap_or(std::process::ExitCode::from(1))
        }
    }
}
