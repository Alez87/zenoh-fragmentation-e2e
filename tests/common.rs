/// Common functions used from any unit tests.
/// 

extern crate fragmentation_e2e;
use fragmentation_e2e::{EVALApiArgs, GETApiChunksArgs, GETApiFoldersArgs, PUTApiArgs, ZenohCdn};
use zenoh::{Properties};
use core::default::Default;

pub fn setup_put(mode: &str, path: &str, value: &str, chunk_size: usize) -> (Properties, String, String, usize) {
    let mut config = Properties::default();
    config.insert("mode".to_string(), mode.to_string());
    config.insert("-l".to_string(), "tcp/127.0.0.1:7448".to_string()); 
    let path: String = path.to_string();
    let value: String = value.to_string();

    (config, path, value, chunk_size)
}

pub fn setup_get(mode: &str, selector: &str, index_start: usize, index_end: usize, chunk_start: usize, chunk_end: usize) -> (Properties, String, &'static str, &'static str, usize, usize, usize, usize) {
    let mut config = Properties::default();
    config.insert("mode".to_string(), mode.to_string()); 
    //config.insert("-l".to_string(), "tcp/127.0.0.1:7448".to_string()); 
    let selector: String = selector.to_string();
    let selector = selector.to_string();
    let root_folder_final = "/tmp/final";
    let root_folder_chunks = "/tmp/chunks";

    (config, selector, root_folder_final, root_folder_chunks, index_start, index_end, chunk_start, chunk_end)
}

pub fn setup_eval(mode: &str, path: &str, chunk_size: usize) -> (Properties, String, usize) {
    let mut config = Properties::default();
    config.insert("mode".to_string(), mode.to_string()); 
    let path: String = path.to_string();

    (config, path, chunk_size)
}

pub async fn call_put(config: Properties, path: String, value: String, chunk_size: usize) -> Result<(), std::io::ErrorKind> {
    println!("Calling the PUT API to share the file...");
    let mut zenohcdn = ZenohCdn::new_session(config).await.unwrap();
    zenohcdn.set_upload_args(PUTApiArgs{chunk_size});

    match zenohcdn.upload(path, value).await {
        Ok(_) => { 
            println!("Finished to send the file.");
            Ok(())
        },
        Err(e) => {
            println!("Error during the Put: {:?}.", e);
            if let Some(ierr) = e.downcast_ref::<std::io::Error>() {
                Err(ierr.kind())
            } else {
                Err(std::io::ErrorKind::Other)
            }
        }
    }
}

pub async fn call_get(config: Properties,
    selector: String,
    root_folder_final: &'static str,
    root_folder_chunks: &'static str,
    index_start: usize,
    index_end: usize,
    chunk_index_start: usize,
    chunk_index_end: usize
    ) -> Result<(), std::io::ErrorKind> {
    println!("Calling the GET API to retrieve the file...");
    let mut zenohcdn = ZenohCdn::new_session(config).await.unwrap();
    
    zenohcdn.set_download_folders(GETApiFoldersArgs{root_folder_final, root_folder_chunks});
    zenohcdn.set_download_bytes_args(GETApiChunksArgs{index_start, index_end, chunk_index_start, chunk_index_end});
    
    match zenohcdn.download(selector, "").await {
        Ok(_) => { 
            println!("Finished to retrieve the file.");
            Ok(())
        },
        Err(e) => {
            println!("Error during the Get: {:?}.", e);
            if let Some(ierr) = e.downcast_ref::<std::io::Error>() {
                Err(ierr.kind())
            } else {
                Err(std::io::ErrorKind::Other)
            }
        }
    }
}

pub async fn call_eval(config: Properties, path: String, chunk_size: usize) -> Result<(), std::io::ErrorKind> {
    println!("Calling the PUT API to share the file...");
    let zenoh_cdn = ZenohCdn::new_session(config).await.unwrap();
    match zenoh_cdn.run_eval_e2e(path, EVALApiArgs{chunk_size}).await {
        Ok(_) => { 
            println!("Finished to execute the Eval.");
            Ok(())
        },
        Err(e) => {
            println!("Error during the EVAL: {:?}.", e);
            if let Some(ierr) = e.downcast_ref::<std::io::Error>() {
                Err(ierr.kind())
            } else {
                Err(std::io::ErrorKind::Other)
            }
        }
    }
}