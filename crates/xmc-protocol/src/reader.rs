use crate::{END, Frame, FrameError, MAX_PAYLOAD, START};

/// Bound on buffered bytes while waiting for an end marker. A valid escaped
/// frame is at most `2 * (MAX_PAYLOAD + 7) + 2` bytes long.
const MAX_BUFFER: usize = 2 * (MAX_PAYLOAD + 7) + 2;

/// Reassembles frames from an arbitrary stream of bytes (RFCOMM reads can split
/// or merge frames).
///
/// Bytes before a start marker are dropped. A frame that fails to decode is
/// reported as an error and skipped, so one corrupted frame never blocks the
/// ones after it.
#[derive(Debug, Default)]
pub struct FrameReader {
    buf: Vec<u8>,
    in_frame: bool,
}

/// One item produced by [`FrameReader::push`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadEvent {
    /// A valid frame, plus its raw wire bytes for logging.
    Frame { frame: Frame, wire: Vec<u8> },
    /// A complete but invalid frame, with its raw bytes and the reason.
    Invalid { wire: Vec<u8>, error: FrameError },
    /// Bytes discarded because they were outside any frame (or overflowed).
    Discarded(Vec<u8>),
}

impl FrameReader {
    pub fn new() -> Self {
        Self::default()
    }

    /// Feeds bytes and returns every event they complete, in order.
    pub fn push(&mut self, bytes: &[u8]) -> Vec<ReadEvent> {
        let mut events = Vec::new();
        let mut junk = Vec::new();
        for &b in bytes {
            if !self.in_frame {
                if b == START {
                    if !junk.is_empty() {
                        events.push(ReadEvent::Discarded(std::mem::take(&mut junk)));
                    }
                    self.in_frame = true;
                    self.buf.clear();
                    self.buf.push(b);
                } else {
                    junk.push(b);
                }
                continue;
            }
            if b == START {
                // A new start before an end: the previous frame was truncated.
                events.push(ReadEvent::Discarded(std::mem::take(&mut self.buf)));
                self.buf.push(b);
                continue;
            }
            self.buf.push(b);
            if b == END {
                let wire = std::mem::take(&mut self.buf);
                self.in_frame = false;
                events.push(match Frame::decode(&wire) {
                    Ok(frame) => ReadEvent::Frame { frame, wire },
                    Err(error) => ReadEvent::Invalid { wire, error },
                });
            } else if self.buf.len() > MAX_BUFFER {
                events.push(ReadEvent::Discarded(std::mem::take(&mut self.buf)));
                self.in_frame = false;
            }
        }
        if !junk.is_empty() {
            events.push(ReadEvent::Discarded(junk));
        }
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DataType;

    fn frame(seq: u8, payload: &[u8]) -> Frame {
        Frame::new(DataType::DataMdr, seq, payload.to_vec())
    }

    fn frames(events: &[ReadEvent]) -> Vec<Frame> {
        events
            .iter()
            .filter_map(|e| match e {
                ReadEvent::Frame { frame, .. } => Some(frame.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn reassembles_a_frame_split_byte_by_byte() {
        let f = frame(1, &[0x3E, 0x01, 0x02]);
        let mut r = FrameReader::new();
        let mut events = Vec::new();
        for b in f.encode() {
            events.extend(r.push(&[b]));
        }
        assert_eq!(frames(&events), vec![f]);
    }

    #[test]
    fn splits_merged_frames_and_drops_leading_junk() {
        let a = frame(0, &[1]);
        let b = frame(1, &[2, 3]);
        let mut stream = vec![0xFF, 0x00];
        stream.extend(a.encode());
        stream.extend(b.encode());
        let events = FrameReader::new().push(&stream);
        assert_eq!(events[0], ReadEvent::Discarded(vec![0xFF, 0x00]));
        assert_eq!(frames(&events), vec![a, b]);
    }

    #[test]
    fn a_corrupted_frame_does_not_block_the_next_one() {
        let mut bad = frame(0, &[5, 6]).encode();
        let i = bad.len() - 2;
        bad[i] ^= 0xFF;
        let good = frame(1, &[7]);
        let mut stream = bad.clone();
        stream.extend(good.encode());
        let events = FrameReader::new().push(&stream);
        assert!(matches!(&events[0], ReadEvent::Invalid { wire, .. } if *wire == bad));
        assert_eq!(frames(&events), vec![good]);
    }

    #[test]
    fn truncated_frame_is_discarded_when_a_new_one_starts() {
        let good = frame(0, &[9]);
        let mut stream = vec![START, 0x0C, 0x00];
        stream.extend(good.encode());
        let events = FrameReader::new().push(&stream);
        assert_eq!(events[0], ReadEvent::Discarded(vec![START, 0x0C, 0x00]));
        assert_eq!(frames(&events), vec![good]);
    }

    #[test]
    fn overlong_garbage_is_bounded() {
        let mut stream = vec![START];
        stream.extend(std::iter::repeat_n(0x11, MAX_BUFFER + 10));
        let mut r = FrameReader::new();
        let events = r.push(&stream);
        assert!(events.iter().any(|e| matches!(e, ReadEvent::Discarded(_))));
        assert!(r.buf.len() <= MAX_BUFFER);
    }
}
