/*
 * Created on Fri Jan 15 2021
 *
 * Copyright (c) storycraft. Licensed under the MIT Licence.
 */

use std::{env, fs::{self, File}, io::{BufWriter, Cursor, Read}, path::Path};

use emote_psb::PsbReader;

pub fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("Usage: {} <scn_file> [output_json_file]", args[0]);
        return;
    }

    let input_path = Path::new(&args[1]);
    let input_name = input_path.file_stem().unwrap().to_str().unwrap();

    let default_output_name = {
        let mut name = String::from(input_path.parent().unwrap().join(input_name).to_str().unwrap());
        name.push_str(".json");
        name
    };

    let output_path = Path::new(args.get(2).unwrap_or(&default_output_name));

    fs::create_dir_all(output_path.parent().unwrap()).expect("Cannot create output directory");

    let output_file = File::create(output_path).expect("Cannot create output file");

    let mut psb_file = PsbReader::open_psb({
        let mut file = File::open(input_path).expect(&format!("Cannot open {}", input_path.to_str().unwrap()));
        let mut mem = Vec::new();
        file.read_to_end(&mut mem).expect("Cannot read input file");

        Cursor::new(mem)
    }).expect("Input file is invalid psb file");
    let psb_root = psb_file.load_root().unwrap();

    serde_json::to_writer_pretty(BufWriter::new(output_file), &psb_root).expect("Cannot write to output file");
}