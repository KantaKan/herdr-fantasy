use std::path::PathBuf;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::EnvFilter;

/// `$HERDR_PLUGIN_STATE_DIR`, else the platform state dir (`~/.local/state/herdr-fantasy` on Linux).
pub fn state_dir() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("HERDR_PLUGIN_STATE_DIR") {
        return Some(dir.into());
    }
    let dirs = directories::ProjectDirs::from("", "", "herdr-fantasy")?;
    Some(
        dirs.state_dir()
            .unwrap_or_else(|| dirs.data_local_dir())
            .to_path_buf(),
    )
}

/// Log to `<state_dir>/logs/herdr-fantasy.<date>.log`, daily rotation, keep 3.
/// Never logs to stdout/stderr: the stage owns the terminal. Logging is best-effort.
pub fn init() -> Option<WorkerGuard> {
    let dir = state_dir()?.join("logs");
    std::fs::create_dir_all(&dir).ok()?;
    let appender = RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .max_log_files(3)
        .filename_prefix("herdr-fantasy")
        .filename_suffix("log")
        .build(dir)
        .ok()?;
    let (writer, guard) = tracing_appender::non_blocking(appender);
    tracing_subscriber::fmt()
        .with_writer(writer)
        .with_ansi(false)
        .with_env_filter(
            EnvFilter::try_from_env("HERDR_FANTASY_LOG").unwrap_or_else(|_| "info".into()),
        )
        .try_init()
        .ok()?;
    Some(guard)
}
