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

mod utils;
use utils::*;

use net::RBuf;
use std::fs;
use std::path::Path;
use std::str;
use std::{
    convert::{TryFrom, TryInto},
    u64,
};
use zenoh::*;
use futures::prelude::*;

const ROOT_FOLDER: &str = "/tmp";

/// The API to share a file.
pub async fn put_e2e(config: Properties, path: String, value: String, chunk_size: usize) {
    println!("New zenoh...");
    let zenoh = Zenoh::new(config.into()).await.unwrap();

    println!("New workspace...");
    let workspace = zenoh.workspace(None).await.unwrap();

    let file_metadata = fs::metadata(&value).expect("unable to read metadata");
    let file_size = file_metadata.len() as usize;

    if file_size <= chunk_size {
        println!("Put Data ('{}': '{}')...\n", path, value);
        workspace
            .put(&path.try_into().unwrap(), value.into())
            .await
            .unwrap();
    } else {
        let path_split: Vec<_> = path.split('/').collect();
        let filename = path_split[path_split.len() - 1];
        let source = value.clone();
        let destination = format!("{}/{}", ROOT_FOLDER, filename);
        std::fs::copy(source, destination).expect("Cannot copy the file.");
        println!(
            "Copied file from {} to {}.",
            value,
            format!("{}/{}", ROOT_FOLDER, filename)
        );

        let file_type = file_metadata.file_type();
        println!("\nFile size: {}", file_size);
        println!("File type: {:?}", file_type);

        let input = Path::new(&value);
        let checksum = checksums::hash_file(input, checksums::Algorithm::SHA2256);
        println!("Checksum: {:?}", checksum);

        let chunks_number: usize = file_size / chunk_size + 1;
        let metadata_path: String = format!("{}/metadata", path);
        let metadata: String = format!(
            "size: {}, checksum: {}, chunks_number: {}, chunk_size: {}, file_type: {:?}",
            file_size, checksum, chunks_number, chunk_size, file_type
        );
        println!("Selector: {}", metadata_path);
        println!("Size metadata: {}", metadata.len());
        workspace
            .put(&metadata_path.try_into().unwrap(), metadata.into())
            .await
            .unwrap();

        let chunks_nums: Vec<_> = (1..=chunks_number).map(|i| i).collect();
        call_eval(path, chunks_nums, chunk_size).await;
    }
    zenoh.close().await.unwrap();
}

/// The API to retrieve a shared file
pub async fn get_e2e(
    config: Properties,
    selector: String,
    /*root_folder_chunks: &str,*/ root_folder_final: String,
    index_start: String,
    index_end: String,
    chunk_index_start: String,
    chunk_index_end: String,
) {
    println!("New zenoh...");
    let zenoh = Zenoh::new(config.into()).await.unwrap();

    println!("New workspace...");
    let workspace = zenoh.workspace(None).await.unwrap();

    let old_selector = selector.clone();
    println!("Get Data from {}'...\n", selector);
    let mut data_stream = workspace.get(&selector.try_into().unwrap()).await.unwrap();

    let mut found_selector = false;
    while let Some(data) = data_stream.next().await {
        found_selector = true;
        println!(
            "  {} : {:?} (encoding: {} , timestamp: {})",
            data.path,
            data.value,
            data.value.encoding_descr(),
            data.timestamp
        )
    }
    if !found_selector {
        let metadata_selector = format!("{}/metadata", old_selector);
        println!("Metadata selector: {}", metadata_selector);
        let mut data_stream = workspace
            .get(&metadata_selector.try_into().unwrap())
            .await
            .unwrap();
        let mut metadata: String = "".to_string();
        while let Some(data) = data_stream.next().await {
            metadata = match data.value {
                Value::StringUtf8(s) => s,
                _ => panic!("Error"),
            };
        }

        let (size, checksum, chunks_number, chunk_size, filename) =
            get_metadata_info(metadata, old_selector.clone());

        let (chunk_start, chunk_end) = get_chunks_interval(
            chunks_number,
            chunk_size,
            index_start,
            index_end,
            chunk_index_start,
            chunk_index_end,
        );

        let path = format!("{}/{}", root_folder_final, &filename);
        let final_file = create_mmap_file(path.clone(), size as u64);

        for chunk_num in chunk_start..chunk_end + 1 {
            let chunk_selector = format!("{}/{}", old_selector, chunk_num);
            println!(
                "\nElaborating chunk number {}. Calling EVAL {}.",
                chunk_num, chunk_selector
            );
            let mut data_stream = workspace
                .get(&chunk_selector.try_into().unwrap())
                .await
                .unwrap();
            while let Some(data) = data_stream.next().await {
                let chunk_content: RBuf = match data.value.clone() {
                    Value::Raw(_, buff) => buff,
                    _ => panic!("Not the data expected."),
                };
                //let filename_num = format!("{}_{}", &filename, chunk_num);
                //let full_filename = format!("{}/{}", root_folder_chunks, filename_num);
                //write_file(chunk_content.to_vec(), full_filename);
                write_mmap_file(&final_file, chunk_content.to_vec(), chunk_num, chunk_size);
                //start_eval(): quando ho il file intero o quando ho un chunk?
                //se quando ho il chunk allora come gestisco quando il chunk size cambia?
            }
        }

        let count_chunks = chunk_end - chunk_start + 1;
        if count_chunks == chunks_number {
            let checksum_ok = check_checksum(checksum, &path);
            if !checksum_ok {
                println!("Checksum verified -> ERROR. Please try to download the file again.");
            } else {
                println!("Checksum verified -> OK");
            }
        } else {
            println!(
                "{} chunks missing. Check them to recrete the whole file.",
                count_chunks
            );
        }
    }
    zenoh.close().await.unwrap();
}

/// The API to retrieve bytes related the chunks
pub async fn run_eval_e2e(config: Properties, path_str: String, chunk_size: usize) {
    let path: zenoh::Path = zenoh::Path::try_from(path_str.clone()).unwrap();
    let path_expr = PathExpr::try_from(path_str.clone()).unwrap();

    println!("New zenoh...");
    let zenoh = Zenoh::new(config.into()).await.unwrap();

    println!("New workspace...");
    let workspace = zenoh.workspace(None).await.unwrap();

    println!("Register eval for {}'...\n", path_str);
    let mut get_stream = workspace.register_eval(&path_expr).await.unwrap();
    while let Some(get_request) = get_stream.next().await {
        let selector = get_request.selector.clone();
        println!(
            ">> [Eval listener] received get with selector: {}",
            selector
        );

        let selector_to_split = format!("{}", selector);
        let selector_split: Vec<_> = selector_to_split.split('/').collect();
        let filename = selector_split[selector_split.len() - 2];
        let chunk_number = selector_split[selector_split.len() - 1]
            .parse::<usize>()
            .unwrap();

        let chunk_bytes: Vec<u8> = get_bytes_from_file(filename, chunk_number, chunk_size);
        println!(r#"Replying to GET "{:02X?}""#, &chunk_bytes[0..100]);
        get_request.reply(path.clone(), chunk_bytes.into()).await;
    }
    get_stream.close().await.unwrap();
    zenoh.close().await.unwrap();
}

pub async fn call_eval(path: String, chunks_nums: Vec<usize>, chunk_size: usize) {
    let mut tasks = Vec::with_capacity(chunks_nums.len());
    for n in chunks_nums {
        let path_eval = path.to_string();
        tasks.push(async_std::task::spawn(async move {
            eval(path_eval, n, chunk_size).await;
        }));
    }
    for task in tasks {
        task.await;
    }
}

async fn eval(path: String, chunk_number: usize, chunk_size: usize) {
    let mut config = Properties::default();
    config.insert("-m".to_string(), "peer".to_string());
    let eval_path = format!("{}/{}", path, chunk_number);
    println!(
        "\nRunning Eval {} on path {} with config {:?}",
        chunk_number, eval_path, config
    );
    run_eval_e2e(config.clone(), eval_path, chunk_size).await;
    println!("\nFinished Eval {}", chunk_number);
}
