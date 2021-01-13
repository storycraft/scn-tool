use std::{collections::HashMap, env, fs::{self, File}, io::{self, BufReader, BufWriter, Cursor, Read, Seek, SeekFrom}, path::Path};

use emote_psb::{PsbReader, header::PsbHeader, offsets::{PsbOffsets, PsbStringOffset}, writer::PsbWriter};

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

    let mut raw_psb = Vec::new();

    BufReader::new(
        File::open(scn_path).expect("Cannot open scn file")
    ).read_to_end(&mut raw_psb).unwrap();

    // 헤더 읽기
    let (_, header) = {
        let mut cursor = Cursor::new(&mut raw_psb);
        cursor.set_position(4);
        PsbHeader::from_bytes(&mut cursor).expect("Cannot read header")
    };

    // 오프셋 로딩
    let (_, mut offsets) = {
        let mut cursor = Cursor::new(&mut raw_psb);
        cursor.set_position(12);
        PsbOffsets::from_bytes(header.version, &mut cursor).expect("Cannot read string offsets")
    };

    // 패치 파일 로딩
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

    let (strings_read, mut strings) = {
        let mut cursor = Cursor::new(&mut raw_psb);

        cursor.set_position(offsets.strings.offset_pos as u64);
        
        PsbReader::read_strings(offsets.strings.data_pos, &mut cursor).expect("Cannot read strings")
    };

    for (key, patch) in chain {
        let key: usize = key.parse().unwrap();

        strings[key] = patch.clone();
    }

    let mut output_file = File::create(output_path).expect("Cannot create output file");
    // 앞부분 복사
    io::copy(&mut Cursor::new(&mut raw_psb[..offsets.strings.offset_pos as usize]), &mut output_file).expect("Cannot copy source file");

    // 문자열 덮어쓰기
    let (new_string_written, new_string_offsets) = PsbWriter::write_strings(&strings, &mut output_file).expect("Cannot write strings");

    // 뒷부분 복사
    io::copy(&mut Cursor::new(&mut raw_psb[(offsets.strings.offset_pos as u64 + strings_read) as usize..]), &mut output_file).expect("Cannot copy source file trail");

    // 리소스 오프셋 업데이트
    let diff = new_string_written as i32 - strings_read as i32;
    
    offsets.resources.offset_pos = (offsets.resources.offset_pos as i32 + diff) as u32;
    offsets.resources.data_pos = (offsets.resources.data_pos as i32 + diff) as u32;
    offsets.resources.lengths_pos = (offsets.resources.lengths_pos as i32 + diff) as u32;

    if header.version > 3 {
        let extra = offsets.extra.unwrap();
        offsets.extra.unwrap().offset_pos = (extra.offset_pos as i32 + diff) as u32;
        offsets.extra.unwrap().data_pos = (extra.data_pos as i32 + diff) as u32;
        offsets.extra.unwrap().lengths_pos = (extra.lengths_pos as i32 + diff) as u32;
    }

    offsets.strings = new_string_offsets;
    // 오프셋 업데이트
    {
        let mut writer = BufWriter::new(&mut output_file);

        writer.seek(SeekFrom::Start(12)).unwrap();
        offsets.write_bytes(header.version, &mut writer).expect("Cannot patch string offsets");
    }
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