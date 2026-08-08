use std::io::ErrorKind;
use std::path::Path;

use crate::rollout::SESSIONS_SUBDIR;
use codex_protocol::error::SolaiAgentErr;
use codex_thread_store::ThreadStoreError;

pub(crate) fn map_session_init_error(err: &anyhow::Error, codex_home: &Path) -> SolaiAgentErr {
    if let Some(ThreadStoreError::Unsupported { operation }) = err
        .chain()
        .find_map(|cause| cause.downcast_ref::<ThreadStoreError>())
    {
        return SolaiAgentErr::UnsupportedOperation(format!("{operation} is not supported yet"));
    }

    if let Some(mapped) = err
        .chain()
        .filter_map(|cause| cause.downcast_ref::<std::io::Error>())
        .find_map(|io_err| map_rollout_io_error(io_err, codex_home))
    {
        return mapped;
    }

    SolaiAgentErr::Fatal(format!("Failed to initialize session: {err:#}"))
}

fn map_rollout_io_error(io_err: &std::io::Error, codex_home: &Path) -> Option<SolaiAgentErr> {
    let sessions_dir = codex_home.join(SESSIONS_SUBDIR);
    let hint = match io_err.kind() {
        ErrorKind::PermissionDenied => format!(
            "SolaiAgent cannot access session files at {} (permission denied). If sessions were created using sudo, fix ownership: sudo chown -R $(whoami) {}",
            sessions_dir.display(),
            codex_home.display()
        ),
        ErrorKind::NotFound => format!(
            "Session storage missing at {}. Create the directory or choose a different SolaiAgent home.",
            sessions_dir.display()
        ),
        ErrorKind::AlreadyExists => format!(
            "Session storage path {} is blocked by an existing file. Remove or rename it so SolaiAgent can create sessions.",
            sessions_dir.display()
        ),
        ErrorKind::InvalidData | ErrorKind::InvalidInput => format!(
            "Session data under {} looks corrupt or unreadable. Clearing the sessions directory may help (this will remove saved threads).",
            sessions_dir.display()
        ),
        ErrorKind::IsADirectory | ErrorKind::NotADirectory => format!(
            "Session storage path {} has an unexpected type. Ensure it is a directory SolaiAgent can use for session files.",
            sessions_dir.display()
        ),
        _ => return None,
    };

    Some(SolaiAgentErr::Fatal(format!(
        "{hint} (underlying error: {io_err})"
    )))
}
