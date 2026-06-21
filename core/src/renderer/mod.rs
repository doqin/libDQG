pub mod types;
mod draw_pass;

pub use draw_pass::DrawPass;
pub use types::*;

use crate::types::Color;

// ===== Renderer =====

pub struct Renderer<'a> {
    device: wgpu::Device,
    surface: wgpu::Surface<'a>,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
}

impl<'a> Renderer<'a> {
    pub async fn new(window: std::sync::Arc<winit::window::Window>) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::default();
        let surface = match instance.create_surface(window.clone()) {
            Ok(surface) => surface,
            Err(e) => panic!("Failed to create surface: {:?}", e),
        };

        let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }).await.expect("Failed to find an appropriate adapter");

        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("Device Descriptor"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                experimental_features: wgpu::ExperimentalFeatures::default(),
                memory_hints: wgpu::MemoryHints::default(),
                trace: wgpu::Trace::default(),
            }
        ).await.expect("Failed to create device");

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps.formats.iter().copied().find(|f| f.is_srgb()).unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        Self {
            device,
            surface,
            queue,
            config,
            size,
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    pub fn surface_format(&self) -> TextureFormat {
        TextureFormat::from_wgpu(self.config.format)
    }

    pub fn render(&mut self, clear_color: Color, draw_fn: impl FnOnce(&mut DrawPass)) {
        let output = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(surface_texture) => surface_texture,
            wgpu::CurrentSurfaceTexture::Suboptimal(surface_texture) => surface_texture,
            wgpu::CurrentSurfaceTexture::Timeout => panic!("Surface timeout"),
            wgpu::CurrentSurfaceTexture::Occluded => panic!("Surface occluded"),
            wgpu::CurrentSurfaceTexture::Outdated => panic!("Surface outdated"),
            wgpu::CurrentSurfaceTexture::Lost => panic!("Surface lost"),
            wgpu::CurrentSurfaceTexture::Validation => todo!(),
        };
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        {
            let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(clear_color.as_wgpu_color()),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            let mut draw_pass = DrawPass { pass: render_pass };
            draw_fn(&mut draw_pass);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
    }

    pub fn create_shader_module(&self, desc: &ShaderModuleDescriptor) -> ShaderModule {
        let module = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: desc.label,
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(desc.source)),
        });
        ShaderModule(module)
    }

    pub fn create_render_pipeline(&self, desc: &RenderPipelineDescriptor) -> RenderPipeline {
        let attrs_per_buffer: Vec<Vec<wgpu::VertexAttribute>> = desc.vertex.buffers.iter()
            .map(|vb| {
                vb.attributes.iter().map(|a| wgpu::VertexAttribute {
                    format: a.format.to_wgpu(),
                    offset: a.offset,
                    shader_location: a.shader_location,
                }).collect()
            })
            .collect();

        let vertex_buffers: Vec<wgpu::VertexBufferLayout> = desc.vertex.buffers.iter().enumerate()
            .map(|(i, vb)| wgpu::VertexBufferLayout {
                array_stride: vb.array_stride,
                step_mode: vb.step_mode.to_wgpu(),
                attributes: &attrs_per_buffer[i],
            })
            .collect();

        let primitive = wgpu::PrimitiveState {
            topology: desc.primitive.topology.to_wgpu(),
            strip_index_format: desc.primitive.strip_index_format.map(|f| f.to_wgpu()),
            front_face: desc.primitive.front_face.to_wgpu(),
            cull_mode: desc.primitive.cull_mode.to_wgpu(),
            unclipped_depth: desc.primitive.unclipped_depth,
            polygon_mode: desc.primitive.polygon_mode.to_wgpu(),
            conservative: false,
        };

        let depth_stencil = desc.depth_stencil.as_ref().map(|ds| wgpu::DepthStencilState {
            format: ds.format.to_wgpu(),
            depth_write_enabled: Some(ds.depth_write_enabled),
            depth_compare: Some(ds.depth_compare.to_wgpu()),
            stencil: wgpu::StencilState {
                front: wgpu::StencilFaceState {
                    compare: ds.stencil.front.compare.to_wgpu(),
                    fail_op: ds.stencil.front.fail_op.to_wgpu(),
                    depth_fail_op: ds.stencil.front.depth_fail_op.to_wgpu(),
                    pass_op: ds.stencil.front.pass_op.to_wgpu(),
                },
                back: wgpu::StencilFaceState {
                    compare: ds.stencil.back.compare.to_wgpu(),
                    fail_op: ds.stencil.back.fail_op.to_wgpu(),
                    depth_fail_op: ds.stencil.back.depth_fail_op.to_wgpu(),
                    pass_op: ds.stencil.back.pass_op.to_wgpu(),
                },
                read_mask: ds.stencil.read_mask,
                write_mask: ds.stencil.write_mask,
            },
            bias: wgpu::DepthBiasState {
                constant: ds.bias.bias,
                slope_scale: ds.bias.slope_scale,
                clamp: ds.bias.clamp,
            },
        });

        let multisample = wgpu::MultisampleState {
            count: desc.multisample.count,
            mask: desc.multisample.mask,
            alpha_to_coverage_enabled: desc.multisample.alpha_to_coverage_enabled,
        };

        let color_targets: Vec<Option<wgpu::ColorTargetState>> = match &desc.fragment {
            Some(frag) => {
                frag.targets.iter().map(|t| {
                    Some(wgpu::ColorTargetState {
                        format: t.format.to_wgpu(),
                        blend: t.blend.as_ref().map(|b| wgpu::BlendState {
                            color: wgpu::BlendComponent {
                                src_factor: b.color.src_factor.to_wgpu(),
                                dst_factor: b.color.dst_factor.to_wgpu(),
                                operation: b.color.operation.to_wgpu(),
                            },
                            alpha: wgpu::BlendComponent {
                                src_factor: b.alpha.src_factor.to_wgpu(),
                                dst_factor: b.alpha.dst_factor.to_wgpu(),
                                operation: b.alpha.operation.to_wgpu(),
                            },
                        }),
                        write_mask: t.write_mask.to_wgpu(),
                    })
                }).collect()
            }
            None => vec![],
        };

        let wgpu_desc = wgpu::RenderPipelineDescriptor {
            label: desc.label,
            layout: None,
            vertex: wgpu::VertexState {
                module: &desc.vertex.module.0,
                entry_point: Some(desc.vertex.entry_point),
                buffers: &vertex_buffers,
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            primitive,
            depth_stencil,
            multisample,
            fragment: desc.fragment.as_ref().map(|frag| wgpu::FragmentState {
                module: &frag.module.0,
                entry_point: Some(frag.entry_point),
                targets: &color_targets,
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            multiview_mask: None,
            cache: None,
        };

        let pipeline = self.device.create_render_pipeline(&wgpu_desc);
        RenderPipeline(pipeline)
    }

    pub fn create_buffer_init(&self, desc: &BufferInitDescriptor) -> Buffer {
        let usage = desc.usage.to_wgpu() | wgpu::BufferUsages::COPY_DST;
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: desc.label,
            size: desc.contents.len() as u64,
            usage,
            mapped_at_creation: false,
        });
        self.queue.write_buffer(&buffer, 0, desc.contents);
        Buffer(buffer)
    }

    pub fn create_buffer(&self, desc: &BufferDescriptor) -> Buffer {
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: desc.label,
            size: desc.size,
            usage: desc.usage.to_wgpu(),
            mapped_at_creation: false,
        });
        Buffer(buffer)
    }
}
