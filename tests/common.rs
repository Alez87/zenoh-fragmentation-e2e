/// Common functions used from any unit tests.
/// 

extern crate fragmentation_e2e;
use fragmentation_e2e::put_e2e;
use fragmentation_e2e::get_e2e;
use fragmentation_e2e::run_eval_e2e;
use zenoh::Properties;

pub fn setup_put(mode: &str, path: &str, value: &str, chunk_size: usize) -> (Properties, String, String, usize) {
    let mut config = Properties::default();
    config.insert("mode".to_string(), mode.to_string()); 
    let path: String = path.to_string();
    let value: String = value.to_string();

    (config, path, value, chunk_size)
}

pub fn setup_get(mode: &str, selector: &str, index_start: usize, index_end: usize, chunk_start: usize, chunk_end: usize) -> (Properties, String, String, &'static str, String, String, String, String) {
    let mut config = Properties::default();
    config.insert("mode".to_string(), mode.to_string()); 
    let selector: String = selector.to_string();
    let selector = selector.to_string();
    let root_folder_final = "/tmp/final".to_string();
    let root_folder_chunks = "/tmp/chunks";
    let index_start = index_start.to_string();
    let index_end = index_end.to_string();
    let chunk_start = chunk_start.to_string();
    let chunk_end = chunk_end.to_string();

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
    match put_e2e(config, path, value, chunk_size).await {
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
    root_folder_final: String,
    root_folder_chunks: &str,
    index_start: String,
    index_end: String,
    chunk_index_start: String,
    chunk_index_end: String
    ) -> Result<(), std::io::ErrorKind> {
    println!("Calling the GET API to retrieve the file...");
    match get_e2e(config, selector, root_folder_final, root_folder_chunks, index_start, index_end, chunk_index_start, chunk_index_end,).await {
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
    match run_eval_e2e(config, path, chunk_size).await {
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