/*
 * Created on Fri Jan 15 2021
 *
 * Copyright (c) storycraft. Licensed under the MIT Licence.
 */

use std::{env, fs::{self, File}, io::{BufReader, BufWriter}, path::Path};

use emote_psb::{PsbWriter, VirtualPsb, header::PsbHeader, types::collection::PsbObject};

pub fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("Usage: {} <json_file> [output_scn_file]", args[0]);
        return;
    }

    let input_path = Path::new(&args[1]);
    let input_name = input_path.file_stem().unwrap().to_str().unwrap();

    let default_output_name = {
        let mut name = String::from(input_path.parent().unwrap().join(input_name).to_str().unwrap());
        name.push_str(".scn");
        name
    };

    let output_path = Path::new(args.get(2).unwrap_or(&default_output_name));

    fs::create_dir_all(output_path.parent().unwrap()).expect("Cannot create output directory");

    let output_file = File::create(output_path).expect("Cannot create output file");

    let psb_root: PsbObject = serde_json::from_reader(BufReader::new(File::open(input_path).unwrap())).expect("Cannot read input file");

    PsbWriter::new(
        VirtualPsb::new(PsbHeader { version: 2, encryption: 0 }, Vec::new(), Vec::new(), psb_root),
        BufWriter::new(output_file)
    ).finish().expect("Cannot write output file");
}