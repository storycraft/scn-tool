/*
 * Created on Mon Jan 11 2021
 *
 * Copyright (c) storycraft. Licensed under the MIT Licence.
 */

use std::{env, fs::{self, File}, io::{BufWriter, Cursor, Read, Write}, path::Path};

use emote_psb::{PsbReader, types::PsbValue};

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

    let mut input_scn = PsbReader::open_psb_file({
        let mut file = File::open(input_path).expect(&format!("Cannot open {}", input_path.to_str().unwrap()));
        let mut mem = Vec::new();
        file.read_to_end(&mut mem).expect("Cannot read input file");

        Cursor::new(mem)
    }).expect("Input file is invalid scn file");

    let (_, root) = input_scn.read_root().expect("Cannot read entry point of input file");

    let ref_table = input_scn.ref_table();

    let scenes_id = ref_table.names().binary_search(&String::from("scenes")).ok();
    let texts_id = ref_table.names().binary_search(&String::from("texts")).ok();

    let title_id = ref_table.names().binary_search(&String::from("title")).ok();

    writeln!(output_writer, "# {}\n", input_name).expect("Cannot write file header");

    let mut scripts: Vec::<(Option<u64>, Option<u64>, u64)> = Vec::new();
    let mut characters: Vec::<u64> = Vec::new();
    let mut characters_display: Vec::<u64> = Vec::new();
    let mut titles: Vec::<u64> = Vec::new();

    let mut strings: Vec::<u64> = Vec::new();

    if let PsbValue::Object(root) = root {
        match scenes_id {

            Some(scenes_id) => {
                let scenes = root.get_value(scenes_id as u64).expect("Root entry not found");

                if let PsbValue::List(list) = scenes {
                    for scene in list.iter() {
        
                        if let PsbValue::Object(scene) = scene {
                            if title_id.is_some() {
                                let title = scene.get_value(title_id.unwrap() as u64);
                                
                                if title.is_some() {
                                    if let PsbValue::String(str_ref) = title.unwrap() {
                                        if !titles.contains(&str_ref.ref_index()) {
                                            titles.push(str_ref.ref_index());
                                        }
                                    }
                                }
                            }
        
                            if texts_id.is_some() {
                                let texts = scene.get_value(texts_id.unwrap() as u64);
                                if texts.is_some() {
                                    if let PsbValue::List(texts) = texts.unwrap() {
                                        for text in texts.iter() {
                                            match text {
            
                                                PsbValue::List(text_list) => {
                                                    let mut character: Option<u64> = None;
                                                    let mut character_display: Option<u64> = None;
                                                    let mut text: Option<u64> = None;
            
                                                    if let PsbValue::String(character_entry) = &text_list.values()[0] {
                                                        character = Some(character_entry.ref_index());
                                                    }
            
                                                    if let PsbValue::String(character_display_entry) = &text_list.values()[1] {
                                                        character_display = Some(character_display_entry.ref_index());
                                                    }
            
                                                    if let PsbValue::String(text_entry) = &text_list.values()[2] {
                                                        text = Some(text_entry.ref_index());
                                                    }
            
                                                    if text.is_none() {
                                                        continue;
                                                    }
            
                                                    if character.is_some() && !characters.contains(&character.unwrap()) {
                                                        characters.push(character.unwrap());
                                                    }
            
                                                    if character_display.is_some() && !characters_display.contains(&character_display.unwrap()) {
                                                        characters_display.push(character_display.unwrap());
                                                    }
            
                                                    scripts.push((character, character_display, text.unwrap()));
                                                }
            
                                                _ => {}
                                            }
                                        }
                                    } else {
                                        panic!("texts entry is invalid")
                                    }
                                }
                            }
                        }
                    }
                } else {
                    panic!("scenes entry is invalid")
                }
            }

            None => {
                for i in 0..ref_table.strings().len() {
                    strings.push(i as u64);
                }
            }
        }
    } else {
        panic!("Input file is invalid scn file")
    }

    writeln!(output_writer, "[info]").unwrap();
    for title in titles.iter() {
        let title_str = ref_table.get_string(*title as usize).expect(&format!("Cannot find string reference # {}", title));
        writeln!(output_writer, "{} = {}", *title, Value::String(title_str.clone())).unwrap();
    }
    writeln!(output_writer).unwrap();

    writeln!(output_writer, "[characters]").unwrap();
    for character in characters.iter() {
        let character_str = ref_table.get_string(*character as usize).expect(&format!("Cannot find string reference # {}", character));

        writeln!(output_writer, "{} = {}", *character, Value::String(character_str.clone())).unwrap();
    }
    writeln!(output_writer).unwrap();

    writeln!(output_writer, "[character_subs]").unwrap();
    for character_sub in characters_display.iter() {
        let character_sub_str = ref_table.get_string(*character_sub as usize).expect(&format!("Cannot find string reference # {}", character_sub));

        writeln!(output_writer, "{} = {}", *character_sub, Value::String(character_sub_str.clone())).unwrap();
    }
    writeln!(output_writer).unwrap();

    let mut used_texts: Vec::<u64> = Vec::new();

    writeln!(output_writer, "[script]").unwrap();
    writeln!(output_writer, "# lines: {}\n", scripts.len()).unwrap();
    for (character, character_sub, text) in scripts.iter() {
        if character.is_some() {
            let character = character.unwrap();
            let character_str = ref_table.get_string(character as usize).expect(&format!("Cannot find string reference # {}", character));

            writeln!(output_writer, "# {}", character_str).unwrap();
        } else {
            writeln!(output_writer, "# monologue").unwrap();
        }

        if character_sub.is_some() {
            let character_sub = character_sub.unwrap();
            let character_sub_str = ref_table.get_string(character_sub as usize).expect(&format!("Cannot find string reference # {}", character_sub));

            writeln!(output_writer, "# display as {}", character_sub_str).unwrap();
        }

        let text_str = ref_table.get_string(*text as usize).expect(&format!("Cannot find string reference # {}", text));

        writeln!(output_writer, "# {}", text_str).unwrap();
        if used_texts.contains(text) {
            write!(output_writer, "# ").unwrap();

        } else {
            used_texts.push(*text);
        }

        writeln!(output_writer, "{} = {}\n", *text, Value::String(text_str.clone())).unwrap();
    }

    if strings.len() > 0 {
        writeln!(output_writer, "[strings]").unwrap();
        writeln!(output_writer, "# count: {}\n", strings.len()).unwrap();

        for string_id in strings.iter() {
            let string = ref_table.get_string(*string_id as usize).expect(&format!("Cannot find string reference # {}", string_id));
    
            writeln!(output_writer, "{} = {}", *string_id, Value::String(string.clone())).unwrap();
        }
    }
}