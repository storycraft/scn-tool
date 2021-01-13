/*
 * Created on Mon Jan 11 2021
 *
 * Copyright (c) storycraft. Licensed under the MIT Licence.
 */

use std::{env, fs::{self, File}, io::{BufWriter, Cursor, Read, Write}, path::Path};

use emote_psb::{PsbReader, types::{PsbValue, collection::PsbList}};

use toml::Value;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("Usage: {} <scn_file> [output_file]", args[0]);
        return;
    }
    
    let input_path = Path::new(&args[1]);
    let input_name = input_path.file_stem().unwrap().to_str().unwrap();

    let default_output_name = {
        let mut name = String::from(input_path.parent().unwrap().join(input_name).to_str().unwrap());
        name.push_str(".toml");
        name
    };

    let output_path = Path::new(args.get(2).unwrap_or(&default_output_name));

    fs::create_dir_all(output_path.parent().unwrap()).expect("Cannot create output directory");

    let output_file = File::create(output_path).expect("Cannot create output file");
    let mut output_writer = BufWriter::new(output_file);

    let mut input_scn = PsbReader::open_psb({
        let mut file = File::open(input_path).expect(&format!("Cannot open {}", input_path.to_str().unwrap()));
        let mut mem = Vec::new();
        file.read_to_end(&mut mem).expect("Cannot read input file");

        Cursor::new(mem)
    }).expect("Input file is invalid scn file");

    let (_, strings_ref, _, _, root) = input_scn.load().expect("Cannot load scn input file").unwrap();

    writeln!(output_writer, "# {}\n", input_name).expect("Cannot write file header");

    let mut scripts: Vec::<(Option<u64>, Option<u64>, u64)> = Vec::new();
    let mut characters: Vec::<u64> = Vec::new();
    let mut characters_display: Vec::<u64> = Vec::new();
    let mut titles: Vec::<u64> = Vec::new();

    let mut strings: Vec::<u64> = Vec::new();

    let mut select_infos: Vec::<u64> = Vec::new();

    match root.get_value("scenes".into()) {

        Some(scenes) => {
            if let PsbValue::List(list) = scenes {
                for scene in list.iter() {
                    if let PsbValue::Object(scene) = scene {
                        // 타이틀 수집
                        match scene.get_value("title".into()) {
                            Some(title) => {
                                if let PsbValue::StringRef(str_ref) = title {
                                    if !titles.contains(&str_ref.ref_index()) {
                                        titles.push(str_ref.ref_index());
                                    }
                                }
                            },

                            None => {}
                        }

                        // 대사 수집
                        match scene.get_value("texts".into()) {
                            Some(texts) => {
                                if let PsbValue::List(texts) = texts {
                                    for text in texts.iter() {
                                        match text {
        
                                            PsbValue::List(text_list) => {
                                                let list = read_items(3, text_list);

                                                let mut character: Option<u64> = None;
                                                let mut character_display: Option<u64> = None;
                                                let text: u64;

                                                if list.len() < 3 {
                                                    continue;
                                                }

                                                if let PsbValue::StringRef(text_entry) = list[2] {
                                                    text = text_entry.ref_index();
                                                } else {
                                                    continue;
                                                }
        
                                                if let PsbValue::StringRef(character_entry) = list[0] {
                                                    character = Some(character_entry.ref_index());
                                                }
        
                                                if let PsbValue::StringRef(character_display_entry) = list[1] {
                                                    character_display = Some(character_display_entry.ref_index());
                                                }
        
                                                if character.is_some() && !characters.contains(&character.unwrap()) {
                                                    characters.push(character.unwrap());
                                                }
        
                                                if character_display.is_some() && !characters_display.contains(&character_display.unwrap()) {
                                                    characters_display.push(character_display.unwrap());
                                                }
        
                                                scripts.push((character, character_display, text));
                                            }
        
                                            _ => {}
                                        }
                                    }
                                } else {
                                    panic!("texts entry is invalid")
                                }
                            },

                            None => {

                            }
                        }

                        // 선택지 정보
                        match scene.get_value("selectInfo".into()) {

                            Some(select_info) => {
                                if let PsbValue::Object(sel) = select_info {
                                    match sel.get_value("select".into()) {

                                        Some(sel) => {
                                            if let PsbValue::List(select) = sel {
                                                for select_item in select.iter() {
                                                    if let PsbValue::Object(item) = select_item {
                                                        match item.get_value("runLineStr".into()) {

                                                            Some(run_line_str) => {
                                                                if let PsbValue::StringRef(str_ref) = run_line_str {
                                                                    select_infos.push(str_ref.ref_index())
                                                                }
                                                            }

                                                            None => {}
                                                        }

                                                        match item.get_value("text".into()) {

                                                            Some(text_str) => {
                                                                if let PsbValue::StringRef(str_ref) = text_str {
                                                                    select_infos.push(str_ref.ref_index())
                                                                }
                                                            }

                                                            None => {}
                                                        }
                                                    }
                                                }
                                            }
                                        },

                                        None => {}
                                    }
                                }
                            },

                            None => {}
                        }
                        
                        // 선택지
                        match scene.get_value("selects".into()) {

                            Some(selects) => {
                                if let PsbValue::List(select_list) = selects {
                                    for select in select_list.iter() {
                                        if let PsbValue::Object(select) = select {
                                            match select.get_value("text".into()) {

                                                Some(text) => {
                                                    if let PsbValue::StringRef(str_ref) = text {
                                                        select_infos.push(str_ref.ref_index());
                                                    }
                                                }

                                                None => {}
                                            }
                                        }
                                    }
                                }
                            },

                            None => {}
                        }
                    }
                }
            } else {
                panic!("scenes entry is invalid")
            }
        }

        None => {
            strings = collect_string_indices(&PsbValue::Object(root));
        }
    }

    let mut used_texts: Vec::<u64> = Vec::new();

    // scn 정보
    writeln!(output_writer, "[info]").unwrap();
    for title in titles.iter() {
        let title_str = strings_ref.get(*title as usize).expect(&format!("Cannot find string reference # {}", title));

        if used_texts.contains(title) {
            write!(output_writer, "# ").unwrap();
        } else {
            used_texts.push(*title);
        }

        writeln!(output_writer, "{} = {}", *title, Value::String(title_str.clone())).unwrap();
    }
    writeln!(output_writer).unwrap();

    // 등장 인물
    writeln!(output_writer, "[characters]").unwrap();
    for character in characters.iter() {
        let character_str = strings_ref.get(*character as usize).expect(&format!("Cannot find string reference # {}", character));

        if used_texts.contains(character) {
            write!(output_writer, "# ").unwrap();
        } else {
            used_texts.push(*character);
        }

        writeln!(output_writer, "{} = {}", *character, Value::String(character_str.clone())).unwrap();
    }
    writeln!(output_writer).unwrap();

    // 등장 인물 이름 치환
    writeln!(output_writer, "[character_subs]").unwrap();
    for character_sub in characters_display.iter() {
        let character_sub_str = strings_ref.get(*character_sub as usize).expect(&format!("Cannot find string reference # {}", character_sub));

        if used_texts.contains(character_sub) {
            write!(output_writer, "# ").unwrap();
        } else {
            used_texts.push(*character_sub);
        }

        writeln!(output_writer, "{} = {}", *character_sub, Value::String(character_sub_str.clone())).unwrap();
    }
    writeln!(output_writer).unwrap();

    // 대사
    writeln!(output_writer, "[script]").unwrap();
    writeln!(output_writer, "# lines: {}\n", scripts.len()).unwrap();
    for (character, character_sub, text) in scripts.iter() {
        if character.is_some() {
            let character = character.unwrap();
            let character_str = strings_ref.get(character as usize).expect(&format!("Cannot find string reference # {}", character));

            writeln!(output_writer, "# {}", character_str).unwrap();
        } else {
            writeln!(output_writer, "# monologue").unwrap();
        }

        if character_sub.is_some() {
            let character_sub = character_sub.unwrap();
            let character_sub_str = strings_ref.get(character_sub as usize).expect(&format!("Cannot find string reference # {}", character_sub));

            writeln!(output_writer, "# display as {}", character_sub_str).unwrap();
        }

        let text_str = strings_ref.get(*text as usize).expect(&format!("Cannot find string reference # {}", text));

        writeln!(output_writer, "# {}", text_str).unwrap();
        if used_texts.contains(text) {
            write!(output_writer, "# ").unwrap();
        } else {
            used_texts.push(*text);
        }

        writeln!(output_writer, "{} = {}\n", *text, Value::String(text_str.clone())).unwrap();
    }

    // 선택지
    if select_infos.len() > 0 {
        writeln!(output_writer, "[selections]").unwrap();

        for select_id in select_infos.iter() {
            let string = strings_ref.get(*select_id as usize).expect(&format!("Cannot find string reference # {}", select_id));

            if used_texts.contains(select_id) {
                write!(output_writer, "# ").unwrap();
            } else {
                used_texts.push(*select_id);
            }

            writeln!(output_writer, "{} = {}", *select_id, Value::String(string.clone())).unwrap();
        }
    }

    // 문자열
    if strings.len() > 0 {
        writeln!(output_writer, "[strings]").unwrap();
        writeln!(output_writer, "# count: {}\n", strings.len()).unwrap();

        for string_id in strings.iter() {
            let string = strings_ref.get(*string_id as usize).expect(&format!("Cannot find string reference # {}", string_id));

            if used_texts.contains(string_id) {
                write!(output_writer, "# ").unwrap();
            } else {
                used_texts.push(*string_id);
            }
    
            writeln!(output_writer, "{} = {}", *string_id, Value::String(string.clone())).unwrap();
        }
    }

}

fn read_items(count: usize, list: &PsbList) -> Vec<&PsbValue> {
    let mut vec = Vec::<&PsbValue>::with_capacity(count);

    let iter = &mut list.iter();
    while vec.len() < count {
        let child = iter.next();

        if child.is_none() {
            return vec;
        }

        match child.unwrap() {

            PsbValue::List(child_list) => {
                vec.append(&mut read_items(count - vec.len(), child_list));
            }

            child => {
                vec.push(child);
            }
        }

    }

    vec
}

fn collect_string_indices(root: &PsbValue) -> Vec<u64> {
    let mut vec = Vec::new();

    fn collect(current: &PsbValue, vec: &mut Vec<u64>) {
        match current {

            PsbValue::StringRef(str_ref) => {
                let index = str_ref.ref_index();
                if !vec.contains(&index) {
                    vec.push(index);
                }
            }

            PsbValue::Object(obj) => {
                for (_, value) in obj.iter() {
                    collect(value, vec);
                }
            }

            PsbValue::List(list) => {
                for value in list.iter() {
                    collect(value, vec);
                }
            }

            _ => {}

        }
    }

    collect(root, &mut vec);

    vec
}
