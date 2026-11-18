//! Compiles the Slint screens in `ui/` (design: design/README.md).

fn main() -> Result<(), Box<dyn std::error::Error>> {
    slint_build::compile("ui/app.slint")?;
    Ok(())
}
