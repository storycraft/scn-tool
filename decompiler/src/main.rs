use std::{
    fs::File,
    io::{BufReader, BufWriter},
    path::PathBuf,
};

use anyhow::Context;
use clap::Parser;
use emote_psb::psb::read::PsbFile;

#[derive(Parser)]
struct App {
    scn_file: PathBuf,
    output_json_file: Option<PathBuf>,
}

fn main() {
    if let Err(err) = run(App::parse()) {
        eprintln!("Error: {err:?}");
    }
}

fn run(app: App) -> anyhow::Result<()> {
    let input_name = app
        .scn_file
        .file_stem()
        .context("invalid path")?
        .to_str()
        .context("invalid path string")?;

    let output_path = app.output_json_file.clone().unwrap_or_else(|| {
        let mut path = app.scn_file.clone();
        path.set_file_name(format!("{}.json", input_name));
        path
    });

    let mut psb = PsbFile::open(BufReader::new(
        File::open(&app.scn_file).context("input scn file not found")?,
    ))
    .context("scn file reading")?;

    let output = BufWriter::new(File::create(&output_path).context("creating output file")?);
    serde_transcode::transcode(
        &mut psb.root_deserializer()?,
        &mut serde_json::Serializer::pretty(output),
    )?;
    Ok(())
}
