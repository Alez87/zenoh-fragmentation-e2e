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

use std::time::Instant;

use fragmentation_e2e::ZenohCdn;
use clap::{App, Arg};
use fragmentation_e2e::{PUTApiArgs};
use zenoh::{Properties, ZError};

#[async_std::main]
async fn main() {
    env_logger::init();

    let (config, path, value, chunk_size) = parse_args();
    println!("Calling the PUT API to share the file...");

    let start = Instant::now();

    let mut zenoh_cdn = ZenohCdn::new_session(config)
    .await
    .map_err(|e: ZError| {
        zenoh_util::zerror2!(zenoh::ZErrorKind::InvalidSession {
            descr: format!("Error during creation of ZenohCdn: {}", e),
        })
    }).unwrap();

    zenoh_cdn.set_upload_args(PUTApiArgs{chunk_size});

    let cretion_time = start.elapsed().as_micros();
    println!("ZenohCDN creation: {}us", cretion_time);

    let res: String = match zenoh_cdn.upload(path, value).await {
        Ok(_) => String::from("Finished to share the file."),
        Err(e) => format!("Error during the Put: {:?}.", e)
    };

    let finish = start.elapsed().as_micros();
    println!("End of PUT Api: {}us", finish);

    println!("{}", res);
}

fn parse_args() -> (Properties, String, String, usize) {
    let args = App::new("zenoh put example")
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
            Arg::from_usage("-p, --path=[PATH]        'The name of the resource to put.'")
                .default_value("/demo/example/zenoh-rs-put"),
        )
        .arg(
            Arg::from_usage("-v, --value=[VALUE]      'The value of the resource to put.'")
                .default_value("Put from Rust!"),
        )
        .arg(Arg::from_usage(
            "--no-multicast-scouting 'Disable the multicast-based scouting mechanism.'",
        ))
        .arg(
            Arg::from_usage(
                "-s, --csize=[VALUE]      'The size of the chunk size to use to fragment.'",
            )
            .default_value("65000"),
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

    let path = args.value_of("path").unwrap().to_string();
    let value = args.value_of("value").unwrap().to_string();
    let chunk_size_string: String = args.value_of("csize").unwrap().to_string();
    let chunk_size: usize = chunk_size_string.parse::<usize>().unwrap();

    (config, path, value, chunk_size)
}
