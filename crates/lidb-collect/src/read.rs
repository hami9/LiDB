#[cfg(target_os = "linux")]
use crate::{error, MAX_SOURCE_BYTES};
use crate::{unsupported, State};
#[cfg(target_os = "linux")]
use lidb_core::MetricState;
use lidb_core::SnapshotMode;
use std::path::Path;

pub(crate) fn source(root: &Path, relative: &str, mode: SnapshotMode) -> Result<String, State> {
    #[cfg(target_os = "linux")]
    {
        if !cfg!(any(target_arch = "x86_64", target_arch = "aarch64")) {
            return Err(unsupported(
                "host collection requires Linux x86_64 or aarch64",
            ));
        }
        linux_source(root, relative, mode)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (root, relative, mode);
        Err(unsupported(
            "host collection requires Linux x86_64 or aarch64",
        ))
    }
}

#[cfg(target_os = "linux")]
const O_NONBLOCK: i32 = 0x800;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
const O_NOFOLLOW: i32 = 0x20000;
#[cfg(all(target_os = "linux", not(target_arch = "x86_64")))]
const O_NOFOLLOW: i32 = 0x8000;

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
    // Stable Linux ABI flags: O_NONBLOCK=0x800.
    // O_NOFOLLOW is 0x20000 on x86_64 and 0x8000 on aarch64 (asm-generic).
    // These protect against opening a substituted FIFO or final-component symlink
    // between metadata inspection and open, without privileged/unsafe code.
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(O_NONBLOCK | O_NOFOLLOW)
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

#[cfg(target_os = "linux")]
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

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    #[test]
    fn open_flags_refuse_final_symlink() {
        use std::os::unix::fs::{symlink, OpenOptionsExt};
        let directory = std::env::temp_dir().join(format!("lidb-nofollow-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir(&directory).unwrap();
        std::fs::write(directory.join("target"), "data").unwrap();
        symlink(directory.join("target"), directory.join("link")).unwrap();
        let result = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(O_NONBLOCK | O_NOFOLLOW)
            .open(directory.join("link"));
        std::fs::remove_dir_all(&directory).unwrap();
        // ELOOP is 40 on both x86_64 and aarch64.
        assert_eq!(
            result.err().and_then(|error| error.raw_os_error()),
            Some(40)
        );
    }

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
