//! Local IPC negotiation contracts. These are pure types only:
//! no Unix socket, remote listener, serialization, or authentication exists yet.
#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// Current pre-stable IPC schema version.
pub const PROTOCOL_VERSION: Version = Version { major: 0, minor: 1 };
/// Planned maximum accepted wire frame length (not yet enforced by a socket).
pub const MAX_FRAME_BYTES: usize = 1024 * 1024;

/// Public local protocol version. Major mismatch is an error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Version {
    /// Breaking protocol generation.
    pub major: u16,
    /// Additive protocol revision.
    pub minor: u16,
}

/// Safe capability negotiation result, independent of transport.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Compatibility {
    /// Same major generation. Minor version is the lowest supported minor.
    Compatible { negotiated: Version },
    /// Different major generation. Do not attempt to interpret the stream.
    UnsupportedMajor,
}

/// Negotiate an agreed schema without attempting to parse untrusted input.
///
/// This does not open or authenticate a local socket.
#[must_use]
pub fn negotiate(local: Version, peer: Version) -> Compatibility {
    if local.major != peer.major {
        Compatibility::UnsupportedMajor
    } else {
        Compatibility::Compatible {
            negotiated: Version { major: local.major, minor: local.minor.min(peer.minor) },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compatible_minor_selects_oldest() {
        assert_eq!(
            negotiate(PROTOCOL_VERSION, Version { major: 0, minor: 0 }),
            Compatibility::Compatible { negotiated: Version { major: 0, minor: 0 } }
        );
    }

    #[test]
    fn unknown_major_is_rejected() {
        assert_eq!(
            negotiate(PROTOCOL_VERSION, Version { major: 1, minor: 0 }),
            Compatibility::UnsupportedMajor
        );
    }
}
