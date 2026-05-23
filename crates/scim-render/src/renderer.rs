//! wgpu renderer. See `Renderer` for the public surface. Built in Tasks 5-7.

/// GPU renderer for the footprint pass. Filled in by Tasks 5-7.
//
// `Default` is transient — it stays only while this is a unit-struct stub.
// Task 6 adds GPU-resource fields (Device, Queue, Surface, ...) that are not
// `Default`-constructible; the derive is dropped at that point.
#[derive(Debug, Default)]
pub struct Renderer;
