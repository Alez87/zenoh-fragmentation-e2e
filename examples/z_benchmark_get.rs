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

extern crate fragmentation_e2e;

use async_std::sync::Arc;
use std::time::Instant;

use fragmentation_e2e::{GETApiChunksArgs, GETApiFoldersArgs, ZenohCdn};
use zenoh::{Properties, ZError};

#[async_std::main]
async fn main() {
    env_logger::init();

    println!("Preparing the parameters...");
    let mut config = Properties::default();
    config.insert("mode".to_string(), "peer".to_string());
    config.insert("-l".to_string(), "tcp/127.0.0.1:7448".to_string()); 
    let path: String = "/demo/example/myfile".to_string();
    //let value: String = "~/Downloads/zenoh.png".to_string();
    let chunk_size: usize = 65_000;
    println!("Calling the PUT API to share the file...");

    let start = Instant::now();

    let mut zenoh_cdn = ZenohCdn::new_session(config)
    .await
    .map_err(|e: ZError| {
        zenoh_util::zerror2!(zenoh::ZErrorKind::InvalidSession {
            descr: format!("Error during creation of ZenohCdn: {}", e),
        })
    }).unwrap();

    zenoh_cdn.set_download_folders(GETApiFoldersArgs::default());
    zenoh_cdn.set_download_bytes_args(GETApiChunksArgs::default());

    let creation_time = start.elapsed().as_micros();
    println!("ZenohCDN creation: {}us", creation_time);

    let zenoh: Arc<ZenohCdn> = Arc::new(zenoh_cdn);
    let count: usize = 5;
    let num: Vec<usize> = (1..=count).collect();
    call_func(zenoh, path, num, chunk_size).await;
}

async fn call_func(zenoh_cdn: Arc<ZenohCdn>, path_init: String, num: Vec<usize>, chunk_size: usize) {
    let mut tasks = Vec::with_capacity(num.len());
    println!("Creating the parallel tasks...");
    let start = Instant::now();
    for n in num {
        println!("Starting task number {}", n);
        let zenoh = zenoh_cdn.clone();
        let path = path_init.to_string();
        tasks.push(async_std::task::spawn(async move {
            func(zenoh, path, n, chunk_size).await;
        }));
        let end = start.elapsed().as_micros();
        println!("End async task {}us", end);
    }
    for task in tasks {
        let start_for_task = start.elapsed().as_micros();
        println!("start_for_task {}us", start_for_task);
        task.await;
        let end_for_task = start.elapsed().as_micros();
        println!("End_for_task async task {}us", end_for_task);
    }
    let finished = start.elapsed().as_micros();
    println!("Finished async task {}us", finished);
}

async fn func(zenoh_cdn: Arc<ZenohCdn>, path: String, chunk_number: usize, _chunk_size: usize) {
    //let path = format!("{}/{}", path, _chunk_number%2);
    println!("Calling GET api for chunk number {}", chunk_number);
    println!("Path: {}", path);
    let _ = match zenoh_cdn.download(path, "").await {
        Ok(_) => println!("Finished Eval {}", chunk_number),
        Err(e) => println!("Error during the Eval: {}.", e)
    };
}
