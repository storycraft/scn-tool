use anyhow::{Context, bail};
use clap::Parser;
use emote_psb::{
    mdf::{MdfReader, MdfWriter},
    psb::{read::PsbFile, write::PsbWriter},
    value::{PsbValue, de},
};
use scn_script_common::{Script, Text};
use serde::{Serialize, de::DeserializeOwned};
use std::{
    fs::File,
    io::{BufRead, BufReader, BufWriter, Cursor, Read, Seek, SeekFrom, Write},
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

enum PsbInput<'a> {
    Mdf(PsbFile<Cursor<Vec<u8>>>),
    Psb(PsbFile<&'a mut BufReader<File>>),
}

impl PsbInput<'_> {
    fn deserialize_root<V: DeserializeOwned>(&mut self) -> Result<V, de::Error> {
        match self {
            PsbInput::Mdf(psb) => psb.deserialize_root(),
            PsbInput::Psb(psb) => psb.deserialize_root(),
        }
    }
}

fn run(app: App) -> anyhow::Result<()> {
    let mut psb_input =
        BufReader::new(File::open(&app.scn_file).context("scn file does not exist")?);

    let mut psb = if let Ok(mut mdf) = MdfReader::open(&mut psb_input) {
        let mut buf = vec![];
        mdf.read_to_end(&mut buf)?;
        PsbInput::Mdf(PsbFile::open(Cursor::new(buf)).context("opening scn(mdf) file")?)
    } else {
        psb_input.seek(SeekFrom::Start(0))?;
        PsbInput::Psb(PsbFile::open(&mut psb_input).context("opening scn file")?)
    };
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

    let out_file = BufWriter::new(File::create(output_path).context("creating output scn file")?);
    match psb {
        PsbInput::Mdf(mut psb) => {
            let mut buf = vec![];
            write_psb(&mut psb, &root, Cursor::new(&mut buf))?;

            let mut mdf = MdfWriter::new(out_file, 1)?;
            mdf.write_all(&buf)?;
            mdf.finish().context("packing mdf file")?;
        }
        PsbInput::Psb(mut psb) => {
            write_psb(&mut psb, &root, out_file)?;
        }
    }

    Ok(())
}

fn write_psb(
    psb: &mut PsbFile<impl BufRead + Seek>,
    root: &impl Serialize,
    out: impl Write + Seek,
) -> anyhow::Result<()> {
    let mut writer = PsbWriter::new(psb.version, psb.encrypted, &root, out)
        .context("writing patched scn file")?;

    for i in 0..psb.resources() {
        let Some(mut res) = psb.open_resource(i)? else {
            unreachable!()
        };
        let mut buf = vec![];
        res.read_to_end(&mut buf)?;

        writer.add_resource(Cursor::new(buf))?;
    }

    for i in 0..psb.extra_resources() {
        let Some(mut res) = psb.open_extra_resource(i)? else {
            unreachable!()
        };
        let mut buf = vec![];
        res.read_to_end(&mut buf)?;

        writer.add_extra(Cursor::new(buf))?;
    }

    writer.finish().context("finishing patched scn file")?;
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
                *scn_select.query_str("text")? = PsbValue::String(select);
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

    let dialogue = text.dialogues.first()?;
    let dialogue_text = dialogue.values.first()?;
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
