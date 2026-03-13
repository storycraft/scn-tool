use anyhow::{Context, bail};
use clap::Parser;
use emote_psb::{
    psb::{read::PsbFile, write::PsbWriter},
    value::PsbValue,
};
use scn_script_common::{Script, Text};
use std::{
    fs::File,
    io::{BufReader, BufWriter},
    path::PathBuf,
};

#[derive(Parser)]
struct App {
    json_patch_file: PathBuf,
    scn_file: PathBuf,
    output_file: Option<PathBuf>,
}

fn main() {
    if let Err(err) = run(App::parse()) {
        eprintln!("Error: {err:?}");
    }
}

fn run(app: App) -> anyhow::Result<()> {
    let mut psb = PsbFile::open(BufReader::new(
        File::open(&app.scn_file).context("scn file does not exist")?,
    ))
    .context("opening scn file")?;
    let mut root = psb.deserialize_root::<PsbValue>()?;

    let output_path = app.output_file.clone().unwrap_or_else(|| {
        let mut path = app.scn_file.clone();
        path.set_file_name(format!(
            "{}.patched.scn",
            app.scn_file.file_stem().unwrap().to_string_lossy()
        ));
        path
    });

    let script: Script = serde_json::from_reader(BufReader::new(
        File::open(&app.json_patch_file).context("patch file does not exist")?,
    ))
    .context("reading patch file")?;
    patch(&script, &mut root).context("patching scn")?;

    PsbWriter::new(
        psb.version,
        psb.encrypted,
        &root,
        BufWriter::new(File::create(output_path).context("creating output scn file")?),
    )
    .context("writing patched scn file")?
    .finish()
    .context("finishing patched scn file")?;

    Ok(())
}

fn patch(script: &Script, root: &mut PsbValue) -> anyhow::Result<()> {
    // query scenes
    let scenes = root.query_str("scenes")?;
    for (i, scene) in script.scenes.iter().enumerate() {
        let scn_scene = scenes.query(i as _)?;
        // set title
        *scn_scene.query_str("title")? = PsbValue::String(From::from(&scene.title));

        if !scene.texts.is_empty() {
            // query texts
            let texts = scn_scene.query_str("texts")?;
            for (i, text) in scene.texts.iter().enumerate() {
                let scn_text = texts.query(i as _)?;
                patch_flatten_text(scn_text, text).context("applying text patch to scn")?;
            }
        }

        if !scene.selects.is_empty() {
            // query selectInfo/selects
            let selects = scn_scene.query_str("selectInfo")?.query_str("selects")?;
            for (i, select) in scene.selects.iter().enumerate() {
                let scn_select = selects.query(i as _)?;
                *scn_select.query_str("text")? = PsbValue::String(From::from(&select.text));
            }
        }
    }

    Ok(())
}

fn patch_flatten_text(scn_text: &mut PsbValue, text: &Text) -> anyhow::Result<()> {
    fn write_flatten(slot: &[Option<&str>], v: &mut PsbValue) -> anyhow::Result<usize> {
        let Some(&string) = slot.first() else {
            return Ok(0);
        };

        match v {
            PsbValue::List(list) => {
                let mut offset = 0;
                for child in list {
                    offset += write_flatten(&slot[offset..], child)?;
                    if offset >= slot.len() {
                        break;
                    }
                }

                Ok(offset)
            }

            v => {
                *v = if let Some(string) = string {
                    PsbValue::String(string.into())
                } else {
                    PsbValue::Null
                };
                Ok(1)
            }
        }
    }

    let written = write_flatten(
        &[
            text.name.as_deref(),
            text.display_name.as_deref(),
            text.text.as_deref(),
        ],
        scn_text,
    )?;
    if written != 3 {
        bail!("fail to patch scn text correctly. Unsupported or invalid");
    }

    Ok(())
}

trait QueryExt {
    fn query(&mut self, index: isize) -> anyhow::Result<&mut PsbValue>;
    fn query_str(&mut self, key: &str) -> anyhow::Result<&mut PsbValue>;
}

impl QueryExt for PsbValue {
    fn query(&mut self, index: isize) -> anyhow::Result<&mut Self> {
        match self {
            Self::Object(obj) => {
                let i = if index < 0 {
                    obj.len().wrapping_add_signed(index)
                } else {
                    index as _
                };

                Ok(obj
                    .get_index_mut(i)
                    .with_context(|| format!("invalid path: {} in object", index))?
                    .1)
            }
            Self::List(list) => {
                let i = if index < 0 {
                    list.len().wrapping_add_signed(index)
                } else {
                    index as _
                };

                list.get_mut(i)
                    .with_context(|| format!("invalid path: {} in list", index))
            }
            _ => bail!("invalid path: {}", index),
        }
    }

    fn query_str(&mut self, key: &str) -> anyhow::Result<&mut PsbValue> {
        match self {
            Self::Object(obj) => obj
                .get_mut(key)
                .with_context(|| format!("invalid path: {} in object", key)),
            _ => bail!("invalid path: {}", key),
        }
    }
}
