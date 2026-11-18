//! JSON-lines capture log. One object per line:
//! `{"t_ms":..,"dir":"rx|tx|note|event","hex":"..","frame":{..}|null,"error":..}`.

use knurl_protocol::{Frame, FrameError};
use serde::Serialize;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::time::Instant;

#[derive(Serialize)]
struct FrameJson {
    data_type: u8,
    seq: u8,
    payload: String,
}

#[derive(Serialize)]
struct Line<'a> {
    t_ms: u128,
    dir: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    hex: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    frame: Option<FrameJson>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<&'a str>,
}

pub fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

pub struct CaptureLog {
    out: BufWriter<File>,
    start: Instant,
}

impl CaptureLog {
    pub fn create(path: &Path) -> std::io::Result<Self> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        Ok(Self {
            out: BufWriter::new(File::create(path)?),
            start: Instant::now(),
        })
    }

    fn write(&mut self, line: Line<'_>) {
        // A failing log write must not kill the capture session; report it on stderr.
        let res = serde_json::to_writer(&mut self.out, &line)
            .map_err(std::io::Error::other)
            .and_then(|()| self.out.write_all(b"\n"))
            .and_then(|()| self.out.flush());
        if let Err(e) = res {
            eprintln!("warning: could not write capture log: {e}");
        }
    }

    fn base(&self, dir: &'static str) -> Line<'static> {
        Line {
            t_ms: self.start.elapsed().as_millis(),
            dir,
            hex: None,
            frame: None,
            error: None,
            text: None,
        }
    }

    pub fn frame(&mut self, dir: &'static str, wire: &[u8], frame: &Frame) {
        let mut l = self.base(dir);
        l.hex = Some(hex(wire));
        l.frame = Some(FrameJson {
            data_type: frame.data_type.to_byte(),
            seq: frame.seq,
            payload: hex(&frame.payload),
        });
        self.write(l);
    }

    pub fn invalid(&mut self, wire: &[u8], error: &FrameError) {
        let mut l = self.base("rx");
        l.hex = Some(hex(wire));
        l.error = Some(error.to_string());
        self.write(l);
    }

    pub fn discarded(&mut self, bytes: &[u8]) {
        let mut l = self.base("rx");
        l.hex = Some(hex(bytes));
        l.error = Some("bytes outside any frame".into());
        self.write(l);
    }

    pub fn note(&mut self, text: &str) {
        let mut l = self.base("note");
        l.text = Some(text);
        self.write(l);
    }

    pub fn event(&mut self, text: &str) {
        let mut l = self.base("event");
        l.text = Some(text);
        self.write(l);
    }
}
