//! Shape pipeline: draws the tessellated model (world mm) via a camera uniform.

use lyon::tessellation::VertexBuffers;
use wgpu::util::DeviceExt;

use super::vertex::Vertex;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    transform: [f32; 4],
}

pub struct Renderer {
    pipeline: wgpu::RenderPipeline,
    camera_buf: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    /// Screen-space transform (px -> NDC) for the UI/chrome layer.
    screen_buf: wgpu::Buffer,
    screen_bind_group: wgpu::BindGroup,
    model: DynBuffers,
    overlay: DynBuffers,
    /// Screen-space UI quads (help panel etc.), drawn on top with `screen_bind_group`.
    ui: DynBuffers,
}

/// Vertex+index buffers that persist across frames and reallocate only when the
/// geometry outgrows their capacity — updated in place via `queue.write_buffer`.
/// This keeps the per-cursor-move overlay update allocation-free in steady state.
struct DynBuffers {
    label: &'static str,
    vbuf: Option<wgpu::Buffer>,
    ibuf: Option<wgpu::Buffer>,
    vcap: u64,
    icap: u64,
    index_count: u32,
}

impl Renderer {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shape shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shape.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("camera bgl"),
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

        let camera_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("camera uniform"),
            contents: bytemuck::cast_slice(&[CameraUniform {
                transform: [1.0, 1.0, 0.0, 0.0],
            }]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("camera bg"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buf.as_entire_binding(),
            }],
        });

        let screen_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("screen uniform"),
            contents: bytemuck::cast_slice(&[CameraUniform {
                transform: [1.0, 1.0, 0.0, 0.0],
            }]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let screen_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("screen bg"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: screen_buf.as_entire_binding(),
            }],
        });

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("shape layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("shape pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                buffers: &[Vertex::LAYOUT],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        Self {
            pipeline,
            camera_buf,
            camera_bind_group,
            screen_buf,
            screen_bind_group,
            model: DynBuffers::new("model"),
            overlay: DynBuffers::new("overlay"),
            ui: DynBuffers::new("ui"),
        }
    }

    pub fn set_camera(&self, queue: &wgpu::Queue, transform: [f32; 4]) {
        queue.write_buffer(
            &self.camera_buf,
            0,
            bytemuck::cast_slice(&[CameraUniform { transform }]),
        );
    }

    /// Set the screen-space transform (physical px -> NDC) for the UI layer.
    pub fn set_screen(&self, queue: &wgpu::Queue, transform: [f32; 4]) {
        queue.write_buffer(
            &self.screen_buf,
            0,
            bytemuck::cast_slice(&[CameraUniform { transform }]),
        );
    }

    pub fn upload_ui(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        geometry: &VertexBuffers<Vertex, u32>,
    ) {
        self.ui.write(device, queue, geometry);
    }

    pub fn upload_model(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        geometry: &VertexBuffers<Vertex, u32>,
    ) {
        self.model.write(device, queue, geometry);
    }

    pub fn upload_overlay(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        geometry: &VertexBuffers<Vertex, u32>,
    ) {
        self.overlay.write(device, queue, geometry);
    }

    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.camera_bind_group, &[]);
        self.model.draw(pass);
        self.overlay.draw(pass);
        // Screen-space UI on top (help panel etc.).
        pass.set_bind_group(0, &self.screen_bind_group, &[]);
        self.ui.draw(pass);
    }
}

impl DynBuffers {
    fn new(label: &'static str) -> Self {
        Self {
            label,
            vbuf: None,
            ibuf: None,
            vcap: 0,
            icap: 0,
            index_count: 0,
        }
    }

    fn write(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        geometry: &VertexBuffers<Vertex, u32>,
    ) {
        self.index_count = geometry.indices.len() as u32;
        if geometry.indices.is_empty() {
            return;
        }
        let vbytes: &[u8] = bytemuck::cast_slice(&geometry.vertices);
        let ibytes: &[u8] = bytemuck::cast_slice(&geometry.indices);

        // Grow (never shrink) each buffer to the next power of two on overflow.
        if self.vbuf.is_none() || vbytes.len() as u64 > self.vcap {
            self.vcap = (vbytes.len() as u64).next_power_of_two().max(1024);
            self.vbuf = Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(self.label),
                size: self.vcap,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
        }
        if self.ibuf.is_none() || ibytes.len() as u64 > self.icap {
            self.icap = (ibytes.len() as u64).next_power_of_two().max(1024);
            self.ibuf = Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(self.label),
                size: self.icap,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
        }
        queue.write_buffer(self.vbuf.as_ref().unwrap(), 0, vbytes);
        queue.write_buffer(self.ibuf.as_ref().unwrap(), 0, ibytes);
    }

    fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        if self.index_count == 0 {
            return;
        }
        let (Some(vbuf), Some(ibuf)) = (self.vbuf.as_ref(), self.ibuf.as_ref()) else {
            return;
        };
        pass.set_vertex_buffer(0, vbuf.slice(..));
        pass.set_index_buffer(ibuf.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..self.index_count, 0, 0..1);
    }
}
