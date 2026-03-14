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
    patch(script, &mut root).context("patching scn")?;

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

fn patch(script: Script, root: &mut PsbValue) -> anyhow::Result<()> {
    // query scenes
    let scenes = root.query_str("scenes")?;
    for (i, scene) in script.scenes.into_iter().enumerate() {
        let scn_scene = scenes.query(i as _)?;
        // set title
        *scn_scene.query_str("title")? = PsbValue::String(From::from(&scene.title));

        if !scene.texts.is_empty() {
            // query texts
            let texts = scn_scene.query_str("texts")?;
            for (i, text) in scene.texts.into_iter().enumerate() {
                let scn_text = texts.query(i as _)?;
                patch_text(text, scn_text).context("applying text patch to scn")?;
            }
        }

        if !scene.selects.is_empty() {
            // query selectInfo/selects
            let selects = scn_scene.query_str("selectInfo")?.query_str("selects")?;
            for (i, select) in scene.selects.into_iter().enumerate() {
                let scn_select = selects.query(i as _)?;
                *scn_select.query_str("text")? = PsbValue::String(From::from(&select.text));
            }
        }
    }

    Ok(())
}

fn patch_text(text: Text, scn_text: &mut PsbValue) -> anyhow::Result<()> {
    if patch_v2(&text, scn_text).is_some() {
        return Ok(());
    }

    if patch_v1(&text, scn_text).is_some() {
        return Ok(());
    }

    bail!("fail to patch scn text correctly. Unsupported or invalid")
}

fn to_opt_string(string: Option<&str>) -> PsbValue {
    match string {
        Some(string) => PsbValue::String(string.to_string()),
        None => PsbValue::Null,
    }
}

fn patch_v1(text: &Text, scn_text: &mut PsbValue) -> Option<()> {
    let PsbValue::List(list) = scn_text else {
        return None;
    };

    if list.len() < 3 {
        return None;
    }

    let dialogue = text.dialogues.get(0)?;
    let dialogue_text = dialogue.values.get(0)?;
    list[0] = to_opt_string(text.name.as_deref());
    list[1] = to_opt_string(dialogue.display_name.as_deref());
    list[2] = dialogue_text.clone();
    Some(())
}

fn patch_v2(text: &Text, scn_text: &mut PsbValue) -> Option<()> {
    fn inner<const OFFSET: usize>(text: &Text, scn_text: &mut PsbValue) -> Option<()> {
        let PsbValue::List(list) = scn_text else {
            return None;
        };

        let PsbValue::List(scn_dialogues) = list.get_mut(1 + OFFSET)? else {
            return None;
        };
        for (scn_dialogue, dialogue) in scn_dialogues.iter_mut().zip(&text.dialogues) {
            let PsbValue::List(scn_dialogue) = scn_dialogue else {
                return None;
            };
            scn_dialogue.clear();
            scn_dialogue.push(to_opt_string(dialogue.display_name.as_deref()));
            scn_dialogue.extend(dialogue.values.iter().cloned());
        }

        list[0] = to_opt_string(text.name.as_deref());
        Some(())
    }

    inner::<0>(text, scn_text).or_else(|| inner::<1>(text, scn_text))
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
