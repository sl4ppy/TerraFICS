//! Error type for `scim-render`.
//! No `anyhow` per design spec §11.1.

use thiserror::Error;

/// Errors emitted by `scim-render`.
#[derive(Debug, Error)]
pub enum Error {
    /// Could not get a wgpu adapter compatible with the surface.
    #[error("no wgpu adapter available")]
    Adapter,
    /// Could not request a wgpu device.
    #[error("wgpu device request failed: {0}")]
    Device(wgpu::RequestDeviceError),
    /// Failed to create a wgpu surface from a window.
    #[error("wgpu surface creation failed: {0}")]
    Surface(#[from] wgpu::CreateSurfaceError),
    /// Underlying `scim-store` failure (DB read, decode).
    #[error("scim-store error: {0}")]
    Store(#[from] scim_store::Error),
    /// Underlying `scim-world` failure (index build).
    #[error("scim-world error: {0}")]
    World(#[from] scim_world::Error),
}

/// Crate result alias.
#[must_use]
#[allow(unused_attributes)]
pub type Result<T> = std::result::Result<T, Error>;
