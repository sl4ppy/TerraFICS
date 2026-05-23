//! Background-thread PNG decoder for base map tiles.
//!
//! Owns a worker thread and two `mpsc` channels:
//!
//! - Request channel (out): `Sender<TileKey>` — the renderer enqueues
//!   tiles it wants loaded.
//! - Response channel (in): `Receiver<LoadedTile>` — the worker delivers
//!   decoded RGBA8 bytes back. The renderer drains this each frame.
//!
//! On `Drop`, the request sender is dropped which closes the channel; the
//! worker's `recv()` returns `Err` and the thread exits cleanly; `join()`
//! reaps it.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::JoinHandle;

use crate::tiles::coord::TileKey;

/// Decoded tile payload delivered from the worker to the renderer.
#[derive(Debug)]
pub struct LoadedTile {
    /// Identifier of the tile this payload corresponds to.
    pub key: TileKey,
    /// Decoded RGBA8 bytes if loading succeeded; the size is always
    /// `TILE_PIXEL_SIZE * TILE_PIXEL_SIZE * 4` for a 256x256 RGBA tile.
    pub result: Result<Vec<u8>, LoadError>,
}

/// Why a tile load failed.
#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    /// PNG file not present on disk.
    #[error("tile file not found: {0}")]
    NotFound(PathBuf),
    /// I/O error reading the PNG file.
    #[error("tile io error for {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    /// PNG decoder rejected the file.
    #[error("tile decode error for {path}: {source}")]
    Decode {
        path: PathBuf,
        source: image::ImageError,
    },
}

/// Background-thread PNG decoder. Cheap to construct; joins the worker on `Drop`.
#[derive(Debug)]
pub struct LoaderHandle {
    requests: Option<Sender<TileKey>>,
    responses: Receiver<LoadedTile>,
    worker: Option<JoinHandle<()>>,
}

impl LoaderHandle {
    /// Spawn a worker that resolves tile paths under `root`.
    #[must_use]
    pub fn spawn(root: PathBuf) -> Self {
        let (req_tx, req_rx) = mpsc::channel::<TileKey>();
        let (resp_tx, resp_rx) = mpsc::channel::<LoadedTile>();
        let worker = std::thread::Builder::new()
            .name("scim-render tile loader".into())
            .spawn(move || worker_loop(&root, &req_rx, &resp_tx))
            .expect("spawn tile loader thread");
        Self {
            requests: Some(req_tx),
            responses: resp_rx,
            worker: Some(worker),
        }
    }

    /// Enqueue a tile load. Returns `false` if the worker has died (rare).
    #[must_use]
    pub fn request(&self, key: TileKey) -> bool {
        self.requests.as_ref().map_or(false, |tx| tx.send(key).is_ok())
    }

    /// Non-blocking drain of decoded tiles delivered since the last call.
    #[must_use]
    pub fn drain_ready(&self) -> Vec<LoadedTile> {
        let mut out = Vec::new();
        while let Ok(tile) = self.responses.try_recv() {
            out.push(tile);
        }
        out
    }
}

impl Drop for LoaderHandle {
    fn drop(&mut self) {
        // Close the request channel; worker's recv will return Err and the
        // worker exits cleanly.
        self.requests.take();
        if let Some(handle) = self.worker.take() {
            // Best-effort join. If it panicked we don't propagate (test
            // process is exiting anyway).
            let _ = handle.join();
        }
    }
}

fn worker_loop(root: &Path, requests: &Receiver<TileKey>, responses: &Sender<LoadedTile>) {
    while let Ok(key) = requests.recv() {
        let result = load_tile(root, key);
        // If the response channel is dropped (renderer shutting down), exit.
        if responses.send(LoadedTile { key, result }).is_err() {
            break;
        }
    }
}

fn load_tile(root: &Path, key: TileKey) -> Result<Vec<u8>, LoadError> {
    let path = tile_path(root, key);
    if !path.exists() {
        return Err(LoadError::NotFound(path));
    }
    let img = image::open(&path).map_err(|source| LoadError::Decode {
        path: path.clone(),
        source,
    })?;
    let rgba = img.to_rgba8();
    Ok(rgba.into_raw())
}

/// Path to a tile PNG under `root`: `{root}/{z}/{x}/{y}.png`.
#[must_use]
pub fn tile_path(root: &Path, key: TileKey) -> PathBuf {
    root.join(key.zoom.to_string())
        .join(key.x.to_string())
        .join(format!("{}.png", key.y))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("tile.png")
    }

    fn tempdir_with_fixture(zoom: u8, x: u32, y: u32) -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        let dst = tile_path(dir.path(), TileKey { zoom, x, y });
        std::fs::create_dir_all(dst.parent().expect("tile_path has at least 2 ancestors")).expect("create dirs");
        std::fs::copy(fixture_path(), &dst).expect("copy fixture");
        dir
    }

    #[test]
    fn load_existing_tile_returns_rgba_bytes() {
        let dir = tempdir_with_fixture(3, 0, 0);
        let loader = LoaderHandle::spawn(dir.path().to_path_buf());
        assert!(loader.request(TileKey { zoom: 3, x: 0, y: 0 }));

        // Poll for up to ~1 second for the decoded tile to arrive.
        let start = std::time::Instant::now();
        let mut got = Vec::new();
        while got.is_empty() && start.elapsed().as_millis() < 1000 {
            got = loader.drain_ready();
            if got.is_empty() {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
        assert_eq!(got.len(), 1, "did not receive any tile within 1s");
        let payload = got.into_iter().next().unwrap();
        assert_eq!(payload.key, TileKey { zoom: 3, x: 0, y: 0 });
        let bytes = payload.result.expect("decode ok");
        // 256x256 RGBA8 = 262144 bytes.
        assert_eq!(bytes.len(), 256 * 256 * 4);
    }

    #[test]
    fn load_missing_tile_returns_not_found() {
        let dir = tempfile::tempdir().expect("tempdir");
        let loader = LoaderHandle::spawn(dir.path().to_path_buf());
        assert!(loader.request(TileKey { zoom: 4, x: 1, y: 2 }));

        let start = std::time::Instant::now();
        let mut got = Vec::new();
        while got.is_empty() && start.elapsed().as_millis() < 1000 {
            got = loader.drain_ready();
            if got.is_empty() {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
        assert_eq!(got.len(), 1);
        let payload = got.into_iter().next().unwrap();
        assert!(matches!(payload.result, Err(LoadError::NotFound(_))));
    }

    #[test]
    fn drop_joins_worker_cleanly() {
        let dir = tempfile::tempdir().expect("tempdir");
        let loader = LoaderHandle::spawn(dir.path().to_path_buf());
        // Send nothing; just drop.
        drop(loader);
        // If the worker leaks or panics, the test process won't exit
        // cleanly; if we get here, drop completed without timing out.
    }
}
