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
use futures::future::{AbortHandle, Abortable};
use utils::*;

use net::RBuf;
use std::{fs, path::Path, sync::Arc};
use std::{
    convert::{TryFrom, TryInto},
    u64, 
    str,
};
use zenoh::*;
use futures::{prelude::*, select};
use std::fs::copy;
use log::{info, warn, error};
use std::io::ErrorKind;
use std::error::Error;

const ROOT_FOLDER: &str = "/tmp";
const MSG_FILE_RECEIVED: &str = "OK";

#[derive(Clone, Copy)]
pub struct PUTApiArgs {
    pub chunk_size: usize
 }

#[derive(Clone)]
pub struct GETApiChunksArgs {
    pub index_start: usize,
    pub index_end: usize,
    pub chunk_index_start: usize,
    pub chunk_index_end: usize
 }

#[derive(Clone)]
pub struct GETApiFoldersArgs {
    pub root_folder_final: &'static str,
    pub root_folder_chunks: &'static str
 }
 pub struct EVALApiArgs {
    pub chunk_size: usize
 }

 #[derive(Clone)]
  pub struct ZenohCdn {
    pub zenoh: Arc<Zenoh>,
    upload_args: PUTApiArgs,
    download_folders: GETApiFoldersArgs, 
    download_bytes_args: GETApiChunksArgs
 }

 impl Default for crate::PUTApiArgs {
    fn default() -> Self { 
       Self {
           chunk_size: 65_000
       }
   }
}

impl Default for crate::GETApiChunksArgs {
   fn default() -> Self { 
       Self {
           index_start: 0,
           index_end: 0,
           chunk_index_start: 0,
           chunk_index_end: 0
      }
  }
}

impl Default for crate::GETApiFoldersArgs {
   fn default() -> Self { 
       Self {
           root_folder_final: "/tmp/final",
           root_folder_chunks: "/tmp/chunks"
      }
  }
}

impl Default for crate::EVALApiArgs {
   fn default() -> Self { 
       Self {
          chunk_size: 65_000
      }
  }
}

impl ZenohCdn {

    /// Creates a ZenohCDN object from an existing Zenoh session.
    pub async fn new(zenoh: Arc<Zenoh>) -> ZResult<ZenohCdn> {
        let upload_args = PUTApiArgs::default();
        let download_folders = GETApiFoldersArgs::default();
        let download_bytes_args = GETApiChunksArgs::default();

        Ok(ZenohCdn {zenoh, upload_args, download_folders, download_bytes_args})
    }

    /// Creates a ZenohCDN object, starting a new Zenoh session.
    pub async fn new_session(config: Properties) -> ZResult<ZenohCdn> {
        info!("New zenoh...");
        let zenoh = Arc::new(Zenoh::new(config.into()).await?);
        let upload_args = PUTApiArgs::default();
        let download_folders = GETApiFoldersArgs::default();
        let download_bytes_args = GETApiChunksArgs::default();

        Ok(ZenohCdn {zenoh, upload_args, download_folders, download_bytes_args})
    }

    /// Returns Zenoh from Zenoh_cdn.
    pub fn get_zenoh(&self) -> Arc<Zenoh> {
        self.zenoh.clone()
    }

    /// Get a reference to the zenoh cdn's folder download folders.
    pub fn download_folders(&self) -> &GETApiFoldersArgs {
        &self.download_folders
    }

    /// Set the zenoh cdn's download folders.
    pub fn set_download_folders(&mut self, download_folders: GETApiFoldersArgs) {
        self.download_folders = download_folders;
    }

    /// Set the zenoh cdn's folder where to download the final file to share.
    pub fn set_download_file_folder(&mut self, download_folder_final: &'static str) {
        let folders: &GETApiFoldersArgs = self.download_folders();
        let root_folder_final = download_folder_final;
        let root_folder_chunks =  folders.root_folder_chunks;
        self.set_download_folders(GETApiFoldersArgs{root_folder_final, root_folder_chunks});
    }

    /// Set the zenoh cdn's folder where to download the file chunks to share.
    pub fn set_download_chunks_folder(&mut self, download_folder_chunks: &'static str) {
        let folders: &GETApiFoldersArgs = self.download_folders();
        let root_folder_final = folders.root_folder_final;
        let root_folder_chunks = download_folder_chunks;
        self.set_download_folders(GETApiFoldersArgs{root_folder_final, root_folder_chunks});
    }

    /// Get a reference to the zenoh cdn's download bytes args.
    pub fn download_bytes_args(&self) -> &GETApiChunksArgs {
        &self.download_bytes_args
    }

    /// Set the zenoh cdn's download bytes args.
    pub fn set_download_bytes_args(&mut self, download_bytes_args: GETApiChunksArgs) {
        self.download_bytes_args = download_bytes_args;
    }

    /// Get a reference to the zenoh cdn's upload args.
    pub fn upload_args(&self) -> &PUTApiArgs {
        &self.upload_args
    }

    /// Set the zenoh cdn's upload args.
    pub fn set_upload_args(&mut self, upload_args: PUTApiArgs) {
        self.upload_args = upload_args;
    }

    /// API to send a file in a client-server fashion.
    pub async fn send(&self, path: String, value: String) -> Result<(), Box<dyn Error>> {
        let same_path = path.clone();
        let chunk_size: usize = check_put_args(&path, &value, self.upload_args)?;
        
        let (filename, chunks_number) = self.share_file(path, value, chunk_size).await?;
        let selector = filename.clone();
        
        println!("Chunk number: {}", chunks_number);
        let chunks_nums: Vec<usize> = (1..=chunks_number).collect();
        let tasks_abort = Vec::with_capacity(chunks_nums.len());
        let t = self.start_evals(same_path, chunks_nums, chunk_size, tasks_abort).await;
        
        //notify that I've shared a file with the pub api
        let workspace = self.zenoh.workspace(None).await.unwrap();
        let key = "New_file";
        let value_notification = format!("new file ready to be downloaded. Filename {}", filename);
        info!("I notify that the {} file is ready to be downloaded.", filename);
        workspace.put(&key.try_into()?, value_notification.into()).await?;

        //listen if the file has been uploaded
        info!("I subscribe to selector <{}>.", selector);
        let mut change_stream = workspace.subscribe(&selector.try_into()?).await?;
        loop {
            select!(
                change = change_stream.next().fuse() => {
                    let change = change.unwrap();
                    println!(
                        ">> [Subscription listener] received {:?} for {} : {:?} with timestamp {}",
                        change.kind,
                        change.path,
                        change.value,
                        change.timestamp
                    );
                    match change.value {
                        Some(s) => {
                            match s {
                                Value::StringUtf8(msg) => 
                                    if msg.to_uppercase() == MSG_FILE_RECEIVED {
                                        break;
                                    },
                                _ => {
                                        error!("Cannot read the data [StringUtf8 expected]."); 
                                        return Err("Cannot read the data [StringUtf8 expected].".into())
                                    },
                            };
                        },
                        _ => {
                                error!("Cannot read the change.value."); 
                                return Err("Cannot read the change.value.".into())
                        },
                    };                   
                }
            );
        }
        change_stream.close().await.unwrap();

        self.stop_evals(t).await;
        Ok(())
    }

    /// API to share a file. 
    pub async fn upload(&self, path: String, value: String) -> Result<(), Box<dyn Error>> {
        let chunk_size: usize = check_put_args(&path, &value, self.upload_args)?;
        let (_filename, chunks_number) = self.share_file(path.clone(), value, chunk_size).await?;
        let chunks_nums: Vec<usize> = (1..=chunks_number).collect();
        self.call_eval(path, chunks_nums, chunk_size).await;
        Ok(())
    }

    /// API to share a file, specifying the chunk size. 
    pub async fn upload_extended(&self, path: String, value: String, chunk_size: usize) -> Result<(), Box<dyn Error>> {
        let (_filename, chunks_number) = self.share_file(path.clone(), value, chunk_size).await?;
        let chunks_nums: Vec<usize> = (1..=chunks_number).collect();
        let tasks_abort = Vec::with_capacity(chunks_nums.len());
        self.start_evals(path, chunks_nums, chunk_size, tasks_abort).await;
        Ok(())
    }

    /// Base method to to share a file. 
    async fn share_file(&self, path: String, value: String, chunk_size: usize) -> Result<(String, usize), Box<dyn Error>> {
        info!("New workspace...");
        let workspace = self.zenoh.workspace(None).await?;

        info!("Value: {}", value);
        let file_metadata = match fs::metadata(&value) {
            Ok(metadata) => metadata,
            Err(e) => { 
                error!("Unable to read metadata from local file {}.", value);
                return Err(e.into()); 
            }
        };
        let file_size = file_metadata.len() as usize;
        let mut chunks_number: usize = 0;
        let path_split: Vec<_> = path.split('/').collect();
        let filename: String = path_split[path_split.len() - 1].to_string();
        if file_size <= chunk_size {
            info!("Put Data ('{}': '{}')...\n", path, value);
            workspace.put(&path.try_into()?, value.into()).await?;
        } else {
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

            chunks_number = file_size / chunk_size + 1;
            info!("Chunks number: {}", chunks_number);

            let metadata_path: String = format!("{}/metadata", path);
            let metadata: String = format!(
                "size: {}, checksum: {}, chunks_number: {}, chunk_size: {}, file_type: {:?}",
                file_size, checksum, chunks_number, chunk_size, file_type
            );
            info!("Selector: {}", metadata_path);
            info!("Size metadata: {}", metadata.len());
            workspace.put(&metadata_path.try_into()?, metadata.into()).await?; 
        }
        Ok((filename, chunks_number))
    }

    /// The API to download a file.
    pub async fn download(&self, selector: String, download_folder_final: &'static str) -> Result<String, Box<dyn Error>> {
        let folders: &GETApiFoldersArgs = self.download_folders();
        let root_folder_chunks = folders.root_folder_chunks;
        let mut root_folder_final = folders.root_folder_final;
        if !download_folder_final.is_empty() {
            root_folder_final = download_folder_final;
        }
        self.retrieve_file(selector, root_folder_final, root_folder_chunks, self.download_bytes_args()).await
    }

    /// The API to download a file, specifying the indexes
    pub async fn download_extended(&self, selector: String, download_folder_final: &str, indexes: Option<&GETApiChunksArgs>) -> Result<String, Box<dyn Error + '_>> { 
        let folders: &GETApiFoldersArgs = self.download_folders();
        let root_folder_chunks = folders.root_folder_chunks;
        let mut root_folder_final = folders.root_folder_final;
        if !download_folder_final.is_empty() {
            root_folder_final = download_folder_final;
        }
        let i: &GETApiChunksArgs = match indexes {
            Some(element) => element,
            None => self.download_bytes_args(),
        };
        self.retrieve_file(selector, root_folder_final, root_folder_chunks, i).await
    }

    /// Base method to retrieve a file.
    async fn retrieve_file(&self, selector: String, root_folder_final: &str, root_folder_chunks: &str, indexes: &GETApiChunksArgs) -> Result<String, Box<dyn Error>> {
        check_get_args(selector.clone())?;

        let index_start: usize = indexes.index_start;
        let index_end: usize = indexes.index_end;
        let chunk_index_start: usize = indexes.chunk_index_start;
        let chunk_index_end: usize = indexes.chunk_index_end;

        info!("New workspace...");
        let workspace = self.zenoh.workspace(None).await?;

        let old_selector = selector.clone();
        info!("Get Data from {}'...\n", selector);
        let mut data_stream = workspace.get(&selector.try_into()?).await?;

        let mut found_selector = false;
        while let Some(_data) = data_stream.next().await {
            found_selector = true;
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
                index_start,
                index_end,
                chunk_index_start,
                chunk_index_end,
            )?;

            let path = format!("{}/{}", root_folder_final, &filename);
            path_to_return = path.clone();
            let final_file = create_mmap_file(path.clone(), root_folder_final, size as u64)?;

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
                    let full_filename = format!("{}/{}", root_folder_chunks, filename_num);
                    write_mmap_file(&final_file, chunk_content.to_vec(), chunk_num, chunk_size);
                    write_file(root_folder_chunks, chunk_content.to_vec(), full_filename)?;                
                }
            }
            let count_chunks = chunk_end - chunk_start;
            if count_chunks == chunks_number {
                let checksum_ok = check_checksum(checksum, &path);
                if !checksum_ok {
                    error!("Checksum verified -> ERROR. Please try to download the file again.");
                } else {
                    info!("Checksum verified -> OK");
                }
            } else {
                warn!("{} chunks missing. Check them to recreate the whole file.", count_chunks);
            }

            let chunks_nums: Vec<_> = (chunk_start..=chunk_end).collect();
            let tasks_abort = Vec::with_capacity(chunks_nums.len());
            self.start_evals(path.clone(), chunks_nums, chunk_size, tasks_abort).await;
        }
        Ok(path_to_return)
    }

    /// Method to run multiple async evals forever.
    pub async fn call_eval(&self, path: String, chunks_nums: Vec<usize>, chunk_size: usize) {
        let mut tasks = Vec::with_capacity(chunks_nums.len());
        for n in chunks_nums {
            let zenoh = self.clone();
            let path_eval = path.to_string();
            tasks.push(async_std::task::spawn(async move {
                zenoh.eval(path_eval, n, chunk_size).await;
            }));
        }
        for task in tasks {
            task.await;
        }
    }

    /// Method to run multiple, stoppable, async evals. 
    async fn start_evals(&self, path: String, chunks_nums: Vec<usize>, chunk_size: usize, mut tasks_abort:Vec<AbortHandle>) -> Vec<AbortHandle> {
        for n in chunks_nums.clone() {
            let (handle, registration) = AbortHandle::new_pair();
            tasks_abort.push(handle);
    
            let zenoh = self.clone();
            let path_eval = path.to_string();
            let fut = async move {
                zenoh.eval(path_eval, n, chunk_size).await;
            };
            async_std::task::spawn(Abortable::new(fut, registration));
        }
       tasks_abort
    }

    /// Method to stop multiple async evals. 
    async fn stop_evals(&self, mut tasks_abort:Vec<AbortHandle>) -> Vec<AbortHandle> {
        for handle in tasks_abort.drain(1..tasks_abort.len()) {
            handle.abort();
        }
        tasks_abort
    }
    
    /// Method to call each evals.
    async fn eval(&self, path: String, chunk_number: usize, chunk_size: usize) {
        let eval_path = format!("{}/{}", path, chunk_number);
        info!("Running Eval {} on path {}", chunk_number, eval_path);
        let _ = match self.run_eval_e2e(eval_path, EVALApiArgs{chunk_size}).await {
            Ok(_) => info!("Finished Eval {}", chunk_number),
            Err(e) => error!("Error during the Eval: {}.", e)
        };
    }
    
    /// The API to retrieve bytes related the chunks
    pub async fn run_eval_e2e(&self, path_str: String, args: EVALApiArgs) -> Result<(), Box<dyn Error>> {
        let _ = env_logger::try_init();
        
        let chunk_size: usize = check_eval_args(path_str.clone(), args)?;
        let path: zenoh::Path = zenoh::Path::try_from(path_str.clone())?;
        let path_expr = PathExpr::try_from(path_str.clone())?;
    
        info!("New workspace...");
        let workspace = self.zenoh.workspace(None).await?;
    
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
        Ok(())
    }
}
