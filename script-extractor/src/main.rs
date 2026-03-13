use std::{
    fs::{self, File},
    io::{BufReader, BufWriter},
    path::PathBuf,
};

use anyhow::{bail, Context};
use clap::Parser;
use emote_psb::{psb::read::PsbFile, value::PsbValue};
use scn_script_common::{Scene, Script, Select, Text};
use serde::{Deserialize, Serialize};

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

    let scn_script = input_scn
        .deserialize_root::<ScnScript>()
        .context("deserializing scn")?;

    let script = Script {
        scenes: scn_script
            .scenes
            .into_iter()
            .map(|scn_scene| {
                Ok(Scene {
                    title: scn_scene.title,
                    texts: scn_scene
                        .texts
                        .into_iter()
                        .map(read_flatten_text)
                        .collect::<anyhow::Result<Vec<_>>>()
                        .context("collecting text from scn")?,
                    selects: scn_scene.selects,
                })
            })
            .collect::<anyhow::Result<Vec<_>>>()?,
    };

    serde_json::to_writer_pretty(out, &script).context("writing json file")?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ScnScript {
    pub scenes: Vec<ScnScene>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ScnScene {
    pub title: String,

    #[serde(default)]
    pub texts: Vec<PsbValue>,

    #[serde(default)]
    pub selects: Vec<Select>,
}

fn read_flatten_text(text: PsbValue) -> anyhow::Result<Text> {
    fn read_flatten(slot: &mut [Option<String>], v: PsbValue) -> anyhow::Result<usize> {
        if slot.is_empty() {
            return Ok(0);
        }

        match v {
            PsbValue::Null => {
                slot[0] = None;
                Ok(1)
            }

            PsbValue::String(string) => {
                slot[0] = Some(string);
                Ok(1)
            }

            PsbValue::List(list) => {
                let mut offset = 0;
                for child in list {
                    offset += read_flatten(&mut slot[offset..], child)?;
                    if offset >= slot.len() {
                        break;
                    }
                }

                Ok(offset)
            }

            _ => bail!("invalid or unsupported scn text"),
        }
    }

    let mut slot = [const { None }; 3];
    if read_flatten(&mut slot, text)? != slot.len() {
        bail!("fail to read scn text correctly. Unsupported or invalid");
    }

    let [name, display_name, text] = slot;
    Ok(Text {
        name,
        display_name,
        text,
    })
}
