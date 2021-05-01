//
// Copyright (c) 2017, 2020 ADLINK Technology Inc.
//
// This program and the accompanying materials are made available under the
// terms of the Eclipse Public License 2.0 which is available at
// http://www.eclipse.org/legal/epl-2.0, or the Apache License, Version 2.0
// which is available at https://www.apache.org/licenses/LICENSE-2.0.
//
// SPDX-License-Identifier: EPL-2.0 OR Apache-2.0
//
// Contributors:
//   ADLINK zenoh team, <zenoh@adlink-labs.tech>
//

use std::fs;
use std::fs::File;
use std::path::Path;
use std::str;
use std::{
    fs::OpenOptions,
    io::{Seek, SeekFrom},
};
use std::{
    io::{Read, Write},
    u64,
};

const ROOT_FOLDER: &str = "/tmp";

pub fn get_bytes_from_file(filename: &str, chunk_number: usize, chunk_size: usize) -> Vec<u8> {
    let full_filename = format!("{}/{}", ROOT_FOLDER, filename);
    println!(
        "Getting the file {}, chunk number {}.",
        full_filename, chunk_number
    );
    let mut f = File::open(&full_filename).expect("No file found");
    let metadata = fs::metadata(&full_filename).expect("Unable to read metadata");
    let file_size = metadata.len() as usize;

    let offset: usize = (chunk_number - 1) * chunk_size;
    let real_offset = f.seek(SeekFrom::Start(offset as u64));
    println!(
        "The offset I'd like is {} and the real offset is {:?}.",
        offset, real_offset
    );

    let missing_bytes = file_size - offset;
    let buffer_len: usize = missing_bytes.min(chunk_size);
    println!(
        "File size {}, missing_bytes {}. I create a vector of {} bytes.",
        file_size, missing_bytes, buffer_len
    );
    let mut buffer = vec![0; buffer_len];
    f.read_exact(&mut buffer)
        .expect("Error during file reading [read_exact].");
    buffer
}

pub fn get_metadata_info(
    metadata: String,
    old_selector: String,
) -> (usize, String, usize, usize, String) {
    println!("\nMetadata {:?}", metadata);

    let metadata_split: Vec<_> = metadata.split(", ").collect();
    let metadata_size: Vec<_> = metadata_split[0].split(": ").collect();
    let metadata_checksum: Vec<_> = metadata_split[1].split(": ").collect();
    let metadata_chunks_number: Vec<_> = metadata_split[2].split(": ").collect();
    let metadata_chunk_size: Vec<_> = metadata_split[3].split(": ").collect();
    let selector_split: Vec<_> = old_selector.split('/').collect();

    let size: usize = metadata_size[1].parse::<usize>().unwrap();
    let checksum: String = metadata_checksum[1].parse::<String>().unwrap();
    let chunks_number: usize = metadata_chunks_number[1].parse::<usize>().unwrap();
    let chunk_size: usize = metadata_chunk_size[1].parse::<usize>().unwrap();
    let filename = String::from(selector_split[selector_split.len() - 1]);
    println!("\nFile size: {}", size);
    println!("Checksum: {}", checksum);
    println!("Chunks_number: {}", chunks_number);
    println!("Chunk size: {}", chunk_size);
    println!("Filename: {}\n", filename);

    (size, checksum, chunks_number, chunk_size, filename)
}

pub fn get_chunks_interval(
    chunks_number: usize,
    chunk_size: usize,
    index_start: String,
    index_end: String,
    chunk_index_start: String,
    chunk_index_end: String,
) -> (usize, usize) {
    let mut chunk_start: usize = 0;
    let mut chunk_end: usize = chunks_number;
    println!(
        "Indeces: bytes start-{}, bytes end-{}, chunk_start-{}, chunk_end-{}",
        index_start, index_end, chunk_index_start, chunk_index_end
    );

    let index_start_num = index_start.parse::<usize>().unwrap();
    let index_end_num = index_end.parse::<usize>().unwrap();
    let chunk_index_start_num = chunk_index_start.parse::<usize>().unwrap();
    let chunk_index_end_num = chunk_index_end.parse::<usize>().unwrap();

    if index_start_num > index_end_num {
        panic!("Wrong bytes interval specified.");
    } else if chunk_index_start_num > chunk_index_end_num {
        panic!("Wrong chunks interval specified.");
    } else if index_end_num != 0 {
        chunk_start = index_start_num / chunk_size + 1;
        let chunk_end_raw = index_end_num / chunk_size + 1;
        chunk_end = chunk_end_raw.min(chunks_number);
        println!(
            "Bytes decision: chunk start {}, chunk end {}",
            chunk_start, chunk_end
        );
    } else if chunk_index_end_num != 0 {
        chunk_start = chunk_index_start_num;
        chunk_end = chunk_index_end_num.min(chunks_number);
        println!(
            "Chunks decision: chunk start {}, chunk end {}",
            chunk_start, chunk_end
        );
    }
    (chunk_start, chunk_end)
}

pub fn create_mmap_file(path: String, size: u64) -> File {
    let mut f = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(path)
        .expect("Unable to open file");

    // Allocate space in the file first
    f.seek(SeekFrom::Start(size - 1)).unwrap();
    f.write_all(&[0]).unwrap();
    f.seek(SeekFrom::Start(0)).unwrap();
    f
}

pub fn write_mmap_file(f: &File, src: Vec<u8>, chunk_num: usize, chunk_size: usize) {
    let mut data = unsafe {
        memmap::MmapOptions::new()
            .map_mut(f)
            .expect("Could not access data from memory mapped file")
    };

    let initial_position: usize = (chunk_num - 1) * chunk_size;
    let final_position: usize = initial_position + src.len();
    //data[..src.len()].copy_from_slice(&src);
    println!(
        "Write from position {} to position {}.",
        initial_position, final_position
    );
    data[initial_position..final_position].copy_from_slice(&src);
}

/*
fn write_file(all_bytes: Vec<u8>, filename: String) -> () {
    //let full_filename = format!("{}/{}", ROOT_FOLDER_CHUNKS, filename_num);
    let mut f = File::create(filename.clone()).expect("Unable to create file");
    f.write_all(&all_bytes).expect("Unable to write data");
    println!("Created file: {:?}", filename);
}
*/

pub fn check_checksum(checksum_old: String, file: &str) -> bool {
    let checksum_new = checksums::hash_file(Path::new(file), checksums::Algorithm::SHA2256);
    println!("\nChecksum old: {}", checksum_old);
    println!("Checksum new: {}", checksum_new);
    checksum_old.eq(&checksum_new)
}
