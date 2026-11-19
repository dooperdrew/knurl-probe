//! Compiles the Slint screens in `ui/` (design: design/README.md), with the
//! translations of their `@tr` texts (`lang/<code>/LC_MESSAGES/
//! knurl-probe-app.po`) bundled into the exe: no catalogue files next to it,
//! and the language is picked at run time with
//! `slint::select_bundled_translation`.
//!
//! On Windows it also embeds the icon and the version information in
//! `knurl-probe.exe`, as the Knurl app does. The icon is rendered here from
//! the design SVGs (design/icons), so the .ico never drifts from the design:
//! `app-icon-small.svg` for 32 px and below, `app-icon.svg` above.
//! `scripts/package.py` checks the version information in the released exe.

use std::path::{Path, PathBuf};

use resvg::{tiny_skia, usvg};

const SMALL_SIZES: [u32; 4] = [16, 20, 24, 32];
const LARGE_SIZES: [u32; 5] = [40, 48, 64, 128, 256];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=lang");
    slint_build::compile_with_config(
        "ui/app.slint",
        slint_build::CompilerConfiguration::new().with_bundled_translations("lang"),
    )?;

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let icons = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../design/icons");
        let small = icons.join("app-icon-small.svg");
        let large = icons.join("app-icon.svg");
        println!("cargo:rerun-if-changed={}", small.display());
        println!("cargo:rerun-if-changed={}", large.display());

        let mut images = Vec::new();
        for size in SMALL_SIZES {
            images.push((size, png_of(&small, size)?));
        }
        for size in LARGE_SIZES {
            images.push((size, png_of(&large, size)?));
        }

        let out = PathBuf::from(std::env::var("OUT_DIR")?);
        let ico = out.join("knurl-probe.ico");
        std::fs::write(&ico, ico_file(&images))?;
        let rc = out.join("knurl-probe.rc");
        std::fs::write(&rc, resource_script(&ico, env!("CARGO_PKG_VERSION"))?)?;
        embed_resource::compile(&rc, embed_resource::NONE).manifest_optional()?;
    }
    Ok(())
}

/// The SVG rendered at `size` px, as a PNG.
fn png_of(svg: &Path, size: u32) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let tree = usvg::Tree::from_data(&std::fs::read(svg)?, &usvg::Options::default())?;
    let mut pixmap = tiny_skia::Pixmap::new(size, size).ok_or("empty icon size")?;
    let scale = size as f32 / tree.size().width();
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    let rgba: Vec<u8> = pixmap
        .pixels()
        .iter()
        .flat_map(|p| {
            let c = p.demultiply();
            [c.red(), c.green(), c.blue(), c.alpha()]
        })
        .collect();

    let mut png = Vec::new();
    let mut encoder = png::Encoder::new(&mut png, size, size);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header()?.write_image_data(&rgba)?;
    Ok(png)
}

/// An .ico file with one PNG image per size (supported since Windows Vista).
fn ico_file(images: &[(u32, Vec<u8>)]) -> Vec<u8> {
    let mut ico = Vec::new();
    ico.extend_from_slice(&[0, 0, 1, 0]); // reserved, type 1 = icon
    ico.extend_from_slice(&(images.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * images.len() as u32;
    for (size, png) in images {
        // Width and height are one byte each; 0 means 256.
        let side = if *size >= 256 { 0 } else { *size as u8 };
        ico.extend_from_slice(&[side, side, 0, 0]); // no palette, reserved
        ico.extend_from_slice(&1u16.to_le_bytes()); // planes
        ico.extend_from_slice(&32u16.to_le_bytes()); // bits per pixel
        ico.extend_from_slice(&(png.len() as u32).to_le_bytes());
        ico.extend_from_slice(&offset.to_le_bytes());
        offset += png.len() as u32;
    }
    for (_, png) in images {
        ico.extend_from_slice(png);
    }
    ico
}

/// Resource script with the icon (first icon = the exe's icon in Explorer)
/// and the version shown in the file's properties.
fn resource_script(ico: &Path, version: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut parts = version.split('.').map(|p| p.parse::<u16>());
    let mut next = || parts.next().unwrap_or(Ok(0));
    let (major, minor, patch) = (next()?, next()?, next()?);
    let ico = ico.display().to_string().replace('\\', "\\\\");
    Ok(format!(
        r#"1 ICON "{ico}"
1 VERSIONINFO
FILEVERSION {major},{minor},{patch},0
PRODUCTVERSION {major},{minor},{patch},0
FILEOS 0x40004
FILETYPE 0x1
BEGIN
  BLOCK "StringFileInfo"
  BEGIN
    BLOCK "040904B0"
    BEGIN
      VALUE "CompanyName", "DooperDrew"
      VALUE "FileDescription", "Knurl Probe"
      VALUE "FileVersion", "{version}"
      VALUE "InternalName", "knurl-probe"
      VALUE "LegalCopyright", "Copyright (c) 2026 DooperDrew. MIT OR Apache-2.0."
      VALUE "OriginalFilename", "knurl-probe.exe"
      VALUE "ProductName", "Knurl Probe"
      VALUE "ProductVersion", "{version}"
    END
  END
  BLOCK "VarFileInfo"
  BEGIN
    VALUE "Translation", 0x409, 1200
  END
END
"#
    ))
}
