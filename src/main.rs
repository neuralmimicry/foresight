#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    nm_workflow_core::runtime::serve(
        nm_workflow_core::App::from_env("foresight").await?,
        foresight::routes(),
    )
    .await?;
    Ok(())
}
