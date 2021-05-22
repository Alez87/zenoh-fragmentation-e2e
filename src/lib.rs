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
use std::{fs, path::Path};
use std::{
    convert::{TryFrom, TryInto},
    u64, 
    str,
};
use zenoh::*;
use futures::prelude::*;
use std::fs::copy;
use log::{info, warn, error};
use std::io::ErrorKind;
use std::error::Error;

const ROOT_FOLDER: &str = "/tmp";

pub struct PUTApiArgs {
    pub chunk_size: usize
 }

pub struct GETApiChunksArgs {
    pub index_start: usize,
    pub index_end: usize,
    pub chunk_index_start: usize,
    pub chunk_index_end: usize
 }

pub struct GETApiFoldersArgs {
    pub root_folder_final: &'static str,
    pub root_folder_chunks: &'static str
 }
 pub struct EVALApiArgs {
    pub chunk_size: usize
 }

pub struct ZenohCdn {
    pub config: Properties,
    pub zenoh: Zenoh
 }

impl ZenohCdn {

    pub async fn new(config: Properties) -> ZResult<ZenohCdn> {
        info!("New zenoh...");
        let zenoh = Zenoh::new(config.clone().into()).await?;

        Ok(ZenohCdn {config, zenoh})
    }

    /// Returns the config that was used to create this Zenoh_cdn.
    pub fn config(&self) -> &Properties {
        &self.config
    }

    /// Returns Zenoh from Zenoh_cdn.
    pub fn get_zenoh(&self) -> &Zenoh {
        &self.zenoh
    }

    /// The API to share a file.
    pub async fn put_e2e(&self, path: String, value: String, args: PUTApiArgs) -> Result<(), Box<dyn Error>> {        
        let chunk_size: usize = check_put_args(&path, &value, args)?;
        info!("New workspace...");
        let workspace = self.zenoh.workspace(None).await?;

        let file_metadata = match fs::metadata(&value) {
            Ok(metadata) => metadata,
            Err(e) => { 
                error!("Unable to read metadata.");
                return Err(e.into()); 
            }
        };
        let file_size = file_metadata.len() as usize;

        if file_size <= chunk_size {
            info!("Put Data ('{}': '{}')...\n", path, value);
            workspace.put(&path.try_into()?, value.into()).await?;
        } else {
            let path_split: Vec<_> = path.split('/').collect();
            let filename = path_split[path_split.len() - 1];
            let source = value.clone();
            let destination = format!("{}/{}", ROOT_FOLDER, filename);
            match copy(source.clone(), destination.clone()) {
                Ok(_) => info!("Copied file from {} to {}.", source, destination),
                Err(e) => {
                    info!("Cannot copy the file from {} to {}.", source, destination);
                    return Err(e.into());
                }
            };
            let file_type = file_metadata.file_type();
            info!("File size: {}", file_size);
            info!("File type: {:?}", file_type);

            let input = Path::new(&value);
            let checksum = checksums::hash_file(input, checksums::Algorithm::SHA2256);
            info!("Checksum: {:?}", checksum);

            let chunks_number: usize = file_size / chunk_size + 1;
            let metadata_path: String = format!("{}/metadata", path);
            let metadata: String = format!(
                "size: {}, checksum: {}, chunks_number: {}, chunk_size: {}, file_type: {:?}",
                file_size, checksum, chunks_number, chunk_size, file_type
            );
            info!("Selector: {}", metadata_path);
            info!("Size metadata: {}", metadata.len());
            workspace.put(&metadata_path.try_into()?, metadata.into()).await?;

            let chunks_nums: Vec<_> = (1..=chunks_number).map(|i| i).collect();
            call_eval(path, chunks_nums, chunk_size).await;
        }
        Ok(())
    }


    /// The API to retrieve a shared file
    pub async fn get_e2e (&self, selector: String, folder_args: GETApiFoldersArgs, bytes_args: GETApiChunksArgs) -> Result<String, Box<dyn Error>> {
        let _ = env_logger::try_init();
        
        check_get_args(selector.clone())?;
        /*info!("New zenoh...");
        let zenoh = Zenoh::new(config.into()).await?;
*/
        info!("New workspace...");
        let workspace = self.zenoh.workspace(None).await?;

        let old_selector = selector.clone();
        info!("Get Data from {}'...\n", selector);
        let mut data_stream = workspace.get(&selector.try_into()?).await?;

        let mut found_selector = false;
        while let Some(data) = data_stream.next().await {
            found_selector = true;
            info!(
                "  {} : {:?} (encoding: {} , timestamp: {})",
                data.path,
                data.value,
                data.value.encoding_descr(),
                data.timestamp
            )
        }
        let mut path_to_return = "".to_string();
        if !found_selector {
            let metadata_selector = format!("{}/metadata", old_selector);
            info!("Metadata selector: {}", metadata_selector);
            let mut data_stream = workspace.get(&metadata_selector.try_into()?).await?;
            let mut metadata: String = String::from("");
            while let Some(data) = data_stream.next().await {
                metadata = match data.value {
                    Value::StringUtf8(s) => s,
                    _ => {
                            error!("Cannot read the data [StringUtf8 expected]."); 
                            return Err("Cannot read the data [StringUtf8 expected].".into())
                        },
                };
            }

            if metadata.is_empty() {
                return Err(std::io::Error::new(ErrorKind::NotFound, "Metadata information not found.").into());
            }

            let (size, checksum, chunks_number, chunk_size, filename) =
                get_metadata_info(metadata, old_selector.clone())?;

            let (chunk_start, chunk_end) = get_chunks_interval(
                chunks_number,
                chunk_size,
                bytes_args.index_start,
                bytes_args.index_end,
                bytes_args.chunk_index_start,
                bytes_args.chunk_index_end,
            )?;

            let path = format!("{}/{}", folder_args.root_folder_final, &filename);
            path_to_return = path.clone();
            let final_file = create_mmap_file(path.clone(), folder_args.root_folder_final, size as u64)?;

            for chunk_num in chunk_start..chunk_end + 1 {
                let chunk_selector = format!("{}/{}", old_selector, chunk_num);
                info!(
                    "\nElaborating chunk number {}. Calling EVAL {}.",
                    chunk_num, chunk_selector
                );
                let mut data_stream = workspace
                    .get(&chunk_selector.try_into()?).await?;
                while let Some(data) = data_stream.next().await {
                    let chunk_content: RBuf = match data.value.clone() {
                        Value::Raw(_, buff) => buff,
                        _ => {
                                error!("Not the data expected [RBuff required]."); 
                                return Err("Not the data expected [RBuff required].".into())
                            },
                    };
                    let filename_num = format!("{}_{}", &filename, chunk_num);
                    let full_filename = format!("{}/{}", folder_args.root_folder_chunks, filename_num);
                    write_mmap_file(&final_file, chunk_content.to_vec(), chunk_num, chunk_size);
                    write_file(folder_args.root_folder_chunks, chunk_content.to_vec(), full_filename)?;                
                }
            }
            let count_chunks = chunk_end - chunk_start + 1;
            if count_chunks == chunks_number {
                let checksum_ok = check_checksum(checksum, &path);
                if !checksum_ok {
                    error!("Checksum verified -> ERROR. Please try to download the file again.");
                } else {
                    info!("Checksum verified -> OK");
                }
            } else {
                warn!("{} chunks missing. Check them to recrete the whole file.", count_chunks);
            }

            let chunks_nums: Vec<_> = (chunk_start..=chunk_end).map(|i| i).collect();
            call_eval(path.clone(), chunks_nums, chunk_size).await;
        }
        Ok(path_to_return)
    }
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
    info!("Running Eval {} on path {} with config {:?}", chunk_number, eval_path, config
    );
    let _ = match run_eval_e2e(config.clone(), eval_path, EVALApiArgs{chunk_size}).await {
        Ok(_) => info!("Finished Eval {}", chunk_number),
        Err(e) => error!("Error during the Eval: {}.", e)
    };
}

/// The API to retrieve bytes related the chunks
pub async fn run_eval_e2e(config: Properties, path_str: String, args: EVALApiArgs) -> Result<(), Box<dyn Error>> {
    let _ = env_logger::try_init();
    
    let chunk_size: usize = check_eval_args(path_str.clone(), args)?;
    let path: zenoh::Path = zenoh::Path::try_from(path_str.clone())?;
    let path_expr = PathExpr::try_from(path_str.clone())?;

    info!("New zenoh...");
    let zenoh = Zenoh::new(config.into()).await?;

    info!("New workspace...");
    let workspace = zenoh.workspace(None).await?;

    info!("Register eval for {}'...\n", path_str);
    let mut get_stream = workspace.register_eval(&path_expr).await?;
    while let Some(get_request) = get_stream.next().await {
        let selector = get_request.selector.clone();
        info!(">> [Eval listener] received get with selector: {}", selector);

        let selector_to_split = format!("{}", selector);
        let selector_split: Vec<_> = selector_to_split.split('/').collect();
        let filename = selector_split[selector_split.len() - 2];
        let chunk_number = match selector_split[selector_split.len() - 1].parse::<usize>() {
            Ok(chunk_number)  => chunk_number,
            Err(e) => {
                error!("Chunk number not a valid number: {}.", e);
                return Err(e.into());
            }
        };

        let chunk_bytes: Vec<u8> = get_bytes_from_file(filename, chunk_number, chunk_size);
        info!(r#"Replying to GET "{:02X?}""#, &chunk_bytes[0..100]);
        get_request.reply(path.clone(), chunk_bytes.into()).await;
    }
    get_stream.close().await?;
    zenoh.close().await?;
    Ok(())
}