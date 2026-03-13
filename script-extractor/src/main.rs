use std::{
    fs::{self, File},
    io::{BufReader, BufWriter},
    path::PathBuf,
};

use anyhow::Context;
use clap::Parser;
use emote_psb::psb::read::PsbFile;
use scn_script_common::Script;

#[derive(Parser)]
struct App {
    scn_file: PathBuf,
    output_file: Option<PathBuf>,
}

fn main() {
    if let Err(err) = run(App::parse()) {
        eprintln!("Error: {err:?}");
    }
}

fn run(app: App) -> anyhow::Result<()> {
    let input_path = app.scn_file;
    let input_name = input_path
        .file_stem()
        .context("invalid path")?
        .to_str()
        .context("invalid path string")?;

    let output_path = app.output_file.unwrap_or_else(|| {
        let mut path = input_path.clone();
        path.set_file_name(format!("{}.json", input_name));
        path
    });

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).context("creating output directory")?;
    }

    let out = BufWriter::new(File::create(output_path).context("creating output file")?);

    let mut input_scn = PsbFile::open(BufReader::new(
        File::open(&input_path).context("cannot open input file")?,
    ))
    .context("input file is invalid scn")?;

    let script = input_scn
        .deserialize_root::<Script>()
        .context("deserializing scn")?;
    serde_json::to_writer_pretty(out, &script).context("writing json file")?;
    Ok(())
}
