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

use clap::{App, Arg};
use fragmentation_e2e::get_e2e;
use zenoh::Properties;

#[async_std::main]
async fn main() {
    let (config, selector, root_folder, index_start, index_end, chunk_index_start, chunk_index_end) =
        parse_args();

    let root_folder_chunks: &str = "/tmp/chunks";

    println!("Calling the GET API to retrieve the file...");
    let res: String = match get_e2e(
        config,
        selector,
        root_folder,
        root_folder_chunks,
        index_start,
        index_end,
        chunk_index_start,
        chunk_index_end,
    ).await {
        Ok(path) => format!("Finished to retrieve the file. The downloaded file is: {}", path),
        Err(e) => format!("Error during the Get: {:?}.", e)
    };
    println!("{}", res);
}

fn parse_args() -> (Properties, String, String, String, String, String, String) {
    let args = App::new("zenoh get example")
        .arg(
            Arg::from_usage("-m, --mode=[MODE] 'The zenoh session mode.")
                .possible_values(&["peer", "client"])
                .default_value("peer"),
        )
        .arg(Arg::from_usage(
            "-e, --peer=[LOCATOR]...  'Peer locators used to initiate the zenoh session.'",
        ))
        .arg(Arg::from_usage(
            "-l, --listener=[LOCATOR]...   'Locators to listen on.'",
        ))
        .arg(
            Arg::from_usage("-s, --selector=[SELECTOR] 'The selection of resources to get'")
                .default_value("/demo/example/**"),
        )
        .arg(Arg::from_usage(
            "--no-multicast-scouting 'Disable the multicast-based scouting mechanism.'",)
        )
        .arg(Arg::from_usage(
            "-r, --root_folder    'Path of the directory where to download the chunks and the file.'",
            ).default_value("/tmp/final")
        )
        .arg(Arg::from_usage(
            "-a, --index_start  'Index where to start to retrieve the bytes of the file.'",
            ).default_value("0")
        )
        .arg(Arg::from_usage(
            "-b, --index_end    'Index where to stop to retrieve the bytes of the file.'",
            ).default_value("0")
        )
        .arg(Arg::from_usage(
            "-c, --chunk_start  'Index of the first chunk of the file to retrieve.'",
            ).default_value("0")
        )
        .arg(Arg::from_usage(
            "-d, --chunk_end    'Index of the last chunk of the file to retrieve.'",
            ).default_value("0")
        )
        .get_matches();

    let mut config = Properties::default();
    for key in ["mode", "peer", "listener"].iter() {
        if let Some(value) = args.values_of(key) {
            config.insert(key.to_string(), value.collect::<Vec<&str>>().join(","));
        }
    }
    if args.is_present("no-multicast-scouting") {
        config.insert("multicast_scouting".to_string(), "false".to_string());
    }

    let selector = args.value_of("selector").unwrap().to_string();
    let root_folder = args.value_of("root_folder").unwrap().to_string();
    let index_start = args.value_of("index_start").unwrap().to_string();
    let index_end = args.value_of("index_end").unwrap().to_string();
    let chunk_start = args.value_of("chunk_start").unwrap().to_string();
    let chunk_end = args.value_of("chunk_end").unwrap().to_string();

    (
        config,
        selector,
        root_folder,
        index_start,
        index_end,
        chunk_start,
        chunk_end,
    )
}
