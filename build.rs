use std::error::Error;
use vergen::EmitBuilder;

fn main() -> Result<(), Box<dyn Error>> {
    EmitBuilder::builder()
        .git_describe(true, false, None)
        .build_date()
        .emit()?;
    Ok(())
}
