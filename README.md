<img src="http://zenoh.io/img/zenoh-dragon-small.png" width="150">

[![CI](https://github.com/Alez87/zenoh-fragmentation-e2e/actions/workflows/ci.yml/badge.svg)](https://github.com/Alez87/zenoh-fragmentation-e2e/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/License-EPL%202.0-blue)](https://choosealicense.com/licenses/epl-2.0/)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)

# Fragmentation system for Eclipse zenoh

Zenoh uses an hop-to-hop fragmentation approach, typical of the Name Data Networking (NDN) concept. Although, generally, this is good approach, in some cases, i.e. when dealing with large files, in particular when intermediate nodes may also consider constraint devices, a different approach may be more suitable in order to minimize the number of fragmentation/reconstrcutions.

See the [zenoh documentation](http://zenoh.io/docs/manual/backends/) for more details.

This library is advisable for sending large files because it allows to fragment and reconstruct as less as it can: at source and at destination.
It's a library that relies on zenoh and shares files on zenoh in order to avoid the h2h fragmentation.

-------------------------------

## **Prerequisites**

Prerequisites:
 - You have a zenoh router running, and the [zenoh-backend-filesystem](https://download.eclipse.org/zenoh/zenoh-backend-filesystem) configured.
 - Declare the `ZBACKEND_FS_ROOT` environment variable to the directory where you want the files to be stored (or exposed from).
If you don't declare it, the `~/.zenoh/zbackend_fs` directory will be used.

Add the zenoh-backend-filesystem library to Zenoh to enable the backend and storage filesystem usage:
```bash
mkdir ~/.zenoh
mkdir ~/.zenoh/lib
cd zenoh-backend-filesystem
cp target/release/libzbackend_fs.dylib ~/.zenoh/lib/
```

Using `curl` on the zenoh router to add backend and storages:
```bash
# Add a backend that will have all its storages storing data in subdirectories of ${ZBACKEND_FS_ROOT} directory.
curl -X PUT -H 'content-type:application/properties' http://localhost:8001/@/router/local/plugin/storages/backend/fs

# Retreive the list and information of existent backends.
curl "http://localhost:8001/@/router/local/plugin/storages/backend/*"

# Add a storage on /demo/example/** storing data in files under ${ZBACKEND_FS_ROOT}/test/ directory
# We use 'path_prefix=/demo/example' thus a zenoh path "/demo/example/a/b" will be stored as "${ZBACKEND_FS_ROOT}/test/a/b"
curl -X PUT -H 'content-type:application/properties' -d "path_expr=/demo/example/myfile/**;path_prefix=/demo/example/myfile;dir=test" http://localhost:8001/@/router/local/plugin/storages/backend/fs/storage/example

# Retrieve the list of storages related to the filesystem backend we created
ls ~/.zenoh/zbackendfs
curl "http://localhost:8001/@/router/local/plugin/storages/backend/fs/storage/*"
```

-------------------------------

## Properties for file sharing and retrieval

PUT API:
- "-m" : The zenoh session mode (peer, client).
- "-p" (required) : The name of the resource to put, e.g. "/demo/example/myfile".
- "-v" (required) : The value of the resource to put, e.g. "~/Downloads/zenoh.png".
- "-l" : Locators to listen on, e.g. "tcp/127.0.0.1:7448".
- "-s" : The size of the chunk size to use to fragment, e.g. "65_000".
- "--no-multicast-scouting" : Disable the multicast-based scouting mechanism.

GET API:
- "-m" : The zenoh session mode (peer, client).
- "-e" : Peer locators used to initiate the zenoh session.
- "-l" : Locators to listen on, e.g. "tcp/127.0.0.1:7448".
- "-s" : (required) The selection of resources to get.
- "--no-multicast-scouting" : Disable the multicast-based scouting mechanism.
- "-r" : Path of the directory where to download the chunks and the file, e.g. "/tmp/final"
- "-a" : Index where to start to retrieve the bytes of the file.
- "-b" : Index where to stop to retrieve the bytes of the file.
- "-c" : Index of the first chunk of the file to retrieve.
- "-d" : Index of the last chunk of the file to retrieve.

-------------------------------

## **Examples of usage**

Execute Zenoh on 7448 port
```bash
$ RUST_LOG=debug ./target/release/zenohd -l tcp/127.0.0.1:7448
```
you can then expose the REST API on a custom port (--rest-http-port 8001) or leave the default 8000

Locate the examples folder and use the PUT Api to share the file to send
```bash
cd target/release/examples
./z_put_file -p /demo/example/myfile -v ~/Downloads/zenoh.png
```

Create the folder where files, both chunks and the final entire files, will be downloaded
```bash
mkdir /tmp/chunks
mkdir /tmp/final 
```

GET api to retrieve the file locally
```bash
cd target/release/examples
./z_get_file -s "/demo/example/myfile" -a 0 -b 70000
./z_get_file -s "/demo/example/myfile" -c 0 -d 3
```

-------------------------------

## How to build it

At first, install [Cargo and Rust](https://doc.rust-lang.org/cargo/getting-started/installation.html). 

:warning: **WARNING** :warning: : As Rust doesn't have a stable ABI, the backend library should be
built with the exact same Rust version than `zenohd`. Otherwise, incompatibilities in memory mapping
of shared types between `zenohd` and the library can lead to a `"SIGSEV"` crash.

To know the Rust version you're `zenohd` has been built with, use the `--version` option.  
Example:
```bash
$ zenohd --version
The zenoh router v0.5.0-beta.8 built with rustc 1.52.0-nightly (107896c32 2021-03-15)
```
Here, `zenohd` has been built with the rustc version `1.52.0-nightly` built on 2021-03-15.  
A nightly build of rustc is included in the **Rustup** nightly toolchain the day after.
Thus you'll need to install to toolchain **`nightly-2021-03-15`**
Install and use this toolchain with the following command:

```bash
$ rustup default nightly-2021-03-15
```

And then build the library with:

```bash
$ cargo build --release --all-targets
```
