use crate::{error, unsupported, State, MAX_SOURCE_BYTES};
use lidb_core::{MetricState, SnapshotMode};
use std::path::Path;

pub(crate) fn source(root: &Path, relative: &str, mode: SnapshotMode) -> Result<String, State> {
    #[cfg(target_os = "linux")]
    {
        linux_source(root, relative, mode)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (root, relative, mode);
        Err(unsupported("host collection requires Linux"))
    }
}

#[cfg(target_os = "linux")]
fn linux_source(root: &Path, relative: &str, mode: SnapshotMode) -> Result<String, State> {
    use std::fs::{self, OpenOptions};
    use std::io::Read;
    use std::os::unix::fs::OpenOptionsExt;

    if mode == SnapshotMode::Fixture {
        // Reject fixture directory symlinks, including /fixture/net -> /proc.
        // Kernel /proc/net -> self/net is allowed only for the exact live root.
        for directory in root.ancestors().chain(
            Path::new(relative)
                .parent()
                .filter(|parent| !parent.as_os_str().is_empty())
                .map(|parent| root.join(parent))
                .iter()
                .map(|path| path.as_path()),
        ) {
            if directory.as_os_str().is_empty() {
                continue;
            }
            let metadata = fs::symlink_metadata(directory).map_err(io_state)?;
            if !metadata.file_type().is_dir() {
                return Err(error("fixture source directory is not a real directory"));
            }
        }
    }
    let path = root.join(relative);
    let metadata = fs::symlink_metadata(&path).map_err(io_state)?;
    if !metadata.file_type().is_file() {
        return Err(error("source is not a regular file"));
    }
    // Stable Linux ABI flags: O_NONBLOCK=0x800, O_NOFOLLOW=0x20000. These
    // protect against opening a substituted FIFO or final-component symlink
    // between metadata inspection and open, without privileged/unsafe code.
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(0x800 | 0x20000)
        .open(path)
        .map_err(io_state)?;
    if !file.metadata().map_err(io_state)?.is_file() {
        return Err(error("opened source is not a regular file"));
    }
    let mut bytes = Vec::new();
    file.take((MAX_SOURCE_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(io_state)?;
    if bytes.len() > MAX_SOURCE_BYTES {
        return Err(error("source exceeds the byte limit"));
    }
    let text = String::from_utf8(bytes).map_err(|_| error("source contains invalid UTF-8"))?;
    if text
        .chars()
        .any(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
    {
        return Err(error("source contains terminal control characters"));
    }
    Ok(text)
}

fn io_state(error_value: std::io::Error) -> State {
    match error_value.kind() {
        std::io::ErrorKind::NotFound => unsupported("source is not present"),
        std::io::ErrorKind::PermissionDenied => {
            MetricState::PermissionDenied("source access is denied".into())
        }
        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::Interrupted => {
            crate::temporarily_unavailable("source read did not complete")
        }
        _ => error("source read failed"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_permission_denied_without_embedding_os_text() {
        assert_eq!(
            io_state(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "untrusted\u{1b}[2Jpath"
            )),
            MetricState::PermissionDenied("source access is denied".into())
        );
    }
}
