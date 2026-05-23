//! `wgpu` renderer for the P1.5-b instanced footprint pass.

use std::path::PathBuf;
use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;
use winit::window::Window;

use scim_world::WorldIndex;

use crate::camera::Camera2d;
use crate::error::{Error, Result};
use crate::instance::Instance;
use crate::picking::PickPass;
use crate::tiles::TilePass;
use crate::ActorId;

/// Maximum instance count the renderer pre-allocates. 200k is comfortably
/// above the spec §6.6 "~150k visible" target with headroom; bump if the
/// 1 GB save target ever materializes.
const MAX_INSTANCES: usize = 200_000;

/// Vertex buffer layout: 4 corners of a unit quad in [-0.5, 0.5].
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct QuadVertex {
    pos: [f32; 2],
}

const QUAD_VERTICES: &[QuadVertex] = &[
    QuadVertex { pos: [-0.5, -0.5] },
    QuadVertex { pos: [0.5, -0.5] },
    QuadVertex { pos: [-0.5, 0.5] },
    QuadVertex { pos: [0.5, 0.5] },
];
const QUAD_INDICES: &[u16] = &[0, 1, 2, 2, 1, 3];

/// Camera UBO layout. Matches `struct CameraUniform { view_proj: mat4x4<f32> }`
/// in `shader.wgsl`. 64 B.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct CameraUniform {
    view_proj: [[f32; 4]; 4],
}

/// GPU renderer for the footprint pass.
#[derive(Debug)]
pub struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    instance_buffer: wgpu::Buffer,
    instance_count: u32,
    actor_ids: Vec<ActorId>,
    selection: Option<ActorId>,
    camera_uniform_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    pick_pass: PickPass,
    tile_pass: Option<TilePass>,
    surface_format: wgpu::TextureFormat,
    // reason: prefix matches existing camera_* fields; visual grouping of camera state
    #[allow(clippy::struct_field_names)]
    camera_bgl: wgpu::BindGroupLayout,
    // reason: prefix matches existing camera_* fields; visual grouping of camera state
    #[allow(clippy::struct_field_names)]
    camera_units_per_pixel: f32,
    // reason: prefix matches existing camera_* fields; visual grouping of camera state
    #[allow(clippy::struct_field_names)]
    camera_world_aabb: [f32; 4],
}

#[allow(clippy::too_many_lines)] // wgpu setup is inherently long; refactor when fragmenting into sub-functions makes the code clearer.
impl Renderer {
    /// Construct a renderer for the given window. The window must remain
    /// alive for the renderer's lifetime — pass an `Arc<Window>`.
    pub async fn new(window: Arc<Window>) -> Result<Self> {
        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = instance.create_surface(window)?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or(Error::Adapter)?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("scim-render device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: wgpu::MemoryHints::default(),
                },
                None,
            )
            .await
            .map_err(Error::Device)?;

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(wgpu::TextureFormat::is_srgb)
            .unwrap_or(surface_caps.formats[0]);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width,
            height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_config);

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("scim-render unit-quad vertex buffer"),
            contents: bytemuck::cast_slice(QUAD_VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("scim-render unit-quad index buffer"),
            contents: bytemuck::cast_slice(QUAD_INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("scim-render instance buffer"),
            size: u64::try_from(std::mem::size_of::<Instance>() * MAX_INSTANCES)
                .expect("instance buffer size fits in u64"),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let camera_uniform = CameraUniform {
            view_proj: [[0.0; 4]; 4],
        };
        let camera_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("scim-render camera uniform buffer"),
            contents: bytemuck::bytes_of(&camera_uniform),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("scim-render camera bgl"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("scim-render camera bg"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_uniform_buffer.as_entire_binding(),
            }],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("scim-render shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("scim-render pipeline layout"),
            bind_group_layouts: &[&camera_bind_group_layout],
            push_constant_ranges: &[],
        });

        let quad_stride = u64::try_from(std::mem::size_of::<QuadVertex>())
            .expect("QuadVertex stride fits in u64");
        let instance_stride =
            u64::try_from(std::mem::size_of::<Instance>()).expect("Instance stride fits in u64");

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("scim-render footprint pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
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
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
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

        let pick_pass = PickPass::new(
            &device,
            &shader,
            &camera_bind_group_layout,
            quad_stride,
            instance_stride,
            width,
            height,
        );

        Ok(Self {
            device,
            queue,
            surface,
            surface_config,
            pipeline,
            vertex_buffer,
            index_buffer,
            instance_buffer,
            instance_count: 0,
            actor_ids: Vec::new(),
            selection: None,
            camera_uniform_buffer,
            camera_bind_group,
            pick_pass,
            tile_pass: None,
            surface_format,
            camera_bgl: camera_bind_group_layout,
            camera_units_per_pixel: 1.0,
            camera_world_aabb: [0.0, 0.0, 0.0, 0.0],
        })
    }

    /// Resize the surface and pick texture in lockstep
    /// (call from `WindowEvent::Resized`).
    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.surface_config.width = width;
        self.surface_config.height = height;
        self.surface.configure(&self.device, &self.surface_config);
        self.pick_pass.resize(&self.device, width, height);
    }

    /// Upload a fresh instance buffer derived from `WorldIndex`. Walks the
    /// index in `iter()` order; the same order is used for the actor-id
    /// sidecar that backs picking. Counts above `MAX_INSTANCES` are
    /// truncated silently.
    pub fn upload_world(&mut self, world: &WorldIndex) {
        let mut instances: Vec<Instance> = Vec::with_capacity(world.len());
        let mut actor_ids: Vec<ActorId> = Vec::with_capacity(world.len());
        for placement in world.iter().take(MAX_INSTANCES) {
            instances.push(Instance {
                position: placement.position,
                flags: 0,
            });
            actor_ids.push(placement.actor_id);
        }
        let bytes = bytemuck::cast_slice(&instances);
        self.queue.write_buffer(&self.instance_buffer, 0, bytes);
        self.instance_count =
            u32::try_from(instances.len()).expect("count <= MAX_INSTANCES (200k) fits in u32");
        self.actor_ids = actor_ids;
        self.selection = None;
    }

    /// Update the camera uniform from a `Camera2d`. 64 B write.
    /// Also caches `units_per_pixel` and `world_aabb` for the tile pass.
    pub fn set_camera(&mut self, camera: &Camera2d) {
        let uniform = CameraUniform {
            view_proj: camera.view_proj(),
        };
        self.queue
            .write_buffer(&self.camera_uniform_buffer, 0, bytemuck::bytes_of(&uniform));
        self.camera_units_per_pixel = camera.units_per_pixel();
        self.camera_world_aabb = camera.world_aabb();
    }

    /// Pick the actor under `screen_xy` (window-pixel coordinates;
    /// origin = top-left, +Y = down). Blocks ~1–10 ms on the GPU readback;
    /// click is rare, so this is acceptable. Returns `None` if the cursor
    /// is over empty space or off-surface.
    // reason: GPU command-encoder + readback is inherently sequential; splitting hurts readability
    #[allow(clippy::too_many_lines)]
    pub fn pick(&mut self, screen_xy: [f32; 2]) -> Option<ActorId> {
        if self.instance_count == 0 {
            return None;
        }
        let surface_w = self.surface_config.width;
        let surface_h = self.surface_config.height;
        // reason: surface_w/h are window pixel counts well below 2^24; f32 represents them exactly
        #[allow(clippy::cast_precision_loss)]
        let x_bound = surface_w.saturating_sub(1) as f32;
        // reason: surface_w/h are window pixel counts well below 2^24; f32 represents them exactly
        #[allow(clippy::cast_precision_loss)]
        let y_bound = surface_h.saturating_sub(1) as f32;
        // reason: screen_xy[0] is clamped to [0, x_bound] (non-negative, < 2^32) before cast; no truncation or sign loss
        #[allow(clippy::cast_sign_loss)]
        #[allow(clippy::cast_possible_truncation)]
        let cx = screen_xy[0].clamp(0.0, x_bound) as u32;
        // reason: screen_xy[1] is clamped to [0, y_bound] (non-negative, < 2^32) before cast; no truncation or sign loss
        #[allow(clippy::cast_sign_loss)]
        #[allow(clippy::cast_possible_truncation)]
        let cy = screen_xy[1].clamp(0.0, y_bound) as u32;

        // ----- encode the scissored pick pass -----
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("scim-render pick encoder"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("scim-render pick pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: self.pick_pass.view(),
                    resolve_target: None,
                    ops: wgpu::Operations {
                        // Clear to 0 = "no hit" everywhere outside the scissor.
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 0.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(self.pick_pass.pipeline());
            pass.set_bind_group(0, &self.camera_bind_group, &[]);
            pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
            pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            // Scissor to the 1×1 pixel under the cursor: the rasteriser
            // only does the work needed to colour that pixel.
            pass.set_scissor_rect(cx, cy, 1, 1);
            let index_count = u32::try_from(QUAD_INDICES.len()).expect("6 indices fit u32");
            pass.draw_indexed(0..index_count, 0, 0..self.instance_count);
        }

        // ----- copy the single pixel into the staging buffer -----
        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture: self.pick_pass.texture(),
                mip_level: 0,
                origin: wgpu::Origin3d { x: cx, y: cy, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: self.pick_pass.staging(),
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(crate::picking::PICK_BYTES_PER_ROW),
                    rows_per_image: Some(1),
                },
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );

        self.queue.submit(std::iter::once(encoder.finish()));

        // ----- block on the readback -----
        let staging = self.pick_pass.staging();
        let slice = staging.slice(..u64::from(crate::picking::PICK_BYTES_PER_ROW));
        let (tx, rx) =
            std::sync::mpsc::channel::<std::result::Result<(), wgpu::BufferAsyncError>>();
        slice.map_async(wgpu::MapMode::Read, move |res| {
            // Channel send can only fail if the receiver was dropped; ignore.
            let _ = tx.send(res);
        });
        // Push the GPU work + map callback through.
        self.device.poll(wgpu::Maintain::Wait);
        match rx.recv() {
            Ok(Ok(())) => {}
            Ok(Err(_)) | Err(_) => {
                // Map failed or sender hung up — treat as a miss. Buffer
                // remains unmapped in this branch which is fine; next
                // call will map again.
                return None;
            }
        }

        let handle = {
            let mapped = slice.get_mapped_range();
            // First 4 bytes are the R32Uint pixel value.
            let bytes: [u8; 4] = [mapped[0], mapped[1], mapped[2], mapped[3]];
            u32::from_ne_bytes(bytes)
        };
        // Drop the mapped range before unmap (required by wgpu).
        staging.unmap();

        if handle == 0 {
            return None;
        }
        // Handle is 1-based.
        // reason: handle is at least 1 (checked above) so subtracting 1 cannot underflow
        let idx = (handle as usize) - 1;
        self.actor_ids.get(idx).copied()
    }

    /// Set the currently-selected actor (or clear with `None`). Cheap:
    /// flips at most two 4-byte slots in the instance buffer via
    /// `queue.write_buffer`. Out-of-bag IDs (not present in the last
    /// `upload_world`) clear the selection.
    pub fn set_selection(&mut self, actor_id: Option<ActorId>) {
        // Clear the old.
        if let Some(prev_id) = self.selection.take() {
            if let Some(prev_idx) = self.actor_index(prev_id) {
                self.write_flags(prev_idx, 0);
            }
        }
        // Set the new.
        if let Some(new_id) = actor_id {
            if let Some(new_idx) = self.actor_index(new_id) {
                self.write_flags(new_idx, Instance::FLAG_SELECTED);
                self.selection = Some(new_id);
            }
        }
    }

    /// The currently-selected actor, if any.
    #[must_use]
    pub const fn selection(&self) -> Option<ActorId> {
        self.selection
    }

    /// Set the root directory containing tile pyramid PNGs laid out as
    /// `{root}/{z}/{x}/{y}.png`. `None` disables tiles entirely; the
    /// renderer keeps working with the existing dark background under
    /// footprints. Safe to call mid-session.
    pub fn set_tile_root(&mut self, root: Option<PathBuf>) {
        // Drop the old pass (closes the loader channel, joins the worker).
        self.tile_pass = None;
        if let Some(root) = root {
            self.tile_pass = Some(TilePass::new(
                &self.device,
                &self.camera_bgl,
                self.surface_format,
                root,
            ));
        }
    }

    /// Linear scan over the actor-id sidecar. For the P1.5-c instance
    /// counts (< 200k) this is negligible; revisit if hover / multi-select
    /// stress it.
    fn actor_index(&self, actor_id: ActorId) -> Option<usize> {
        self.actor_ids.iter().position(|&id| id == actor_id)
    }

    /// Write a 4 B flags value at the right offset in the instance buffer.
    /// Offset within an Instance is 12 (after `position: [f32; 3]`); see
    /// `selection_tests::flag_offset_within_instance_is_12`.
    fn write_flags(&self, instance_index: usize, flags: u32) {
        let instance_stride =
            u64::try_from(std::mem::size_of::<Instance>()).expect("Instance stride fits in u64");
        let flag_offset_within_instance: u64 = 12;
        let buffer_offset = u64::try_from(instance_index).expect("instance_index fits in u64")
            * instance_stride
            + flag_offset_within_instance;
        self.queue.write_buffer(
            &self.instance_buffer,
            buffer_offset,
            bytemuck::bytes_of(&flags),
        );
    }

    /// Render one frame.
    pub fn render(&mut self) -> Result<()> {
        let frame = match self.surface.get_current_texture() {
            Ok(f) => f,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                self.surface.configure(&self.device, &self.surface_config);
                self.surface
                    .get_current_texture()
                    .expect("surface reconfigured")
            }
            Err(wgpu::SurfaceError::OutOfMemory | wgpu::SurfaceError::Timeout) => return Ok(()),
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Drive the tile pass cache + loader BEFORE encoder construction
        // so create_resident_tile can use &queue freely.
        let visible_tiles: Vec<crate::tiles::coord::TileKey> =
            if let Some(tp) = self.tile_pass.as_mut() {
                tp.update(
                    &self.device,
                    &self.queue,
                    self.camera_units_per_pixel,
                    self.camera_world_aabb,
                )
            } else {
                Vec::new()
            };

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("scim-render encoder"),
            });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("scim-render combined pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.08,
                            g: 0.08,
                            b: 0.10,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            // Tile pass (under footprints).
            if let Some(tp) = self.tile_pass.as_ref() {
                tp.encode(&mut pass, &self.camera_bind_group, &visible_tiles);
            }

            // Footprint pass.
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.camera_bind_group, &[]);
            pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
            pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            let index_count = u32::try_from(QUAD_INDICES.len()).expect("6 indices fit u32");
            pass.draw_indexed(0..index_count, 0, 0..self.instance_count);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
        Ok(())
    }
}

#[cfg(test)]
mod selection_tests {
    use super::*;

    #[test]
    fn flag_offset_within_instance_is_12() {
        // The flags field is the last 4 bytes of the 16 B Instance.
        // Selection writes go to `instance_index * 16 + 12`. Document the
        // invariant so the upload code stays in sync with Instance layout.
        assert_eq!(std::mem::size_of::<Instance>(), 16);
        assert_eq!(std::mem::offset_of!(Instance, flags), 12);
    }
}
