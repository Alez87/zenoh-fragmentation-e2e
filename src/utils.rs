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
    error::Error,
};
use std::fs::create_dir_all;
use log::{info, warn, error};
use memmap::MmapOptions;

const ROOT_FOLDER: &str = "/tmp";

pub fn get_bytes_from_file(filename: &str, chunk_number: usize, chunk_size: usize) -> Vec<u8> {
    let full_filename = format!("{}/{}", ROOT_FOLDER, filename);
    info!(
        "Getting the file {}, chunk number {}.",
        full_filename, chunk_number
    );
    let mut f = File::open(&full_filename).expect("No file found");
    let metadata = fs::metadata(&full_filename).expect("Unable to read metadata");
    let file_size = metadata.len() as usize;

    let offset: usize = (chunk_number - 1) * chunk_size;
    let real_offset = f.seek(SeekFrom::Start(offset as u64));
    info!(
        "The offset I'd like is {} and the real offset is {:?}.",
        offset, real_offset
    );

    let missing_bytes = file_size - offset;
    let buffer_len: usize = missing_bytes.min(chunk_size);
    info!(
        "File size {}, missing_bytes {}. I create a vector of {} bytes.",
        file_size, missing_bytes, buffer_len
    );
    let mut buffer = vec![0; buffer_len];
    f.read_exact(&mut buffer)
        .expect("Error during file reading [read_exact].");
    buffer
}

pub fn get_metadata_info (
    metadata: String,
    old_selector: String,
) -> Result<(usize, String, usize, usize, String), Box<dyn Error>> {
    info!("\nMetadata {:?}", metadata);

    let metadata_split: Vec<_> = metadata.split(", ").collect();
    let metadata_size: Vec<_> = metadata_split[0].split(": ").collect();
    let metadata_checksum: Vec<_> = metadata_split[1].split(": ").collect();
    let metadata_chunks_number: Vec<_> = metadata_split[2].split(": ").collect();
    let metadata_chunk_size: Vec<_> = metadata_split[3].split(": ").collect();
    let selector_split: Vec<_> = old_selector.split('/').collect();

    let size: usize = match metadata_size[1].parse::<usize>() {
        Ok(s) => {
            info!("File size: {}", s);
            s
        },
        Err(e) => {
            error!("Cannot find the file size.");
            return Err(e.into())
        }
    };
    
    let checksum: String = match metadata_checksum[1].parse::<String>() {
        Ok(s) => {
            info!("File size: {}", s);
            s
        },
        Err(e) => {
            error!("Cannot find the file size.");
            return Err(e.into())
        }
    };
    
    let chunks_number: usize = match metadata_chunks_number[1].parse::<usize>() {
        Ok(s) => {
            info!("Chunks number: {}", s);
            s
        },
        Err(e) => {
            error!("Cannot find the number of chunks.");
            return Err(e.into())
        }
    };

    let chunk_size: usize = match metadata_chunk_size[1].parse::<usize>() {
        Ok(s) => {
            info!("Chunks size: {}", s);
            s
        },
        Err(e) => {
            error!("Cannot find the size of the chunks.");
            return Err(e.into())
        }
    };

    let filename = String::from(selector_split[selector_split.len() - 1]);
    info!("Filename: {}\n", filename);

    Ok((size, checksum, chunks_number, chunk_size, filename))
}


pub fn get_chunks_interval(
    chunks_number: usize,
    chunk_size: usize,
    index_start: usize,
    index_end: usize,
    chunk_index_start: usize,
    chunk_index_end: usize,
) -> Result<(usize, usize), Box<dyn Error>> {
    let mut chunk_start: usize = 0;
    let mut chunk_end: usize = chunks_number;
    info!(
        "Indeces: bytes start-{}, bytes end-{}, chunk_start-{}, chunk_end-{}",
        index_start, index_end, chunk_index_start, chunk_index_end
    );

    if index_start > index_end {
        return Err("Wrong bytes interval specified.".into());
    } else if chunk_index_start > chunk_index_end {
        return Err("Wrong chunks interval specified.".into());
    } else if index_end != 0 {
        chunk_start = index_start / chunk_size + 1;
        let chunk_end_raw = index_end / chunk_size + 1;
        chunk_end = chunk_end_raw.min(chunks_number);
        info!(
            "Bytes decision: chunk start {}, chunk end {}",
            chunk_start, chunk_end
        );
    } else if chunk_index_end != 0 {
        chunk_start = chunk_index_start;
        chunk_end = chunk_index_end.min(chunks_number);
        info!(
            "Chunks decision: chunk start {}, chunk end {}",
            chunk_start, chunk_end
        );
    }

    info!("Chunk_start {}, chunk_end {}", chunk_start, chunk_end);

    Ok((chunk_start, chunk_end))
}

pub fn create_mmap_file(path: String, root_folder_final: &str, size: u64) -> Result<File, Box<dyn Error>> {
    let mut f = match OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(path.clone()) {
            Ok(f) => f,
            Err(_) => {
                warn!("Cannot create the file {}.", path);
                warn!("Checking if the folder {} exists.", root_folder_final);
                create_dir_all(&root_folder_final)?;
                info!("Created the folder {}.", root_folder_final);
                match OpenOptions::new()
                    .read(true)
                    .write(true)
                    .create(true)
                    .open(path.clone()) {
                    Ok(f) => f,
                    Err(e) => {
                        error!("Cannot create, again, the file {}.", path);
                        return Err(e.into());
                    }
                }
            }
    };

    // Allocate space in the file first
    f.seek(SeekFrom::Start(size - 1))?;
    f.write_all(&[0])?;
    f.seek(SeekFrom::Start(0))?;
    Ok(f)
}


pub fn write_mmap_file(f: &File, src: Vec<u8>, chunk_num: usize, chunk_size: usize) {
    let mut data = unsafe {
        MmapOptions::new()
            .map_mut(f)
            .expect("Could not access data from memory mapped file")
    };
    let initial_position: usize = (chunk_num - 1) * chunk_size;
    let final_position: usize = initial_position + src.len();
    info!(
        "Write from position {} to position {}.",
        initial_position, final_position
    );
    data[initial_position..final_position].copy_from_slice(&src);
}
    

pub fn write_file(root_folder_chunks: &str, all_bytes: Vec<u8>, filename: String) -> Result<(), Box<dyn Error>> {
    //let mut f = File::create(filename.clone()).expect("Unable to create file");
    let mut f = match File::create(filename.clone()) {
        Ok(f) => f,
        Err(_) => {
            warn!("Cannot create the file {}.", filename);
            warn!("Checking if the folder {} exists...", root_folder_chunks);
            create_dir_all(root_folder_chunks)?;
            warn!("Created the folder {}.", root_folder_chunks);
            match File::create(filename.clone()) {
                Ok(f) => f,
                Err(e) => {
                    error!("Cannot create, again, the file {}.", filename);
                    return Err(e.into());
                }
            }
        }
    };
    f.write_all(&all_bytes).expect("Unable to write data");
    info!("Created file: {:?}", filename);
    Ok(())
}

pub fn check_checksum(checksum_old: String, file: &str) -> bool {
    let checksum_new = checksums::hash_file(Path::new(file), checksums::Algorithm::SHA2256);
    info!("\nChecksum old: {}", checksum_old);
    info!("Checksum new: {}", checksum_new);
    checksum_old.eq(&checksum_new)
}