//! Base map tile layer for the scim-render footprint viewer (P1.5-d).
//!
//! Per design spec §6.1 and the P1.5-d MVP design doc: PNG tiles laid out
//! as `{root}/{z}/{x}/{y}.png` are loaded on a background thread and
//! rendered as textured quads underneath the actor footprints. Dynamic
//! zoom selection by `Camera2d::units_per_pixel`. No network / no CDN
//! fetch in this milestone — see the spec for the follow-up plan.

pub mod coord;
pub mod loader;

use std::collections::HashMap;
use std::path::PathBuf;

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use crate::tiles::coord::TileKey;
use crate::tiles::loader::LoaderHandle;

/// Cap on the number of resident tiles in VRAM. 256 tiles * 256x256 RGBA
/// = 64 MB; comfortable on any desktop GPU.
const MAX_RESIDENT_TILES: usize = 256;

/// Unit-quad vertex layout for the tile pipeline. Tile pass uses [0, 1]
/// (not [-0.5, 0.5] like the footprint pass) so the vertex shader can
/// directly `mix()` between the per-tile AABB corners.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct TileVertex {
    pos: [f32; 2],
}

const TILE_VERTICES: &[TileVertex] = &[
    TileVertex { pos: [0.0, 0.0] },
    TileVertex { pos: [1.0, 0.0] },
    TileVertex { pos: [0.0, 1.0] },
    TileVertex { pos: [1.0, 1.0] },
];
const TILE_INDICES: &[u16] = &[0, 1, 2, 2, 1, 3];

/// Off-screen tile pass — pipeline + tile cache + loader handle.
/// Constructed by `Renderer` when `set_tile_root(Some(_))` is called.
#[derive(Debug)]
pub struct TilePass {
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    tile_bgl: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    /// Currently-resident tiles, keyed by `TileKey`. Filled in by Task 5.
    resident: HashMap<TileKey, ResidentTile>,
    /// Tiles we've asked the loader for but haven't received yet.
    in_flight: std::collections::HashSet<TileKey>,
    /// Tiles whose load returned an error; we don't retry them this session.
    failed: std::collections::HashSet<TileKey>,
    /// Monotonic frame counter for LRU bookkeeping.
    frame_counter: u64,
    /// Background loader. `None` only between construction stages — always
    /// `Some` after `new`.
    loader: LoaderHandle,
}

/// One resident tile's GPU resources + LRU metadata. Built when a tile
/// arrives from the loader.
#[derive(Debug)]
struct ResidentTile {
    _texture: wgpu::Texture,
    _view: wgpu::TextureView,
    _uniform: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    last_touched_frame: u64,
}

impl TilePass {
    /// Construct a tile pass against the given root directory. Shares the
    /// caller's camera bind group layout so we can bind the existing
    /// camera uniform.
    // reason: wgpu pipeline construction is inherently long; refactor when fragmenting helps
    #[allow(clippy::too_many_lines)]
    #[must_use]
    pub fn new(
        device: &wgpu::Device,
        camera_bgl: &wgpu::BindGroupLayout,
        surface_format: wgpu::TextureFormat,
        root: PathBuf,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("scim-render tile shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../tile_shader.wgsl").into()),
        });

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("scim-render tile unit-quad vertex buffer"),
            contents: bytemuck::cast_slice(TILE_VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("scim-render tile unit-quad index buffer"),
            contents: bytemuck::cast_slice(TILE_INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        let tile_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("scim-render tile bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("scim-render tile sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("scim-render tile pipeline layout"),
            bind_group_layouts: &[camera_bgl, &tile_bgl],
            push_constant_ranges: &[],
        });

        let quad_stride = u64::try_from(std::mem::size_of::<TileVertex>())
            .expect("TileVertex stride fits in u64");

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("scim-render tile pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: quad_stride,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2],
                }],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
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

        let loader = LoaderHandle::spawn(root);

        Self {
            pipeline,
            vertex_buffer,
            index_buffer,
            tile_bgl,
            sampler,
            resident: HashMap::new(),
            in_flight: std::collections::HashSet::new(),
            failed: std::collections::HashSet::new(),
            frame_counter: 0,
            loader,
        }
    }

    /// Drive the loader + cache for the current frame. Computes the
    /// visible tile set from the camera, enqueues missing tiles, drains
    /// the loader response channel, and evicts LRU once over budget.
    /// Returns the visible tile set so `encode` can iterate it.
    pub fn update(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        camera_units_per_pixel: f32,
        camera_world_aabb: [f32; 4],
    ) -> Vec<TileKey> {
        self.frame_counter = self.frame_counter.wrapping_add(1);

        let zoom = crate::tiles::coord::tile_zoom_for(camera_units_per_pixel);
        let visible = crate::tiles::coord::visible_tiles(zoom, camera_world_aabb);

        // Drain decoded tiles delivered since last frame.
        for delivered in self.loader.drain_ready() {
            self.in_flight.remove(&delivered.key);
            match delivered.result {
                Ok(bytes) => {
                    let resident =
                        self.create_resident_tile(device, queue, delivered.key, &bytes);
                    self.resident.insert(delivered.key, resident);
                }
                Err(_) => {
                    self.failed.insert(delivered.key);
                }
            }
        }

        // For each visible tile: touch if resident, else enqueue load.
        for key in &visible {
            if let Some(r) = self.resident.get_mut(key) {
                r.last_touched_frame = self.frame_counter;
            } else if !self.in_flight.contains(key) && !self.failed.contains(key) && self.loader.request(*key) {
                self.in_flight.insert(*key);
            }
        }

        // LRU eviction if over budget.
        if self.resident.len() > MAX_RESIDENT_TILES {
            let mut by_age: Vec<(TileKey, u64)> = self
                .resident
                .iter()
                .map(|(k, r)| (*k, r.last_touched_frame))
                .collect();
            by_age.sort_by_key(|(_, age)| *age);
            let to_drop = self.resident.len() - MAX_RESIDENT_TILES;
            for (key, _) in by_age.into_iter().take(to_drop) {
                self.resident.remove(&key);
            }
        }

        visible
    }

    /// Encode tile draws into the active render pass. Called between the
    /// pass clear and the footprint draws.
    pub fn encode<'pass>(
        &'pass self,
        pass: &mut wgpu::RenderPass<'pass>,
        camera_bg: &'pass wgpu::BindGroup,
        visible: &[TileKey],
    ) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, camera_bg, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        let index_count = u32::try_from(TILE_INDICES.len()).expect("6 indices fit u32");
        for key in visible {
            if let Some(r) = self.resident.get(key) {
                pass.set_bind_group(1, &r.bind_group, &[]);
                pass.draw_indexed(0..index_count, 0, 0..1);
            }
        }
    }

    fn create_resident_tile(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        key: TileKey,
        rgba: &[u8],
    ) -> ResidentTile {
        let size = wgpu::Extent3d {
            width: crate::tiles::coord::TILE_PIXEL_SIZE,
            height: crate::tiles::coord::TILE_PIXEL_SIZE,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("scim-render tile texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            rgba,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(crate::tiles::coord::TILE_PIXEL_SIZE * 4),
                rows_per_image: Some(crate::tiles::coord::TILE_PIXEL_SIZE),
            },
            size,
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let aabb = crate::tiles::coord::tile_world_aabb(key.zoom, key.x, key.y);
        let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("scim-render tile aabb uniform"),
            contents: bytemuck::bytes_of(&aabb),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("scim-render tile bg"),
            layout: &self.tile_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: uniform.as_entire_binding(),
                },
            ],
        });

        ResidentTile {
            _texture: texture,
            _view: view,
            _uniform: uniform,
            bind_group,
            last_touched_frame: self.frame_counter,
        }
    }
}
