/// Unit tests for the library.
///
/// The tests are divided per type of example:
/// - z_put_file
/// - z_get_file
/// - z_eval_file
///
mod common;
use std::io;

#[cfg(test)]
mod tests_put {

    use std::path::PathBuf;

    use super::*;

    #[async_std::test]
    async fn path_invalid() {
        let (config, path, value, chunk_size) = common::setup_put("peer", "", "image.png", 65_000);
        let result = common::call_put(config, path, value, chunk_size).await;
        println!("Result: {:?}", result);
        assert_eq!(Err(io::ErrorKind::InvalidInput), result);
    }

    #[async_std::test]
    async fn value_invalid() {
        let (config, path, value, chunk_size) =
            common::setup_put("peer", "/demo/example/myfile", "", 65_000);
        let result = common::call_put(config, path, value, chunk_size).await;
        assert_eq!(Err(io::ErrorKind::InvalidInput), result);
    }

    #[async_std::test]
    async fn chunk_size_invalid() {
        let (config, path, value, chunk_size) =
            common::setup_put("peer", "/demo/example/myfile", "image.png", 0);
        let result = common::call_put(config, path, value, chunk_size).await;
        assert_eq!(Err(io::ErrorKind::InvalidInput), result);
    }

    #[async_std::test]
    async fn file_notfound() {
        let (config, path, value, chunk_size) = common::setup_put(
            "peer",
            "/demo/example/myfile",
            "wrong_path/image.png",
            65_000,
        );
        let result = common::call_put(config, path, value, chunk_size).await;
        assert_eq!(Err(io::ErrorKind::NotFound), result);
    }

    #[ignore]
    #[async_std::test]
    async fn full_test() {
        //let absolute_path = format!("{}/tests/zenoh.png", std::env::current_dir().unwrap().into_os_string().into_string().unwrap());
        //println!("Absolute image path: {}", absolute_path);
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("tests/zenoh.png");
        let absolute_path = format!("{}", d.display());
        let (config, path, value, chunk_size) =
            common::setup_put("peer", "/demo/example/myfile", &absolute_path, 65_000);
        let res = common::call_put(config, path, value, chunk_size).await;
        assert_eq!(res.is_err(), false);
    }
}

#[cfg(test)]
mod tests_get {

    use super::*;

    #[async_std::test]
    async fn path_invalid() {
        let (
            config,
            selector,
            root_folder,
            root_folder_chunks,
            index_start,
            index_end,
            chunk_index_start,
            chunk_index_end,
        ) = common::setup_get("peer", "", 0, 70_000, 0, 0);
        let result = common::call_get(
            config,
            selector,
            root_folder,
            root_folder_chunks,
            index_start,
            index_end,
            chunk_index_start,
            chunk_index_end,
        )
        .await;
        println!("Result: {:?}", result);
        assert_eq!(Err(io::ErrorKind::InvalidInput), result);
    }

    #[async_std::test]
    async fn path_notfound() {
        let (
            config,
            selector,
            root_folder,
            root_folder_chunks,
            index_start,
            index_end,
            chunk_index_start,
            chunk_index_end,
        ) = common::setup_get("peer", "/demo/wrong_path", 0, 70_000, 0, 0);
        let result = common::call_get(
            config,
            selector,
            root_folder,
            root_folder_chunks,
            index_start,
            index_end,
            chunk_index_start,
            chunk_index_end,
        )
        .await;
        println!("Result: {:?}", result);
        assert_eq!(Err(io::ErrorKind::NotFound), result);
    }

    #[ignore]
    #[async_std::test]
    async fn index_invalid() {
        let (
            config,
            selector,
            root_folder,
            root_folder_chunks,
            index_start,
            index_end,
            chunk_index_start,
            chunk_index_end,
        ) = common::setup_get("peer", "/demo/example/myfile", 2, 1, 0, 0);
        let result = common::call_get(
            config,
            selector,
            root_folder,
            root_folder_chunks,
            index_start,
            index_end,
            chunk_index_start,
            chunk_index_end,
        )
        .await;
        println!("Result: {:?}", result);
        assert_eq!(Err(io::ErrorKind::Other), result);
    }

    #[ignore]
    #[async_std::test]
    async fn chunksindex_invalid() {
        let (
            config,
            selector,
            root_folder,
            root_folder_chunks,
            index_start,
            index_end,
            chunk_index_start,
            chunk_index_end,
        ) = common::setup_get("peer", "/demo/example/myfile", 0, 0, 2, 1);
        let result = common::call_get(
            config,
            selector,
            root_folder,
            root_folder_chunks,
            index_start,
            index_end,
            chunk_index_start,
            chunk_index_end,
        )
        .await;
        println!("Result: {:?}", result);
        assert_eq!(Err(io::ErrorKind::Other), result);
    }

    #[ignore]
    #[async_std::test]
    async fn full_test() {
        let (
            config,
            selector,
            root_folder,
            root_folder_chunks,
            index_start,
            index_end,
            chunk_index_start,
            chunk_index_end,
        ) = common::setup_get("peer", "/demo/example/myfile", 0, 2, 0, 0);
        let result = common::call_get(
            config,
            selector,
            root_folder,
            root_folder_chunks,
            index_start,
            index_end,
            chunk_index_start,
            chunk_index_end,
        )
        .await;
        println!("Result: {:?}", result);
        assert_eq!(Err(io::ErrorKind::Other), result);
    }
}

#[cfg(test)]
mod tests_eval {
    use super::*;

    #[async_std::test]
    async fn path_invalid() {
        let (config, path, chunk_size) = common::setup_eval("peer", "", 65_000);
        let result = common::call_eval(config, path, chunk_size).await;
        println!("Result: {:?}", result);
        assert_eq!(Err(io::ErrorKind::InvalidInput), result);
    }

    #[async_std::test]
    async fn chunksize_invalid() {
        let (config, path, chunk_size) = common::setup_eval("peer", "/demp/example/myfile", 0);
        let result = common::call_eval(config, path, chunk_size).await;
        println!("Result: {:?}", result);
        assert_eq!(Err(io::ErrorKind::InvalidInput), result);
    }

    #[ignore]
    #[async_std::test]
    async fn full_test() {
        let (config, path, chunk_size) = common::setup_eval("peer", "/demp/example/myfile", 65_000);
        let result = common::call_eval(config, path, chunk_size).await;
        println!("Result: {:?}", result);
        assert_eq!(Err(io::ErrorKind::InvalidInput), result);
    }
}
