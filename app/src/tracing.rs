use tracing::subscriber;

/// Installs a global subscriber that ignores every span and event, so `tracing` instrumentation
/// never writes log lines or does export work.
pub fn init() -> anyhow::Result<()> {
    subscriber::set_global_default(subscriber::NoSubscriber::new())?;
    Ok(())
}
