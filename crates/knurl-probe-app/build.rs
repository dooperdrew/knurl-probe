//! Compiles the Slint screens in `ui/` (design: design/README.md), with the
//! translations of their `@tr` texts (`lang/<code>/LC_MESSAGES/
//! knurl-probe-app.po`) bundled into the exe: no catalogue files next to it,
//! and the language is picked at run time with
//! `slint::select_bundled_translation`.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=lang");
    slint_build::compile_with_config(
        "ui/app.slint",
        slint_build::CompilerConfiguration::new().with_bundled_translations("lang"),
    )?;
    Ok(())
}
