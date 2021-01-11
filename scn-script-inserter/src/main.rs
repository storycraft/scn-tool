use std::{collections::HashMap, env, fs::File, io::{BufReader, BufWriter, Cursor, Read}, path::Path};

use emote_psb::{PsbReader, writer::PsbWriter};

use serde::{Serialize, Deserialize};

/*
 * Created on Mon Jan 11 2021
 *
 * Copyright (c) storycraft. Licensed under the MIT Licence.
 */

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        println!("Usage: {} <toml_patch_file> <scn_file> [output_file]", args[0]);
        return;
    }

    let patch_path = Path::new(&args[1]);
    let mut patch_file = File::open(patch_path).expect("Cannot create patch file");

    let scn_path = Path::new(&args[2]);
    let scn_name = scn_path.file_stem().unwrap().to_str().unwrap();
    
    let default_output_path = {
        let mut name = String::from(scn_path.parent().unwrap().join(scn_name).to_str().unwrap());

        name.push_str(".patched.scn");

        name
    };

    let output_path = Path::new(args.get(3).unwrap_or(&default_output_path));

    let mut psb = PsbReader::open_psb_file({
        let mut mem = Vec::new();

        BufReader::new(
            File::open(scn_path).expect("Cannot open scn file")
        ).read_to_end(&mut mem).unwrap();
        
        Cursor::new(mem)
    }).expect("Input scn file is invalid");

    let (_, root) = psb.read_root().expect("scn entry is invalid");

    let mut ref_table = psb.ref_table().clone();

    let raw_patch_file = {
        let mut mem = Vec::new();

        BufReader::new(&mut patch_file).read_to_end(&mut mem).expect("Cannot read patch file");

        mem
    };

    let patch_file = toml::from_slice::<PatchFile>(&raw_patch_file).expect("Corrupted patch file");

    let chain = patch_file.info.string_keys.iter()
        .chain(patch_file.characters.string_keys.iter())
        .chain(patch_file.character_subs.string_keys.iter()) 
        .chain(patch_file.script.string_keys.iter())
        .chain(patch_file.strings.string_keys.iter());

    for (key, patch) in chain {
        let key: usize = key.parse().unwrap();

        ref_table.strings_mut()[key] = patch.clone();
    }

    let mut output_file = File::create(output_path).expect("Cannot create output file");

    let writer = PsbWriter::new(psb.header(), ref_table, root, BufWriter::new(&mut output_file));

    writer.finish().expect("Cannot write patched scn file");
}

#[derive(Debug, Deserialize, Serialize)]
struct PatchFile {

    pub info: StringPatchSet,
    pub characters: StringPatchSet,
    pub character_subs: StringPatchSet,
    pub script: StringPatchSet,
    #[serde(default)]
    pub strings: StringPatchSet

}

#[derive(Debug, Default, Deserialize, Serialize)]
struct StringPatchSet {

    #[serde(flatten)]
    pub string_keys: HashMap<String, String>

}