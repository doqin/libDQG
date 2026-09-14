use std::sync::Arc;

use libdqg::renderer::Renderer;

/// Bundles the `egui`/`egui-winit`/`egui-wgpu` plumbing (window handle, input state, the wgpu
/// renderer, and the frame's pending output) that every `egui`-driven `Scene` in this editor
/// needs, so scenes don't each reimplement lazy init and overlay compositing.
pub struct EguiLayer {
    window: Option<Arc<winit::window::Window>>,
    ctx: egui::Context,
    state: Option<egui_winit::State>,
    renderer: Option<egui_wgpu::Renderer>,
    pending_output: Option<egui::FullOutput>,
}

impl EguiLayer {
    pub fn new() -> Self {
        Self {
            window: None,
            ctx: egui::Context::default(),
            state: None,
            renderer: None,
            pending_output: None,
        }
    }

    fn ensure_init(&mut self, renderer: &Renderer) {
        if self.window.is_some() {
            return;
        }

        let window = renderer.window();
        self.state = Some(egui_winit::State::new(
            self.ctx.clone(),
            egui::ViewportId::ROOT,
            window.as_ref(),
            Some(window.scale_factor() as f32),
            None,
            None,
        ));
        self.renderer = Some(egui_wgpu::Renderer::new(
            renderer.device(),
            renderer.wgpu_surface_format(),
            egui_wgpu::RendererOptions::default(),
        ));
        self.window = Some(window);
    }

    /// Runs one egui frame, invoking `run_ui` to build the UI, and stashes the output for
    /// [`Self::render_overlay`] to composite later in the same frame.
    pub fn run(&mut self, renderer: &Renderer, run_ui: impl FnMut(&mut egui::Ui)) {
        self.ensure_init(renderer);

        let window = self.window.clone().unwrap();
        let state = self.state.as_mut().unwrap();
        let raw_input = state.take_egui_input(&window);
        let full_output = self.ctx.run_ui(raw_input, run_ui);
        state.handle_platform_output(&window, full_output.platform_output.clone());
        self.pending_output = Some(full_output);
    }

    pub fn wants_pointer_input(&self) -> bool {
        self.ctx.egui_wants_pointer_input()
    }

    pub fn on_window_event(&mut self, event: &winit::event::WindowEvent) {
        if let (Some(window), Some(state)) = (self.window.as_ref(), self.state.as_mut()) {
            let _ = state.on_window_event(window, event);
        }
    }

    pub fn render_overlay(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
    ) {
        let (Some(mut full_output), Some(egui_renderer), Some(window)) =
            (self.pending_output.take(), self.renderer.as_mut(), self.window.as_ref())
        else {
            return;
        };

        let pixels_per_point = full_output.pixels_per_point;
        let paint_jobs = self.ctx.tessellate(std::mem::take(&mut full_output.shapes), pixels_per_point);

        // `TexturesDelta` panics on drop if `set`/`free` aren't fully drained, so consume them
        // rather than iterating by reference.
        for (id, image_deltas) in full_output.textures_delta.set.drain() {
            for image_delta in &image_deltas {
                egui_renderer.update_texture(device, queue, id, image_delta);
            }
        }

        let size = window.inner_size();
        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [size.width, size.height],
            pixels_per_point,
        };

        egui_renderer.update_buffers(device, queue, encoder, &paint_jobs, &screen_descriptor);

        let mut rpass = encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("egui overlay pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            })
            .forget_lifetime();

        egui_renderer.render(&mut rpass, &paint_jobs, &screen_descriptor);
        drop(rpass);

        for id in full_output.textures_delta.free.drain() {
            egui_renderer.free_texture(&id);
        }
    }
}
