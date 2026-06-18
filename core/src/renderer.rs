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

        // Request a GPU adapter
        let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }).await.expect("Failed to find an appropriate adapter");

        // Request logical device and command queue from the adapter
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

        // Configure the surface
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

    pub fn clear(&mut self) {
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
            let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.1, g: 0.2, b: 0.3, a: 1.0 }), // Cornflower blue background
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            // Rendering commands would go here
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
    }
}