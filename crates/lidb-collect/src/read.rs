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
fn linux_source(root: &Path, relative: &str, mode: SnapshotMode) -> Result<String, State> {
    use std::fs;
    use std::io::Read;

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
    let file = open_source(&path)?;
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
fn open_source(path: &Path) -> Result<std::fs::File, State> {
    use std::fs::OpenOptions;
    use std::os::unix::fs::OpenOptionsExt;

    // Linux arm64 overrides asm-generic's O_NOFOLLOW. On arm64 0x20000
    // means O_LARGEFILE, which cannot stop a replaced final-component link.
    // source() restricts this helper to the two explicitly supported ABIs.
    #[cfg(target_arch = "aarch64")]
    const O_NOFOLLOW: i32 = 0x8000;
    #[cfg(not(target_arch = "aarch64"))]
    const O_NOFOLLOW: i32 = 0x20000;
    const O_NONBLOCK: i32 = 0x800;
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(O_NONBLOCK | O_NOFOLLOW)
        .open(path)
        .map_err(io_state)?;
    if !file.metadata().map_err(io_state)?.is_file() {
        return Err(error("opened source is not a regular file"));
    }
    Ok(file)
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
    fn reports_permission_denied_without_embedding_os_text() {
        assert_eq!(
            io_state(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "untrusted\u{1b}[2Jpath"
            )),
            MetricState::PermissionDenied("source access is denied".into())
        );
    }

    #[test]
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    fn open_itself_rejects_symlinks_without_relying_on_preinspection() {
        use std::{fs, os::unix::fs::symlink, time::SystemTime};
        let unique = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory =
            std::env::temp_dir().join(format!("lidb-open-flags-{}-{unique}", std::process::id()));
        fs::create_dir(&directory).unwrap();
        let regular = directory.join("regular");
        let link = directory.join("replacement-link");
        fs::write(&regular, "bounded fixture").unwrap();
        symlink(&regular, &link).unwrap();
        assert!(open_source(&regular).is_ok());
        // Exercise the open boundary directly: source()'s earlier metadata
        // rejection would conceal an incorrect O_NOFOLLOW value on arm64.
        assert!(matches!(open_source(&link), Err(MetricState::Error(_))));
        fs::remove_dir_all(directory).unwrap();
    }
}
