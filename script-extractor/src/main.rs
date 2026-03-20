use std::{
    fs::{self, File},
    io::{BufReader, BufWriter, Cursor, Read, Seek, SeekFrom},
    path::PathBuf,
};

use anyhow::{Context, bail};
use clap::Parser;
use emote_psb::{mdf::MdfReader, psb::read::PsbFile, value::PsbValue};
use scn_script_common::{Dialogue, Scene, Script, Text};
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

    let mut input = File::open(&input_path).context("cannot open input file")?;
    let mut buf = vec![];
    if let Ok(mut mdf) = MdfReader::open(BufReader::new(&mut input)) {
        mdf.read_to_end(&mut buf)
            .context("uncompressing mdf file")?;
    } else {
        input.seek(SeekFrom::Start(0))?;
        input.read_to_end(&mut buf).context("reading scn file")?;
    };

    let scn_script: ScnScript = PsbFile::open(Cursor::new(buf))
        .context("input file is invalid scn")?
        .deserialize_root()?;

    let out = BufWriter::new(File::create(output_path).context("creating output file")?);

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
                        .map(read_text)
                        .collect::<anyhow::Result<Vec<_>>>()
                        .context("collecting text from scn")?,
                    selects: scn_scene.selects.into_iter().map(|sel| sel.text).collect(),
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
    pub selects: Vec<ScnSelect>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ScnSelect {
    pub text: String,
}

fn read_text(text: PsbValue) -> anyhow::Result<Text> {
    if let Some(text) = read_extract_v2(&text) {
        return Ok(text);
    }

    if let Some(text) = read_extract_v1(&text) {
        return Ok(text);
    }

    bail!("fail to read scn text correctly. Unsupported or invalid")
}

fn read_option_str(v: &PsbValue) -> Option<Option<String>> {
    match v {
        PsbValue::Null => Some(None),
        PsbValue::String(display_name) => Some(Some(display_name.to_string())),
        _ => None,
    }
}

fn read_extract_v1(text: &PsbValue) -> Option<Text> {
    let PsbValue::List(list) = text else {
        return None;
    };
    let name = read_option_str(list.first()?)?;
    let display_name = read_option_str(list.get(1)?)?;

    let text = list.get(2)?.clone();
    Some(Text {
        name,
        dialogues: vec![Dialogue {
            display_name,
            values: vec![text],
        }],
    })
}

fn read_extract_v2(text: &PsbValue) -> Option<Text> {
    fn inner<const OFFSET: usize>(text: &PsbValue) -> Option<Text> {
        let PsbValue::List(list) = text else {
            return None;
        };
        let name = read_option_str(list.first()?)?;

        let PsbValue::List(list) = list.get(1 + OFFSET)? else {
            return None;
        };

        let mut dialogues = Vec::with_capacity(list.len());
        for item in list {
            let PsbValue::List(item) = item else {
                return None;
            };

            let dialogue = Dialogue {
                display_name: read_option_str(item.first()?)?,
                values: item[1..].to_vec(),
            };
            dialogues.push(dialogue);
        }

        Some(Text { name, dialogues })
    }

    inner::<0>(text).or_else(|| inner::<1>(text))
}
