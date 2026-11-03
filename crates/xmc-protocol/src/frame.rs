use crate::{END, ESCAPE, ESCAPE_MASK, MAX_PAYLOAD, START};

/// The kind of frame, carried in the first byte after the start marker.
///
/// Only the values below are named; anything else is kept as
/// [`DataType::Other`] so that captures of unknown traffic are never lost.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DataType {
    /// Acknowledgement of a received frame.
    Ack,
    /// A regular MDR command, reply or notification.
    DataMdr,
    /// The second MDR channel used by some firmwares (unverified on XM3/XM4).
    DataMdrNo2,
    /// Any other value, preserved verbatim.
    Other(u8),
}

impl DataType {
    /// Raw byte value on the wire.
    pub const fn to_byte(self) -> u8 {
        match self {
            DataType::Ack => 0x01,
            DataType::DataMdr => 0x0C,
            DataType::DataMdrNo2 => 0x0E,
            DataType::Other(b) => b,
        }
    }

    /// Interprets a raw byte. Never fails: unknown values become `Other`.
    pub const fn from_byte(b: u8) -> Self {
        match b {
            0x01 => DataType::Ack,
            0x0C => DataType::DataMdr,
            0x0E => DataType::DataMdrNo2,
            other => DataType::Other(other),
        }
    }
}

/// One decoded protocol frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub data_type: DataType,
    pub seq: u8,
    pub payload: Vec<u8>,
}

/// Why a byte sequence is not a valid frame.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FrameError {
    #[error("frame does not start with 0x3E")]
    MissingStart,
    #[error("frame does not end with 0x3C")]
    MissingEnd,
    #[error("escape byte 0x3D at the end of the frame body")]
    DanglingEscape,
    #[error("escape byte followed by 0x{0:02X}, which is not a valid escaped value")]
    InvalidEscape(u8),
    #[error("unescaped marker byte 0x{0:02X} inside the frame body")]
    UnescapedMarker(u8),
    #[error("frame body too short: {0} bytes, need at least 7")]
    TooShort(usize),
    #[error("length field says {declared} payload bytes, frame has {actual}")]
    LengthMismatch { declared: usize, actual: usize },
    #[error("payload length {0} exceeds the {MAX_PAYLOAD}-byte limit")]
    PayloadTooLarge(usize),
    #[error("checksum mismatch: frame says 0x{expected:02X}, computed 0x{computed:02X}")]
    BadChecksum { expected: u8, computed: u8 },
}

impl Frame {
    pub fn new(data_type: DataType, seq: u8, payload: Vec<u8>) -> Self {
        Self {
            data_type,
            seq,
            payload,
        }
    }

    /// Checksum over the unescaped header and payload.
    fn checksum(data_type: u8, seq: u8, len: [u8; 4], payload: &[u8]) -> u8 {
        let header = [data_type, seq, len[0], len[1], len[2], len[3]];
        header
            .iter()
            .chain(payload)
            .fold(0u8, |acc, b| acc.wrapping_add(*b))
    }

    /// Encodes the frame to its escaped wire form, markers included.
    ///
    /// # Panics
    /// Never: payloads longer than `u32::MAX` cannot exist on supported targets'
    /// realistic memory, but the length is still checked and saturated to avoid
    /// a silent truncation.
    pub fn encode(&self) -> Vec<u8> {
        let len = u32::try_from(self.payload.len())
            .unwrap_or(u32::MAX)
            .to_be_bytes();
        let dt = self.data_type.to_byte();
        let sum = Self::checksum(dt, self.seq, len, &self.payload);

        let mut out = Vec::with_capacity(self.payload.len() + 10);
        out.push(START);
        let body = [dt, self.seq]
            .into_iter()
            .chain(len)
            .chain(self.payload.iter().copied())
            .chain([sum]);
        for b in body {
            if matches!(b, START | END | ESCAPE) {
                out.push(ESCAPE);
                out.push(b & !ESCAPE_MASK);
            } else {
                out.push(b);
            }
        }
        out.push(END);
        out
    }

    /// Decodes one complete wire frame, markers included.
    pub fn decode(wire: &[u8]) -> Result<Self, FrameError> {
        let (&first, rest) = wire.split_first().ok_or(FrameError::MissingStart)?;
        if first != START {
            return Err(FrameError::MissingStart);
        }
        let (&last, inner) = rest.split_last().ok_or(FrameError::MissingEnd)?;
        if last != END {
            return Err(FrameError::MissingEnd);
        }
        let body = unescape(inner)?;
        Self::from_body(&body)
    }

    /// Parses an unescaped body: `data_type, seq, len[4], payload, checksum`.
    fn from_body(body: &[u8]) -> Result<Self, FrameError> {
        if body.len() < 7 {
            return Err(FrameError::TooShort(body.len()));
        }
        let dt = body[0];
        let seq = body[1];
        let len = [body[2], body[3], body[4], body[5]];
        let declared = u32::from_be_bytes(len) as usize;
        if declared > MAX_PAYLOAD {
            return Err(FrameError::PayloadTooLarge(declared));
        }
        let payload = &body[6..body.len() - 1];
        if payload.len() != declared {
            return Err(FrameError::LengthMismatch {
                declared,
                actual: payload.len(),
            });
        }
        let expected = body[body.len() - 1];
        let computed = Self::checksum(dt, seq, len, payload);
        if expected != computed {
            return Err(FrameError::BadChecksum { expected, computed });
        }
        Ok(Self::new(DataType::from_byte(dt), seq, payload.to_vec()))
    }
}

/// Builds the acknowledgement for a received frame.
///
/// Assumption to verify on hardware: the ACK carries `1 - seq` of the frame it
/// acknowledges and an empty payload, which is what public implementations do
/// for the 1-bit alternating sequence number.
pub fn ack_for(received: &Frame) -> Frame {
    Frame::new(
        DataType::Ack,
        1u8.wrapping_sub(received.seq & 1),
        Vec::new(),
    )
}

fn unescape(inner: &[u8]) -> Result<Vec<u8>, FrameError> {
    let mut out = Vec::with_capacity(inner.len());
    let mut it = inner.iter().copied();
    while let Some(b) = it.next() {
        match b {
            ESCAPE => {
                let next = it.next().ok_or(FrameError::DanglingEscape)?;
                let restored = next | ESCAPE_MASK;
                if !matches!(restored, START | END | ESCAPE) || next == restored {
                    return Err(FrameError::InvalidEscape(next));
                }
                out.push(restored);
            }
            START | END => return Err(FrameError::UnescapedMarker(b)),
            other => out.push(other),
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mdr(seq: u8, payload: &[u8]) -> Frame {
        Frame::new(DataType::DataMdr, seq, payload.to_vec())
    }

    #[test]
    fn encodes_known_layout() {
        // payload [0x00, 0x00]: checksum = 0x0C + 0 + (0,0,0,2) + 0 + 0 = 0x0E
        let wire = mdr(0, &[0x00, 0x00]).encode();
        assert_eq!(
            wire,
            vec![
                0x3E, 0x0C, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x0E, 0x3C
            ]
        );
    }

    #[test]
    fn roundtrips_all_single_byte_payloads() {
        for b in 0..=u8::MAX {
            for seq in [0, 1] {
                let f = mdr(seq, &[b, b, 0x3C, 0x3D, 0x3E]);
                assert_eq!(Frame::decode(&f.encode()), Ok(f));
            }
        }
    }

    #[test]
    fn escapes_marker_bytes_inside_the_body() {
        let wire = mdr(0, &[0x3E, 0x3C, 0x3D]).encode();
        let inner = &wire[1..wire.len() - 1];
        assert!(!inner.contains(&START) && !inner.contains(&END));
        assert!(inner.windows(2).any(|w| w == [0x3D, 0x2E]));
        assert!(inner.windows(2).any(|w| w == [0x3D, 0x2C]));
        assert!(inner.windows(2).any(|w| w == [0x3D, 0x2D]));
    }

    #[test]
    fn rejects_bad_checksum() {
        let mut wire = mdr(1, &[0x10, 0x20]).encode();
        let i = wire.len() - 2;
        wire[i] ^= 0x40; // 0x3F -> 0x7F: corrupts the checksum without creating a marker byte
        assert!(matches!(
            Frame::decode(&wire),
            Err(FrameError::BadChecksum { .. })
        ));
    }

    #[test]
    fn rejects_length_mismatch() {
        // Declares 3 payload bytes but carries 2; checksum computed over what is there.
        let body = [0x0C, 0x00, 0x00, 0x00, 0x00, 0x03, 0xAA, 0xBB];
        let sum = body.iter().fold(0u8, |a, b| a.wrapping_add(*b));
        let mut wire = vec![START];
        wire.extend_from_slice(&body);
        wire.push(sum);
        wire.push(END);
        assert_eq!(
            Frame::decode(&wire),
            Err(FrameError::LengthMismatch {
                declared: 3,
                actual: 2
            })
        );
    }

    #[test]
    fn rejects_oversized_length_field_without_allocating() {
        let wire = [START, 0x0C, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0x00, END];
        assert_eq!(
            Frame::decode(&wire),
            Err(FrameError::PayloadTooLarge(0xFFFF_FFFF))
        );
    }

    #[test]
    fn rejects_structural_errors() {
        assert_eq!(Frame::decode(&[]), Err(FrameError::MissingStart));
        assert_eq!(Frame::decode(&[0x00, END]), Err(FrameError::MissingStart));
        assert_eq!(Frame::decode(&[START]), Err(FrameError::MissingEnd));
        assert_eq!(
            Frame::decode(&[START, 0x0C, 0x00]),
            Err(FrameError::MissingEnd)
        );
        assert_eq!(
            Frame::decode(&[START, 0x0C, ESCAPE, END]),
            Err(FrameError::DanglingEscape)
        );
        assert_eq!(
            Frame::decode(&[START, 0x0C, ESCAPE, 0x41, END]),
            Err(FrameError::InvalidEscape(0x41))
        );
        assert_eq!(
            Frame::decode(&[START, 0x0C, START, END]),
            Err(FrameError::UnescapedMarker(START))
        );
        assert_eq!(
            Frame::decode(&[START, 0x0C, 0x00, END]),
            Err(FrameError::TooShort(2))
        );
    }

    #[test]
    fn ack_flips_the_sequence_bit() {
        assert_eq!(ack_for(&mdr(0, &[])).seq, 1);
        assert_eq!(ack_for(&mdr(1, &[])).seq, 0);
        let ack = ack_for(&mdr(0, &[1, 2, 3]));
        assert_eq!(ack.data_type, DataType::Ack);
        assert!(ack.payload.is_empty());
    }

    #[test]
    fn unknown_data_types_are_preserved() {
        let f = Frame::new(DataType::Other(0x7F), 0, vec![9]);
        let back = Frame::decode(&f.encode()).expect("valid frame");
        assert_eq!(back.data_type, DataType::Other(0x7F));
        assert_eq!(DataType::from_byte(0x7F).to_byte(), 0x7F);
    }
}
