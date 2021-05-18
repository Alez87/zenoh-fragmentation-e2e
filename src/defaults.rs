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

use std::default::Default;

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

impl Default for EVALApiArgs {
    fn default() -> Self { 
        Self {
           chunk_size: 65_000
       }
   }
}