//! Meaning of MDR payloads (protocol V1, "table 1" = [`DataType::DataMdr`]).
//!
//! Read requests plus two kinds of setting change: noise cancelling / ambient
//! sound (`0x68`) and equalizer (`0x58`). The setter layouts come from
//! Gadgetbridge (`SonyProtocolImplV1.java`); the headphones confirm a change
//! with the matching notification (`0x69`, `0x59`), which updates the state
//! like any other. On our XM3 (2026-11-08) both were checked: the noise modes
//! through the app, the equalizer with `xmc-probe` captures.
//!
//! A change can also be answered by a question (`ALERT_NTFY_PARAM`, `0x99`):
//! on our XM3 over aptX, an equalizer change asked to switch the connection
//! mode, and was applied only after the switch to SBC. See [`Message::Alert`].
//!
//! Decoders check every reply byte they rely on. Layouts come from the public
//! sources in `docs/02-riferimenti-protocollo.md`. Each decoder has a test with
//! a frame captured from our own XM3 (`tests/xm3_capture_2026_11_07.rs`) unless
//! its doc comment says otherwise.
//!
//! [`DataType::DataMdr`]: crate::DataType::DataMdr

use thiserror::Error;

/// Opcodes of the requests and replies this module understands.
pub mod opcode {
    pub const GET_PROTOCOL_INFO: u8 = 0x00;
    pub const RET_PROTOCOL_INFO: u8 = 0x01;
    pub const GET_DEVICE_INFO: u8 = 0x04;
    pub const RET_DEVICE_INFO: u8 = 0x05;
    pub const GET_SUPPORT_FUNCTION: u8 = 0x06;
    pub const RET_SUPPORT_FUNCTION: u8 = 0x07;
    pub const GET_BATTERY: u8 = 0x10;
    pub const RET_BATTERY: u8 = 0x11;
    pub const NTFY_BATTERY: u8 = 0x13;
    pub const GET_CODEC: u8 = 0x18;
    pub const RET_CODEC: u8 = 0x19;
    pub const NTFY_CODEC: u8 = 0x1B;
    pub const GET_EQ: u8 = 0x56;
    pub const RET_EQ: u8 = 0x57;
    pub const SET_EQ: u8 = 0x58;
    pub const NTFY_EQ: u8 = 0x59;
    pub const GET_NC_ASM: u8 = 0x66;
    pub const RET_NC_ASM: u8 = 0x67;
    pub const SET_NC_ASM: u8 = 0x68;
    pub const NTFY_NC_ASM: u8 = 0x69;
    pub const ALERT_SET_PARAM: u8 = 0x98;
    pub const ALERT_NTFY_PARAM: u8 = 0x99;
    pub const NTFY_PLAY: u8 = 0xA9;
}

/// Alert message type of [`Message::Alert`]: the change needs the connection
/// mode to change ("DISCONNECT_CAUSED_BY_CONNECTION_MODE_CHANGE" in
/// mos9527/SonyHeadphonesClient). Seen on our XM3 on 2026-11-08 after an
/// equalizer change over aptX.
pub const ALERT_CONNECTION_MODE_CHANGE: u8 = 0x01;

/// Codec code for SBC in codec replies and notifications (`0x19`, `0x1B`),
/// per Gadgetbridge (`AudioCodec`). Our XM3 reported it on 2026-11-08 after
/// switching from aptX (`0x20`).
pub const CODEC_SBC: u8 = 0x01;

/// Which text field `GET_DEVICE_INFO` asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceInfoKind {
    ModelName,
    Firmware,
}

impl DeviceInfoKind {
    const fn to_byte(self) -> u8 {
        match self {
            DeviceInfoKind::ModelName => 0x01,
            DeviceInfoKind::Firmware => 0x02,
        }
    }
}

/// A request to the headphones. [`Request::payload`] gives the bytes to send
/// in a `DATA_MDR` frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Request {
    /// Must be the first request after connecting.
    ProtocolInfo,
    DeviceInfo(DeviceInfoKind),
    SupportedFunctions,
    Battery,
    /// Audio codec in use. Not yet captured on our headphones.
    Codec,
    Equalizer,
    NoiseCancelling,
    /// Change noise cancelling / ambient sound. Checked by the user on our XM3
    /// (2026-11-08); unverified on the XM4.
    SetNcAsm(NcAsmSetting),
    /// Select an equalizer preset (see [`EQ_PRESETS`]). On our XM3
    /// (2026-11-08): applied over SBC; over aptX answered with an alert.
    SetEqPreset(u8),
    /// Set Clear Bass and the five bands, each -10..=10. On our XM3
    /// (2026-11-08, over SBC) applied and reported as preset Manual (`0xA0`).
    SetEqBands {
        clear_bass: i8,
        bands: [i8; 5],
    },
    /// Answer a yes/no [`Message::Alert`]: `98 01 <message> <00 no | 01 yes>`,
    /// layout from mos9527/SonyHeadphonesClient (`SetAlertParamFixedMessageParam`).
    /// The headphones only acknowledge it. On our XM3 (2026-11-08), for
    /// [`ALERT_CONNECTION_MODE_CHANGE`]: "no" left the equalizer unchanged;
    /// "yes" switched the codec from aptX to SBC, also without applying the
    /// change, which then had to be sent again.
    AnswerAlert {
        message: u8,
        yes: bool,
    },
}

/// What [`Request::SetNcAsm`] sends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NcAsmSetting {
    pub mode: NcAsmMode,
    /// The headphones report the three-way "NC / wind / ambient" setting
    /// (byte 3 = `0x02`, as our XM3 does) rather than NC on/off (`0x00`).
    pub wind_capable: bool,
    /// Ambient level sent with [`NcAsmMode::Off`], so turning everything off
    /// keeps the last ambient level (Gadgetbridge does the same).
    pub level_when_off: u8,
}

impl NcAsmSetting {
    /// `68 02 <effect> <group> <mode> 01 <voice> <level>`, per Gadgetbridge.
    fn payload(self) -> Vec<u8> {
        let three_way = self.wind_capable;
        let (effect, nc, voice, level) = match self.mode {
            NcAsmMode::Off => (0x00, 0x00, false, self.level_when_off),
            NcAsmMode::NoiseCancelling => (0x11, if three_way { 2 } else { 1 }, false, 0),
            // Without the three-way setting there is no wind mode; Gadgetbridge
            // then sends NC off, which is ambient.
            NcAsmMode::WindReduction => (0x11, u8::from(three_way), false, 0),
            NcAsmMode::Ambient {
                level,
                focus_on_voice,
            } => (0x11, 0x00, focus_on_voice, level),
        };
        let group = if three_way { 0x02 } else { 0x00 };
        vec![
            opcode::SET_NC_ASM,
            0x02,
            effect,
            group,
            nc,
            0x01,
            u8::from(voice),
            level.min(20),
        ]
    }
}

/// Equalizer presets with their codes, per Gadgetbridge (`EqualizerPreset`).
/// Unverified on our headphones, except `0x00` (off) seen in our XM3 capture.
pub const EQ_PRESETS: [(u8, &str); 12] = [
    (0x00, "Off"),
    (0x10, "Bright"),
    (0x11, "Excited"),
    (0x12, "Mellow"),
    (0x13, "Relaxed"),
    (0x14, "Vocal"),
    (0x15, "Treble Boost"),
    (0x16, "Bass Boost"),
    (0x17, "Speech"),
    (0xA0, "Manual"),
    (0xA1, "Custom 1"),
    (0xA2, "Custom 2"),
];

/// Name of an equalizer preset code, if known.
pub fn eq_preset_name(code: u8) -> Option<&'static str> {
    EQ_PRESETS.iter().find(|(c, _)| *c == code).map(|(_, n)| *n)
}

/// -10..=10 to the wire's 0..=20.
fn offset10(v: i8) -> u8 {
    (v.clamp(-10, 10) + 10) as u8
}

impl Request {
    pub fn payload(self) -> Vec<u8> {
        use opcode::*;
        match self {
            Request::ProtocolInfo => vec![GET_PROTOCOL_INFO, 0x00],
            Request::DeviceInfo(kind) => vec![GET_DEVICE_INFO, kind.to_byte()],
            Request::SupportedFunctions => vec![GET_SUPPORT_FUNCTION, 0x00],
            Request::Battery => vec![GET_BATTERY, 0x00],
            Request::Codec => vec![GET_CODEC, 0x00],
            Request::Equalizer => vec![GET_EQ, 0x01],
            Request::NoiseCancelling => vec![GET_NC_ASM, 0x02],
            Request::SetNcAsm(setting) => setting.payload(),
            Request::SetEqPreset(code) => vec![SET_EQ, 0x01, code, 0x00],
            Request::SetEqBands { clear_bass, bands } => {
                // Gadgetbridge sends 0xFF in the preset slot for custom bands.
                let mut p = vec![SET_EQ, 0x01, 0xFF, 0x06, offset10(clear_bass)];
                p.extend(bands.iter().map(|&b| offset10(b)));
                p
            }
            Request::AnswerAlert { message, yes } => {
                vec![ALERT_SET_PARAM, 0x01, message, u8::from(yes)]
            }
        }
    }

    /// False for requests the headphones only acknowledge.
    pub fn expects_reply(self) -> bool {
        !matches!(self, Request::AnswerAlert { .. })
    }

    /// True for requests that change a setting. Only the latest change of a
    /// kind is worth sending, so queued ones may be replaced.
    pub fn is_setting(self) -> bool {
        matches!(
            self,
            Request::SetNcAsm(_) | Request::SetEqPreset(_) | Request::SetEqBands { .. }
        )
    }

    /// Opcode of the reply that answers this request (request opcode + 1).
    /// For a setting change this is the matching notification (`0x69`,
    /// `0x59`), which is how the headphones confirm it.
    pub fn reply_opcode(self) -> u8 {
        self.payload()[0].wrapping_add(1)
    }
}

/// Battery charging state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Charging {
    No,
    Yes,
    /// `0xF0` in the sources.
    Unknown,
}

/// Noise cancelling / ambient sound state, as the headphones report it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NcAsmMode {
    /// Everything off.
    Off,
    NoiseCancelling,
    /// "Wind noise reduction" (XM3 per the sources; unverified on XM4).
    WindReduction,
    Ambient {
        /// 0..=20.
        level: u8,
        focus_on_voice: bool,
    },
}

/// Decoded payload of a frame sent by the headphones.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    ProtocolInfo {
        version: u16,
    },
    ModelName(String),
    Firmware(String),
    /// Function codes the headphones declare (see [`function_name`]).
    SupportedFunctions(Vec<u8>),
    Battery {
        level: u8,
        charging: Charging,
        /// `true` for a notification (`0x13`), `false` for a reply (`0x11`).
        notification: bool,
    },
    /// Unverified on our headphones.
    Codec {
        codec: u8,
        notification: bool,
    },
    Equalizer {
        /// Preset id (`0x00` = off, see the references for the others).
        preset: u8,
        /// -10..=10.
        clear_bass: i8,
        /// 400, 1000, 2500, 6300, 16000 Hz; -10..=10 each.
        bands: [i8; 5],
        notification: bool,
    },
    NcAsm {
        mode: NcAsmMode,
        notification: bool,
    },
    /// The headphones ask something about the last change instead of
    /// confirming it: `99 01 <message> <00 confirmation only | 01 yes/no>`,
    /// layout from mos9527/SonyHeadphonesClient (`AlertMessageType`,
    /// `AlertActionType`). Our XM3 sent `99 01 01 01` on 2026-11-08.
    Alert {
        /// Message type, e.g. [`ALERT_CONNECTION_MODE_CHANGE`].
        message: u8,
        /// True when a yes/no answer ([`Request::AnswerAlert`]) is expected.
        asks: bool,
    },
    /// Volume notification (0..=30): `A9 01 20 vv`, sent before an NC/ASM
    /// notification when the NC button is pressed, per the sources.
    /// Unverified on our headphones. Other `A9` layouts are not decoded: our
    /// XM3 sent a 3-byte `A9` payload on 2026-11-07 (bytes not recorded).
    Volume(u8),
    /// A payload this module does not decode. Kept verbatim.
    Unknown(Vec<u8>),
}

/// Why a payload with a known opcode could not be decoded.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum MessageError {
    #[error("empty payload")]
    Empty,
    #[error("opcode 0x{opcode:02X}: expected {expected} bytes, got {actual}")]
    Length {
        opcode: u8,
        expected: usize,
        actual: usize,
    },
    #[error("opcode 0x{opcode:02X}: unexpected byte 0x{value:02X} at offset {offset}")]
    Field {
        opcode: u8,
        offset: usize,
        value: u8,
    },
    #[error("opcode 0x{0:02X}: text is not valid UTF-8")]
    Text(u8),
}

fn need(p: &[u8], expected: usize) -> Result<(), MessageError> {
    if p.len() == expected {
        Ok(())
    } else {
        Err(MessageError::Length {
            opcode: p[0],
            expected,
            actual: p.len(),
        })
    }
}

fn expect(p: &[u8], offset: usize, value: u8) -> Result<(), MessageError> {
    if p[offset] == value {
        Ok(())
    } else {
        Err(MessageError::Field {
            opcode: p[0],
            offset,
            value: p[offset],
        })
    }
}

/// A value stored with a +10 offset (0..=20 on the wire, -10..=10 in reality).
fn centred(p: &[u8], offset: usize) -> Result<i8, MessageError> {
    let v = p[offset];
    if v > 20 {
        return Err(MessageError::Field {
            opcode: p[0],
            offset,
            value: v,
        });
    }
    Ok(v as i8 - 10)
}

impl Message {
    /// Decodes the payload of a `DATA_MDR` frame. Unknown opcodes give
    /// [`Message::Unknown`]; known opcodes with an unexpected layout give an
    /// error instead of a guess.
    pub fn decode(p: &[u8]) -> Result<Message, MessageError> {
        use opcode::*;
        let Some(&op) = p.first() else {
            return Err(MessageError::Empty);
        };
        match op {
            RET_PROTOCOL_INFO => {
                need(p, 4)?;
                expect(p, 1, 0x00)?;
                Ok(Message::ProtocolInfo {
                    version: u16::from_be_bytes([p[2], p[3]]),
                })
            }
            RET_DEVICE_INFO => {
                if p.len() < 3 {
                    return Err(MessageError::Length {
                        opcode: op,
                        expected: 3,
                        actual: p.len(),
                    });
                }
                need(p, 3 + p[2] as usize)?;
                let text = std::str::from_utf8(&p[3..])
                    .map_err(|_| MessageError::Text(op))?
                    .to_owned();
                match p[1] {
                    0x01 => Ok(Message::ModelName(text)),
                    0x02 => Ok(Message::Firmware(text)),
                    _ => Ok(Message::Unknown(p.to_vec())),
                }
            }
            RET_SUPPORT_FUNCTION => {
                if p.len() < 3 {
                    return Err(MessageError::Length {
                        opcode: op,
                        expected: 3,
                        actual: p.len(),
                    });
                }
                expect(p, 1, 0x00)?;
                need(p, 3 + p[2] as usize)?;
                Ok(Message::SupportedFunctions(p[3..].to_vec()))
            }
            RET_BATTERY | NTFY_BATTERY => {
                need(p, 4)?;
                expect(p, 1, 0x00)?;
                let level = p[2];
                if level > 100 {
                    return Err(MessageError::Field {
                        opcode: op,
                        offset: 2,
                        value: level,
                    });
                }
                let charging = match p[3] {
                    0x00 => Charging::No,
                    0x01 => Charging::Yes,
                    0xF0 => Charging::Unknown,
                    value => {
                        return Err(MessageError::Field {
                            opcode: op,
                            offset: 3,
                            value,
                        });
                    }
                };
                Ok(Message::Battery {
                    level,
                    charging,
                    notification: op == NTFY_BATTERY,
                })
            }
            RET_CODEC | NTFY_CODEC => {
                need(p, 3)?;
                expect(p, 1, 0x00)?;
                Ok(Message::Codec {
                    codec: p[2],
                    notification: op == NTFY_CODEC,
                })
            }
            RET_EQ | NTFY_EQ => {
                need(p, 10)?;
                expect(p, 1, 0x01)?;
                expect(p, 3, 0x06)?;
                let mut bands = [0i8; 5];
                for (i, band) in bands.iter_mut().enumerate() {
                    *band = centred(p, 5 + i)?;
                }
                Ok(Message::Equalizer {
                    preset: p[2],
                    clear_bass: centred(p, 4)?,
                    bands,
                    notification: op == NTFY_EQ,
                })
            }
            RET_NC_ASM | NTFY_NC_ASM => {
                need(p, 8)?;
                expect(p, 1, 0x02)?;
                Ok(Message::NcAsm {
                    mode: decode_nc_asm(p)?,
                    notification: op == NTFY_NC_ASM,
                })
            }
            ALERT_NTFY_PARAM if p.get(1) == Some(&0x01) => {
                need(p, 4)?;
                let asks = match p[3] {
                    0x00 => false,
                    0x01 => true,
                    value => {
                        return Err(MessageError::Field {
                            opcode: op,
                            offset: 3,
                            value,
                        });
                    }
                };
                Ok(Message::Alert {
                    message: p[2],
                    asks,
                })
            }
            // 0xA9 carries several kinds of playback notification; only the
            // volume layout is known. Anything else is kept, not rejected.
            NTFY_PLAY if p.get(1..3) == Some(&[0x01, 0x20]) => {
                need(p, 4)?;
                Ok(Message::Volume(p[3]))
            }
            _ => Ok(Message::Unknown(p.to_vec())),
        }
    }
}

/// Bytes 2..8 of an NC/ASM payload: effect, NC setting type, NC value,
/// ambient setting type, ambient id (voice focus), ambient level.
fn decode_nc_asm(p: &[u8]) -> Result<NcAsmMode, MessageError> {
    let level = p[7];
    if level > 20 {
        return Err(MessageError::Field {
            opcode: p[0],
            offset: 7,
            value: level,
        });
    }
    let focus_on_voice = match p[6] {
        0x00 => false,
        0x01 => true,
        value => {
            return Err(MessageError::Field {
                opcode: p[0],
                offset: 6,
                value,
            });
        }
    };
    match p[2] {
        // Effect off: neither NC nor ambient is active.
        0x00 => Ok(NcAsmMode::Off),
        // Effect on (0x01), or adjustment in progress / completed (0x10, 0x11).
        0x01 | 0x10 | 0x11 => match (p[3], p[4]) {
            // "Dual/single/off" setting: 2 = NC, 1 = wind reduction, 0 = ambient.
            (0x02, 0x02) => Ok(NcAsmMode::NoiseCancelling),
            (0x02, 0x01) => Ok(NcAsmMode::WindReduction),
            (0x02, 0x00) => Ok(NcAsmMode::Ambient {
                level,
                focus_on_voice,
            }),
            // "On/off" setting: 1 = NC, 0 = ambient.
            (0x00, 0x01) => Ok(NcAsmMode::NoiseCancelling),
            (0x00, 0x00) => Ok(NcAsmMode::Ambient {
                level,
                focus_on_voice,
            }),
            (0x00 | 0x02, value) => Err(MessageError::Field {
                opcode: p[0],
                offset: 4,
                value,
            }),
            (value, _) => Err(MessageError::Field {
                opcode: p[0],
                offset: 3,
                value,
            }),
        },
        value => Err(MessageError::Field {
            opcode: p[0],
            offset: 2,
            value,
        }),
    }
}

/// Name of a function code from `SupportedFunctions`, per the public sources
/// (mos9527/SonyHeadphonesClient, `docs/device-support/WH-1000XM3.md`). The
/// code-to-name pairing is not verified on our headphones.
pub fn function_name(code: u8) -> Option<&'static str> {
    Some(match code {
        0x11 => "battery level",
        0xC1 => "action log notifier",
        0x12 => "upscaling indicator",
        0x13 => "codec indicator",
        0x14 => "BLE setup",
        0x30 => "firmware update",
        0x71 => "auto NC/ASM",
        0x62 => "noise cancelling and ambient sound",
        0x81 => "NC optimizer",
        0x42 => "sound position",
        0x41 => "VPT (surround)",
        0x51 => "preset equalizer",
        0xA1 => "playback controller",
        0xE1 => "connection mode",
        0xE2 => "upscaling",
        0xD1 => "general setting 1",
        0xD2 => "general setting 2",
        0xF4 => "auto power off",
        0x39 => "voice guidance",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_payloads() {
        assert_eq!(Request::ProtocolInfo.payload(), vec![0x00, 0x00]);
        assert_eq!(
            Request::DeviceInfo(DeviceInfoKind::Firmware).payload(),
            vec![0x04, 0x02]
        );
        assert_eq!(Request::NoiseCancelling.payload(), vec![0x66, 0x02]);
        assert_eq!(Request::Equalizer.reply_opcode(), 0x57);
    }

    #[test]
    fn nc_asm_set_payloads_match_gadgetbridge() {
        let set = |mode, wind_capable| {
            Request::SetNcAsm(NcAsmSetting {
                mode,
                wind_capable,
                level_when_off: 20,
            })
            .payload()
        };
        // Our XM3 reports the three-way setting (byte 3 = 0x02).
        assert_eq!(
            set(NcAsmMode::NoiseCancelling, true),
            [0x68, 0x02, 0x11, 0x02, 0x02, 0x01, 0x00, 0x00]
        );
        assert_eq!(
            set(NcAsmMode::WindReduction, true),
            [0x68, 0x02, 0x11, 0x02, 0x01, 0x01, 0x00, 0x00]
        );
        let ambient = |level, focus_on_voice| NcAsmMode::Ambient {
            level,
            focus_on_voice,
        };
        assert_eq!(
            set(ambient(12, true), true),
            [0x68, 0x02, 0x11, 0x02, 0x00, 0x01, 0x01, 0x0C]
        );
        assert_eq!(
            set(NcAsmMode::Off, true),
            [0x68, 0x02, 0x00, 0x02, 0x00, 0x01, 0x00, 0x14]
        );
        assert_eq!(
            set(NcAsmMode::NoiseCancelling, false),
            [0x68, 0x02, 0x11, 0x00, 0x01, 0x01, 0x00, 0x00]
        );
        assert_eq!(set(ambient(99, false), true)[7], 20);
    }

    #[test]
    fn equalizer_set_payloads_match_gadgetbridge() {
        assert_eq!(
            Request::SetEqPreset(0x16).payload(),
            [0x58, 0x01, 0x16, 0x00]
        );
        assert_eq!(
            Request::SetEqBands {
                clear_bass: 3,
                bands: [-10, -1, 0, 1, 10]
            }
            .payload(),
            [0x58, 0x01, 0xFF, 0x06, 13, 0, 9, 10, 11, 20]
        );
        assert_eq!(Request::SetEqPreset(0x00).reply_opcode(), 0x59);
        assert!(Request::SetEqPreset(0x00).is_setting());
        assert!(!Request::Equalizer.is_setting());
        assert_eq!(eq_preset_name(0xA0), Some("Manual"));
        assert_eq!(eq_preset_name(0x99), None);
    }

    #[test]
    fn alert_and_answer_match_our_xm3_capture() {
        // 2026-11-08, XM3 over aptX, right after `58 01 16 00`.
        assert_eq!(
            Message::decode(&[0x99, 0x01, 0x01, 0x01]),
            Ok(Message::Alert {
                message: ALERT_CONNECTION_MODE_CHANGE,
                asks: true
            })
        );
        assert_eq!(
            Message::decode(&[0x99, 0x01, 0x01, 0x00]),
            Ok(Message::Alert {
                message: 0x01,
                asks: false
            })
        );
        assert!(Message::decode(&[0x99, 0x01, 0x01, 0x07]).is_err());
        assert!(Message::decode(&[0x99, 0x01, 0x01]).is_err());
        // Other alert kinds (e.g. vibrator) are kept, not guessed.
        assert_eq!(
            Message::decode(&[0x99, 0x02, 0x01]),
            Ok(Message::Unknown(vec![0x99, 0x02, 0x01]))
        );
        let yes = Request::AnswerAlert {
            message: 0x01,
            yes: true,
        };
        assert_eq!(yes.payload(), [0x98, 0x01, 0x01, 0x01]);
        assert_eq!(
            Request::AnswerAlert {
                message: 0x01,
                yes: false
            }
            .payload(),
            [0x98, 0x01, 0x01, 0x00]
        );
        assert!(!yes.expects_reply());
        assert!(!yes.is_setting());
        assert!(Request::SetEqPreset(0x16).expects_reply());
    }

    #[test]
    fn eq_replies_from_our_xm3_over_sbc() {
        // 2026-11-08: Bass Boost, then custom bands reported as Manual.
        assert_eq!(
            Message::decode(&[0x59, 0x01, 0x16, 0x06, 0x11, 0x0A, 0x0A, 0x0A, 0x0A, 0x0A]),
            Ok(Message::Equalizer {
                preset: 0x16,
                clear_bass: 7,
                bands: [0; 5],
                notification: true
            })
        );
        assert_eq!(
            Message::decode(&[0x59, 0x01, 0xA0, 0x06, 0x0D, 0x0A, 0x09, 0x0A, 0x0B, 0x0C]),
            Ok(Message::Equalizer {
                preset: 0xA0,
                clear_bass: 3,
                bands: [0, -1, 0, 1, 2],
                notification: true
            })
        );
        assert_eq!(
            Message::decode(&[0x1B, 0x00, 0x01]),
            Ok(Message::Codec {
                codec: CODEC_SBC,
                notification: true
            })
        );
    }

    #[test]
    fn unknown_opcode_is_kept() {
        assert_eq!(
            Message::decode(&[0xEE, 0x01]),
            Ok(Message::Unknown(vec![0xEE, 0x01]))
        );
    }

    #[test]
    fn rejects_bad_lengths_and_values() {
        assert!(matches!(
            Message::decode(&[0x11, 0x00, 0x64]),
            Err(MessageError::Length { .. })
        ));
        assert!(matches!(
            Message::decode(&[0x11, 0x00, 0x65, 0x00]),
            Err(MessageError::Field { offset: 2, .. })
        ));
        assert!(matches!(
            Message::decode(&[0x67, 0x02, 0x01, 0x02, 0x00, 0x01, 0x00, 21]),
            Err(MessageError::Field { offset: 7, .. })
        ));
        assert!(matches!(
            Message::decode(&[0x57, 0x01, 0x00, 0x06, 0x0A, 0x0A, 0x0A, 0x0A, 0x0A, 0x15]),
            Err(MessageError::Field { offset: 9, .. })
        ));
        assert_eq!(Message::decode(&[]), Err(MessageError::Empty));
        assert!(matches!(
            Message::decode(&[0xA9, 0x01, 0x20]),
            Err(MessageError::Length { .. })
        ));
    }

    #[test]
    fn other_playback_notifications_are_kept_not_rejected() {
        assert_eq!(
            Message::decode(&[0xA9, 0x01, 0x20, 0x0F]),
            Ok(Message::Volume(15))
        );
        // Our XM3 sent a 3-byte 0xA9 payload; its bytes were not recorded.
        for p in [&[0xA9, 0x01, 0x00][..], &[0xA9, 0x03, 0x01], &[0xA9]] {
            assert_eq!(Message::decode(p), Ok(Message::Unknown(p.to_vec())));
        }
    }

    #[test]
    fn nc_asm_off_wind_and_on_off_variants() {
        // Vectors from the references (Gadgetbridge tests, setter layout).
        let off = Message::decode(&[0x67, 0x02, 0x00, 0x02, 0x00, 0x01, 0x00, 0x00]);
        assert!(matches!(
            off,
            Ok(Message::NcAsm {
                mode: NcAsmMode::Off,
                ..
            })
        ));
        let wind = Message::decode(&[0x67, 0x02, 0x11, 0x02, 0x01, 0x01, 0x00, 0x00]);
        assert!(matches!(
            wind,
            Ok(Message::NcAsm {
                mode: NcAsmMode::WindReduction,
                ..
            })
        ));
        let voice = Message::decode(&[0x67, 0x02, 0x11, 0x02, 0x00, 0x01, 0x01, 0x0F]);
        assert_eq!(
            voice,
            Ok(Message::NcAsm {
                mode: NcAsmMode::Ambient {
                    level: 15,
                    focus_on_voice: true
                },
                notification: false
            })
        );
    }

    #[test]
    fn every_xm3_function_code_has_a_name() {
        for code in [
            0x11, 0xC1, 0x12, 0x13, 0x14, 0x30, 0x71, 0x62, 0x81, 0x42, 0x41, 0x51, 0xA1, 0xE1,
            0xE2, 0xD1, 0xD2, 0xF4, 0x39,
        ] {
            assert!(function_name(code).is_some(), "0x{code:02X}");
        }
        assert_eq!(function_name(0x00), None);
    }
}
