//! Bounded, length-prefixed local message framing.
//!
//! A frame consists of a four-byte unsigned **big-endian payload length**
//! followed by exactly that many opaque payload bytes. Empty frames and
//! payloads larger than one MiB are forbidden. No transport, authentication,
//! message schema or socket implementation is provided by this module.
use crate::MAX_FRAME_BYTES;
use std::fmt;

/// Maximum number of untrusted bytes accepted by one decoder call.
///
/// This limits per-call allocations and number of completed messages.
/// Callers can split larger reads into consecutive chunks.
pub const MAX_INPUT_CHUNK_BYTES: usize = 64 * 1024;
const HEADER_BYTES: usize = 4;

/// A malformed or incomplete frame, or a decoder requiring explicit reset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameError {
    /// Zero-length payload is forbidden.
    EmptyPayload,
    /// An advertised or supplied payload exceeds the allowed bound.
    PayloadTooLarge,
    /// Input would bypass bounded per-call resource limits.
    InputChunkTooLarge,
    /// A frame ended before its declared number of bytes was received.
    Truncated,
    /// A prior decoding error invalidated the current stream.
    Poisoned,
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::EmptyPayload => "zero-length IPC payload",
            Self::PayloadTooLarge => "IPC payload exceeds one MiB",
            Self::InputChunkTooLarge => "IPC input chunk exceeds 64 KiB",
            Self::Truncated => "incomplete IPC frame at end of stream",
            Self::Poisoned => "IPC decoder poisoned by earlier invalid frame",
        };
        f.write_str(message)
    }
}

impl std::error::Error for FrameError {}

/// Add a length prefix to one nonempty, bounded payload.
///
/// Encoding accepts opaque bytes only. The caller is responsible for schema
/// validation, authentication, confidentiality, and redaction as applicable.
pub fn encode_frame(payload: &[u8]) -> Result<Vec<u8>, FrameError> {
    if payload.is_empty() {
        return Err(FrameError::EmptyPayload);
    }
    if payload.len() > MAX_FRAME_BYTES {
        return Err(FrameError::PayloadTooLarge);
    }
    // The enforced maximum is smaller than u32::MAX.
    let payload_len = u32::try_from(payload.len()).map_err(|_| FrameError::PayloadTooLarge)?;
    let mut frame = Vec::with_capacity(HEADER_BYTES + payload.len());
    frame.extend_from_slice(&payload_len.to_be_bytes());
    frame.extend_from_slice(payload);
    Ok(frame)
}

/// Stateful streaming frame decoder with a bounded in-progress message.
///
/// The decoder holds at most `MAX_FRAME_BYTES` payload bytes plus a four-byte
/// header; each `push` processes at most `MAX_INPUT_CHUNK_BYTES`. Decoded
/// output is owned by the caller, not retained inside the decoder. An invalid
/// frame poisons the decoder; the caller should close the connection rather
/// than resynchronize untrusted bytes.
#[derive(Debug, Default)]
pub struct FrameDecoder {
    header: [u8; HEADER_BYTES],
    header_read: usize,
    expected: Option<usize>,
    payload: Vec<u8>,
    poisoned: bool,
}

impl FrameDecoder {
    /// Start with an empty decoder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Decode zero or more completed frames from an input chunk.
    ///
    /// Chunks larger than 64 KiB must be split by the caller. On invalid input,
    /// this decoder remains poisoned and rejects subsequent calls.
    pub fn push(&mut self, chunk: &[u8]) -> Result<Vec<Vec<u8>>, FrameError> {
        if self.poisoned {
            return Err(FrameError::Poisoned);
        }
        if chunk.len() > MAX_INPUT_CHUNK_BYTES {
            return self.reject(FrameError::InputChunkTooLarge);
        }

        let mut completed = Vec::new();
        let mut input = chunk;
        while !input.is_empty() {
            if self.expected.is_none() {
                let count = (HEADER_BYTES - self.header_read).min(input.len());
                self.header[self.header_read..self.header_read + count]
                    .copy_from_slice(&input[..count]);
                self.header_read += count;
                input = &input[count..];
                if self.header_read < HEADER_BYTES {
                    continue;
                }
                let advertised = u32::from_be_bytes(self.header) as usize;
                if advertised == 0 {
                    return self.reject(FrameError::EmptyPayload);
                }
                if advertised > MAX_FRAME_BYTES {
                    return self.reject(FrameError::PayloadTooLarge);
                }
                self.expected = Some(advertised);
                self.payload.clear();
            }

            let expected = self.expected.expect("header parsed before payload");
            let take = (expected - self.payload.len()).min(input.len());
            self.payload.extend_from_slice(&input[..take]);
            input = &input[take..];
            if self.payload.len() == expected {
                completed.push(std::mem::take(&mut self.payload));
                self.expected = None;
                self.header_read = 0;
                self.header = [0; HEADER_BYTES];
            }
        }
        Ok(completed)
    }

    /// Check whether no incomplete frame remains at the end of a stream.
    ///
    /// This must be called on EOF to detect partial headers and payloads.
    pub fn finish(&self) -> Result<(), FrameError> {
        if self.poisoned {
            return Err(FrameError::Poisoned);
        }
        if self.header_read != 0 || self.expected.is_some() {
            return Err(FrameError::Truncated);
        }
        Ok(())
    }

    /// Whether the decoder is ready for a fresh frame boundary.
    #[must_use]
    pub fn is_idle(&self) -> bool {
        !self.poisoned && self.header_read == 0 && self.expected.is_none()
    }

    /// Explicitly discard a faulty connection's decoding state.
    ///
    /// Do not reuse this decoder with the same untrusted stream after reset.
    pub fn reset(&mut self) {
        self.header = [0; HEADER_BYTES];
        self.header_read = 0;
        self.expected = None;
        self.payload.clear();
        self.poisoned = false;
    }

    fn reject<T>(&mut self, err: FrameError) -> Result<T, FrameError> {
        self.poisoned = true;
        self.payload.clear();
        Err(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fragmented_header_and_payload_are_reassembled() {
        let frame = encode_frame(b"hello").unwrap();
        let mut decoder = FrameDecoder::new();
        assert!(decoder.push(&frame[..2]).unwrap().is_empty());
        assert_eq!(decoder.finish(), Err(FrameError::Truncated));
        assert!(decoder.push(&frame[2..5]).unwrap().is_empty());
        assert_eq!(decoder.push(&frame[5..]).unwrap(), vec![b"hello".to_vec()]);
        assert!(decoder.is_idle());
        assert_eq!(decoder.finish(), Ok(()));
    }

    #[test]
    fn multiple_frames_in_one_chunk_keep_order() {
        let mut frames = encode_frame(b"one").unwrap();
        frames.extend_from_slice(&encode_frame(b"two").unwrap());
        let mut decoder = FrameDecoder::new();
        assert_eq!(
            decoder.push(&frames).unwrap(),
            vec![b"one".to_vec(), b"two".to_vec()]
        );
        assert_eq!(decoder.finish(), Ok(()));
    }

    #[test]
    fn empty_and_oversized_frames_are_rejected_and_poison_stream() {
        assert_eq!(encode_frame(b""), Err(FrameError::EmptyPayload));
        assert_eq!(
            encode_frame(&vec![0u8; MAX_FRAME_BYTES + 1]),
            Err(FrameError::PayloadTooLarge)
        );
        let mut decoder = FrameDecoder::new();
        assert_eq!(
            decoder.push(&0u32.to_be_bytes()),
            Err(FrameError::EmptyPayload)
        );
        assert_eq!(decoder.push(b"new"), Err(FrameError::Poisoned));
        decoder.reset();
        assert_eq!(
            decoder.push(&((MAX_FRAME_BYTES + 1) as u32).to_be_bytes()),
            Err(FrameError::PayloadTooLarge)
        );
    }

    #[test]
    fn oversized_read_chunk_is_rejected() {
        let mut decoder = FrameDecoder::new();
        assert_eq!(
            decoder.push(&vec![0u8; MAX_INPUT_CHUNK_BYTES + 1]),
            Err(FrameError::InputChunkTooLarge)
        );
        assert_eq!(decoder.finish(), Err(FrameError::Poisoned));
    }

    #[test]
    fn abrupt_eof_inside_body_is_detected() {
        let mut decoder = FrameDecoder::new();
        let frame = encode_frame(b"abcde").unwrap();
        assert!(decoder.push(&frame[..6]).unwrap().is_empty());
        assert_eq!(decoder.finish(), Err(FrameError::Truncated));
    }

    #[test]
    fn maximum_legal_frame_across_small_chunks() {
        let payload = vec![0xabu8; MAX_FRAME_BYTES];
        let frame = encode_frame(&payload).unwrap();
        let mut decoder = FrameDecoder::new();
        let mut seen = Vec::new();
        for chunk in frame.chunks(4096) {
            seen.extend(decoder.push(chunk).unwrap());
        }
        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0], payload);
        assert_eq!(decoder.finish(), Ok(()));
    }

    #[test]
    fn header_and_payload_can_cross_arbitrary_chunks() {
        let mut bytes = encode_frame(b"x").unwrap();
        bytes.extend_from_slice(&encode_frame(b"yz").unwrap());
        let mut decoder = FrameDecoder::new();
        let mut seen = Vec::new();
        for chunk in bytes.chunks(1) {
            seen.extend(decoder.push(chunk).unwrap());
        }
        assert_eq!(seen, vec![b"x".to_vec(), b"yz".to_vec()]);
        assert!(decoder.is_idle());
    }

    #[test]
    fn resetting_uses_new_stream_not_prior_data() {
        let mut decoder = FrameDecoder::new();
        assert!(decoder.push(&[0, 0]).unwrap().is_empty());
        decoder.reset();
        assert_eq!(
            decoder.push(&encode_frame(b"safe").unwrap()).unwrap(),
            vec![b"safe".to_vec()]
        );
        assert_eq!(decoder.finish(), Ok(()));
    }
}
