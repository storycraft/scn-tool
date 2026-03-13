use std::{
    fs::File,
    io::{BufReader, BufWriter},
    path::PathBuf,
};

use anyhow::Context;
use clap::Parser;
use emote_psb::{psb::write::PsbWriter, value::PsbValue};

#[derive(Parser)]
struct App {
    json_file: PathBuf,
    output_scn_file: Option<PathBuf>,
}

fn main() {
    if let Err(err) = run(App::parse()) {
        eprintln!("Error: {err:?}");
    }
}

fn run(app: App) -> anyhow::Result<()> {
    let input_name = app
        .json_file
        .file_stem()
        .context("invalid path")?
        .to_str()
        .context("invalid path string")?;

    let output_path = app.output_scn_file.clone().unwrap_or_else(|| {
        let mut path = app.json_file.clone();
        path.set_file_name(format!("{}.scn", input_name));
        path
    });

    let root: PsbValue = serde_json::from_reader(BufReader::new(
        File::open(&app.json_file).context("input json file not found")?,
    ))
    .context("invalid json")?;

    PsbWriter::new(
        2,
        false,
        &root,
        BufWriter::new(File::create(&output_path).context("creating output file")?),
    )
    .context("writing psb file")?
    .finish()?;
    Ok(())
}
