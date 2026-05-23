//! Off-screen pick pass for click-pick selection (P1.5-c).
//!
//! Per design spec §6.3 and the P1.5-c MVP design doc: a sibling render
//! pipeline rasterises the same instanced geometry but emits a 1-based
//! instance handle (u32) into an `R32_UINT` texture. On click, the renderer
//! scissors this pass to a 1×1 rect at the cursor, copies the pixel into a
//! staging buffer, blocks on `map_async` via `device.poll(Wait)`, and
//! decodes the handle.

/// Bytes per row required for `copy_texture_to_buffer` destinations.
/// wgpu requires alignment to `COPY_BYTES_PER_ROW_ALIGNMENT` (256).
pub(crate) const PICK_BYTES_PER_ROW: u32 = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;

/// Off-screen pick pass: `R32_UINT` texture + staging buffer + pipeline.
#[derive(Debug)]
pub struct PickPass {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    staging: wgpu::Buffer,
    pipeline: wgpu::RenderPipeline,
    width: u32,
    height: u32,
}

impl PickPass {
    /// Construct a pick pass sized to `width × height`. The pipeline reuses
    /// the `vs_main` entry point in `shader.wgsl` plus the new `fs_pick`
    /// entry. `camera_bgl` and `instance_stride` must match the main
    /// pipeline's so the same buffers can be bound.
    // reason: constructor for a GPU-coupled struct; refactor to a builder if more params arrive
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn new(
        device: &wgpu::Device,
        shader: &wgpu::ShaderModule,
        camera_bgl: &wgpu::BindGroupLayout,
        quad_stride: u64,
        instance_stride: u64,
        width: u32,
        height: u32,
    ) -> Self {
        let (texture, view) = Self::create_texture(device, width, height);
        let staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("scim-render pick staging buffer"),
            // One padded row, enough for a 1×1 readback.
            size: u64::from(PICK_BYTES_PER_ROW),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("scim-render pick pipeline layout"),
            bind_group_layouts: &[camera_bgl],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("scim-render pick pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: "vs_main",
                buffers: &[
                    wgpu::VertexBufferLayout {
                        array_stride: quad_stride,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0 => Float32x2],
                    },
                    wgpu::VertexBufferLayout {
                        array_stride: instance_stride,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &wgpu::vertex_attr_array![
                            1 => Float32x3,
                            2 => Uint32,
                        ],
                    },
                ],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: shader,
                entry_point: "fs_pick",
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::R32Uint,
                    // R32_UINT does not support blending.
                    blend: None,
                    write_mask: wgpu::ColorWrites::RED,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self {
            texture,
            view,
            staging,
            pipeline,
            width,
            height,
        }
    }

    /// Resize the pick texture in lockstep with the surface. Called from
    /// `Renderer::resize`.
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if width == self.width && height == self.height {
            return;
        }
        let (texture, view) = Self::create_texture(device, width, height);
        self.texture = texture;
        self.view = view;
        self.width = width;
        self.height = height;
    }

    /// Pick texture's render target view. Used by `Renderer::pick` when
    /// it begins the pick render pass.
    // reason: const fn cannot return a reference borrowed from &self on stable Rust 1.83
    #[allow(clippy::missing_const_for_fn)]
    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    /// Pipeline used by the pick pass.
    // reason: const fn cannot return a reference borrowed from &self on stable Rust 1.83
    #[allow(clippy::missing_const_for_fn)]
    pub fn pipeline(&self) -> &wgpu::RenderPipeline {
        &self.pipeline
    }

    /// Texture handle (used to source `copy_texture_to_buffer`).
    // reason: const fn cannot return a reference borrowed from &self on stable Rust 1.83
    #[allow(clippy::missing_const_for_fn)]
    pub fn texture(&self) -> &wgpu::Texture {
        &self.texture
    }

    /// Staging buffer holding the readback target row.
    // reason: const fn cannot return a reference borrowed from &self on stable Rust 1.83
    #[allow(clippy::missing_const_for_fn)]
    pub fn staging(&self) -> &wgpu::Buffer {
        &self.staging
    }

    /// Current width of the pick texture.
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// Current height of the pick texture.
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }

    fn create_texture(
        device: &wgpu::Device,
        width: u32,
        height: u32,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("scim-render pick texture (R32Uint)"),
            size: wgpu::Extent3d {
                width: width.max(1),
                height: height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R32Uint,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        (texture, view)
    }
}

#[cfg(test)]
mod tests {
    use std::mem::size_of;

    use super::*;

    #[test]
    fn pick_bytes_per_row_is_wgpu_alignment_256() {
        // Sanity: COPY_BYTES_PER_ROW_ALIGNMENT is a wgpu constant we depend
        // on for the staging buffer sizing math.
        assert_eq!(PICK_BYTES_PER_ROW, 256);
    }

    #[test]
    fn one_u32_fits_in_padded_row() {
        // Trivial assertion: a single u32 is 4 bytes; the padded row is
        // 256 bytes; the first 4 bytes are the data we care about.
        // reason: size_of::<u32>() is 4, which fits trivially in u32; no truncation possible.
        #[allow(clippy::cast_possible_truncation)]
        let sz = size_of::<u32>() as u32;
        assert!(sz <= PICK_BYTES_PER_ROW);
    }
}
