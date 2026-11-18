//! Renders every screen of Knurl Probe to PNG files with Slint's software
//! renderer, light and dark, so the layout can be checked without Windows,
//! headphones or a display. The headphone names and report values are
//! SAMPLES typed below, not data read from headphones; this is a
//! development tool, not part of the app.
//!
//! `cargo run -p knurl-probe-app --example snapshot -- <output folder>`

use knurl_probe::CheckStep;
use knurl_probe_app::flow::{Event, Flow};
use knurl_probe_app::ui::{MainWindow, Palette};
use knurl_probe_app::view;
use knurl_report::{Answers, Function, REPORT_FORMAT, Report, ServiceResult, Services, Verdict};
use slint::platform::software_renderer::{MinimalSoftwareWindow, RepaintBufferType};
use slint::platform::{Platform, PlatformError, WindowAdapter};
use slint::{ComponentHandle, PhysicalSize, Rgb8Pixel};
use std::path::{Path, PathBuf};
use std::rc::Rc;

struct Snapshots {
    window: Rc<MinimalSoftwareWindow>,
}

impl Platform for Snapshots {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, PlatformError> {
        Ok(self.window.clone())
    }
}

fn save(window: &MinimalSoftwareWindow, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // Two passes: the first settles layouts that depend on timers.
    let size = window.size();
    let (w, h) = (size.width as usize, size.height as usize);
    let mut pixels = vec![Rgb8Pixel::new(0, 0, 0); w * h];
    for _ in 0..2 {
        window.request_redraw();
        slint::platform::update_timers_and_animations();
        window.draw_if_needed(|renderer| {
            renderer.render(&mut pixels, w);
        });
    }
    let file = std::fs::File::create(path)?;
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), size.width, size.height);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let bytes: Vec<u8> = pixels.iter().flat_map(|p| [p.r, p.g, p.b]).collect();
    encoder.write_header()?.write_image_data(&bytes)?;
    println!("{}", path.display());
    Ok(())
}

/// A SAMPLE report: what a check of a model Knurl does not know could look
/// like. Not read from headphones.
fn sample_report(verdict: Verdict) -> Report {
    let answered = matches!(verdict, Verdict::Supported | Verdict::NewModel);
    let model = if verdict == Verdict::Supported {
        "WH-1000XM3"
    } else {
        "WH-1000XM6"
    };
    Report {
        format: REPORT_FORMAT,
        tool: "knurl-probe 0.1.0".into(),
        date: "2026-11-18".into(),
        os: "Windows 10.0.26100".into(),
        device_name: model.into(),
        services: Services {
            v1: if verdict == Verdict::NotDecoded {
                ServiceResult::NotOffered
            } else {
                ServiceResult::Opened
            },
            v2: if verdict == Verdict::NotDecoded {
                ServiceResult::Opened
            } else {
                ServiceResult::NotOffered
            },
        },
        answers: if answered {
            Answers {
                protocol_version: Some("0x4010".into()),
                model: Some(model.into()),
                firmware: Some("1.2.0".into()),
                functions: Some(
                    ["0x11", "0xC1", "0x12", "0x13", "0x14"]
                        .iter()
                        .map(|c| Function {
                            code: (*c).into(),
                            name: None,
                        })
                        .collect(),
                ),
                battery_percent: Some(80),
                ..Answers::default()
            }
        } else {
            Answers::default()
        },
        verdict,
        frames: Vec::new(),
        problems: if verdict == Verdict::NothingRead {
            vec!["could not open MDR v1: the headphones did not answer (sample)".into()]
        } else {
            Vec::new()
        },
    }
}

fn shot(
    ui: &MainWindow,
    window: &MinimalSoftwareWindow,
    flow: &Flow,
    path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    view::apply(ui, flow);
    save(window, path)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = PathBuf::from(
        std::env::args()
            .nth(1)
            .unwrap_or_else(|| "snapshots".into()),
    );
    std::fs::create_dir_all(&out)?;
    let window = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
    slint::platform::set_platform(Box::new(Snapshots {
        window: window.clone(),
    }))
    .map_err(|e| format!("{e:?}"))?;
    let ui = MainWindow::new()?;
    window.set_size(PhysicalSize::new(440, 660));
    ui.show()?;

    let names = vec!["WH-1000XM6".to_string(), "WH-1000XM3".to_string()];
    for (theme, scheme) in [
        ("light", slint::language::ColorScheme::Light),
        ("dark", slint::language::ColorScheme::Dark),
    ] {
        ui.global::<Palette>().set_color_scheme(scheme);
        let at = |n: &str| out.join(format!("{n}-{theme}.png"));

        let mut f = Flow::default();
        shot(&ui, &window, &f, &at("01-welcome"))?;
        f.handle(Event::Start);
        shot(&ui, &window, &f, &at("02-choose-searching"))?;
        let mut none = f.clone();
        none.handle(Event::DevicesFound(Ok(Vec::new())));
        shot(&ui, &window, &none, &at("03-choose-none"))?;
        let mut failed = f.clone();
        failed.handle(Event::DevicesFound(Err(
            "The Bluetooth radio is off (sample)".into(),
        )));
        shot(&ui, &window, &failed, &at("04-choose-failed"))?;
        f.handle(Event::DevicesFound(Ok(names.clone())));
        shot(&ui, &window, &f, &at("05-choose-found"))?;
        f.handle(Event::Choose(0));
        f.handle(Event::Progress(CheckStep::Opening));
        f.handle(Event::Progress(CheckStep::Asking));
        shot(&ui, &window, &f, &at("06-checking"))?;

        for (n, verdict) in [
            ("07-result-new", Verdict::NewModel),
            ("08-result-supported", Verdict::Supported),
            ("09-result-not-decoded", Verdict::NotDecoded),
            ("10-result-nothing", Verdict::NothingRead),
        ] {
            let mut r = f.clone();
            r.handle(Event::Checked(Box::new(sample_report(verdict))));
            shot(&ui, &window, &r, &at(n))?;
        }

        f.handle(Event::Checked(Box::new(sample_report(Verdict::NewModel))));
        f.handle(Event::ToSend);
        shot(&ui, &window, &f, &at("11-send"))?;
        let mut saved = f.clone();
        saved.handle(Event::Saved(Ok(
            r"C:\Users\sample\Documents\Knurl Probe\knurl-report-WH-1000XM6-1794960000.json".into(),
        )));
        saved.handle(Event::ToggleData);
        shot(&ui, &window, &saved, &at("12-send-saved-data"))?;
        let mut not_saved = f.clone();
        not_saved.handle(Event::Saved(Err(
            "could not write the file: access denied (sample)".into(),
        )));
        shot(&ui, &window, &not_saved, &at("13-send-not-saved"))?;
    }
    Ok(())
}
