use std::collections::HashMap;
use std::sync::Arc;

use libdqg::camera::Camera;
use libdqg::ecs::Entity;
use libdqg::glam;
use libdqg::input::{InputState, MouseState};
use libdqg::renderer::{DrawPass, Model, Renderer, Sprite, Texture};
use libdqg::scene::{Scene, SceneTransition};
use libdqg::world::{Renderable, Transform, World};

use crate::fly_camera::FlyCamera;
use crate::picking;
use crate::project::{Project, RenderableAsset, RenderableKind};
use crate::ui;

pub struct EditorScene {
    world: World,
    selected: Option<Entity>,
    renaming: Option<Entity>,
    rename_buffer: String,
    project: Option<Project>,
    entity_assets: HashMap<Entity, RenderableAsset>,
    fly_camera: FlyCamera,
    last_mouse_pos: (f32, f32),
    seeded: bool,
    window: Option<Arc<winit::window::Window>>,
    egui_ctx: egui::Context,
    egui_state: Option<egui_winit::State>,
    egui_renderer: Option<egui_wgpu::Renderer>,
    pending_output: Option<egui::FullOutput>,
}

impl EditorScene {
    pub fn new() -> Self {
        let camera = Camera {
            eye: glam::Vec3::new(0.0, 1.5, 4.0),
            target: glam::Vec3::ZERO,
            up: glam::Vec3::Y,
            aspect: 1.0,
            fov: 45.0,
            znear: 0.1,
            zfar: 100.0,
        };

        Self {
            world: World::new(camera),
            selected: None,
            renaming: None,
            rename_buffer: String::new(),
            project: None,
            entity_assets: HashMap::new(),
            fly_camera: FlyCamera::new(6.0),
            last_mouse_pos: (0.0, 0.0),
            seeded: false,
            window: None,
            egui_ctx: egui::Context::default(),
            egui_state: None,
            egui_renderer: None,
            pending_output: None,
        }
    }

    fn seed_world(&mut self, renderer: &Renderer) {
        let mut model = Model::load(renderer, "./res/models/cube.obj").expect("Failed to load model");
        model.transform = glam::Mat4::IDENTITY;
        let cube = self.world.spawn_empty(
            "Cube",
            Transform { position: glam::Vec3::new(-1.2, 0.0, 0.0), ..Default::default() },
        );
        self.world.set_renderable(cube, Renderable::Model(model));

        let texture = Arc::new(Texture::from_path(renderer, "./res/textures/smug.png").expect("Failed to load texture"));
        let mut sprite = Sprite::new(texture);
        sprite.width = 1.0;
        sprite.height = 1.0;
        let sprite_entity = self.world.spawn_empty(
            "Sprite",
            Transform { position: glam::Vec3::new(1.2, 0.0, 0.0), ..Default::default() },
        );
        self.world.set_renderable(sprite_entity, Renderable::Sprite(sprite));
    }
}

impl Scene for EditorScene {
    fn update(
        &mut self,
        delta_time: f32,
        input_state: &InputState,
        mouse_state: &MouseState,
        renderer: Option<&mut Renderer>,
    ) -> SceneTransition {
        let Some(renderer) = renderer else { return SceneTransition::None };

        if self.window.is_none() {
            let window = renderer.window();
            self.egui_state = Some(egui_winit::State::new(
                self.egui_ctx.clone(),
                egui::ViewportId::ROOT,
                window.as_ref(),
                Some(window.scale_factor() as f32),
                None,
                None,
            ));
            self.egui_renderer = Some(egui_wgpu::Renderer::new(
                renderer.device(),
                renderer.wgpu_surface_format(),
                egui_wgpu::RendererOptions::default(),
            ));
            self.window = Some(window);
        }

        if self.project.is_none() && !self.seeded {
            self.seed_world(renderer);
            self.seeded = true;
        }

        let window = self.window.clone().unwrap();
        let egui_state = self.egui_state.as_mut().unwrap();
        let raw_input = egui_state.take_egui_input(&window);

        let mouse_pos = mouse_state.position();
        let mouse_delta = (mouse_pos.0 - self.last_mouse_pos.0, mouse_pos.1 - self.last_mouse_pos.1);
        self.last_mouse_pos = mouse_pos;

        let world = &mut self.world;
        let selected = &mut self.selected;
        let renaming = &mut self.renaming;
        let rename_buffer = &mut self.rename_buffer;
        let project = self.project.as_ref();
        let entity_assets = &mut self.entity_assets;
        let mut requests = ui::UiRequests::default();
        let full_output = self.egui_ctx.run_ui(raw_input, |ui| {
            ui::draw(ui, world, selected, renaming, rename_buffer, project, entity_assets, &mut requests);
        });

        egui_state.handle_platform_output(&window, full_output.platform_output.clone());

        if let Some(root) = requests.new_project.take() {
            match Project::create(root) {
                Ok(project) => {
                    self.world = World::new(self.world.camera);
                    self.entity_assets.clear();
                    self.selected = None;
                    self.project = Some(project);
                }
                Err(e) => eprintln!("Failed to create project: {e}"),
            }
        }

        if let Some(root) = requests.open_project.take() {
            match Project::open(root).and_then(|project| project.load_scene().map(|scene| (project, scene))) {
                Ok((project, scene_file)) => {
                    self.world = World::new(self.world.camera);
                    self.entity_assets.clear();
                    self.selected = None;
                    for record in scene_file.entities {
                        let entity = self.world.spawn_empty(record.name, record.transform);
                        if let Some(asset) = record.renderable {
                            match asset.load(renderer, &project) {
                                Ok(renderable) => {
                                    self.world.set_renderable(entity, renderable);
                                    self.entity_assets.insert(entity, asset);
                                }
                                Err(e) => eprintln!("Failed to load renderable: {e}"),
                            }
                        }
                    }
                    self.project = Some(project);
                }
                Err(e) => eprintln!("Failed to open project: {e}"),
            }
        }

        if let Some((entity, kind, path)) = requests.attach_renderable.take() {
            if let Some(project) = self.project.as_ref() {
                let probe_asset = match kind {
                    RenderableKind::Sprite => {
                        RenderableAsset::Sprite { texture_path: path.clone(), width: 0.0, height: 0.0 }
                    }
                    RenderableKind::Model => RenderableAsset::Model { model_path: path.clone() },
                };
                match probe_asset.load(renderer, project) {
                    Ok(renderable) => {
                        // Re-derive the sprite's width/height from the loaded texture's native
                        // size rather than trusting the placeholder above.
                        let asset = match &renderable {
                            Renderable::Sprite(sprite) => {
                                RenderableAsset::Sprite { texture_path: path, width: sprite.width, height: sprite.height }
                            }
                            Renderable::Model(_) => RenderableAsset::Model { model_path: path },
                        };
                        self.world.set_renderable(entity, renderable);
                        self.entity_assets.insert(entity, asset);
                    }
                    Err(e) => eprintln!("Failed to attach renderable: {e}"),
                }
            }
        }

        if !self.egui_ctx.egui_wants_pointer_input() {
            self.fly_camera.update(delta_time, input_state, mouse_state, &mut self.world.camera, mouse_delta);

            if mouse_state.is_button_pressed(winit::event::MouseButton::Left) {
                let size = window.inner_size();
                let ndc_x = (mouse_pos.0 / (size.width.max(1) as f32)) * 2.0 - 1.0;
                let ndc_y = 1.0 - (mouse_pos.1 / (size.height.max(1) as f32)) * 2.0;
                self.selected = picking::pick(&self.world, &self.world.camera, ndc_x, ndc_y);
            }
        }

        self.world.sync_transforms();
        let size = renderer.size();
        self.world.camera.aspect = size.width as f32 / (size.height.max(1) as f32);
        *renderer.camera_mut() = self.world.camera;

        self.pending_output = Some(full_output);

        SceneTransition::None
    }

    fn render(&mut self, pass: &mut DrawPass) {
        for (_, renderable) in self.world.renderables.iter() {
            match renderable {
                Renderable::Sprite(sprite) => sprite.draw_world(pass),
                Renderable::Model(model) => pass.draw_model(model),
            }
        }
    }

    fn on_window_event(&mut self, event: &winit::event::WindowEvent) {
        if let (Some(window), Some(egui_state)) = (self.window.as_ref(), self.egui_state.as_mut()) {
            let _ = egui_state.on_window_event(window, event);
        }
    }

    fn render_overlay(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        let (Some(mut full_output), Some(egui_renderer), Some(window)) =
            (self.pending_output.take(), self.egui_renderer.as_mut(), self.window.as_ref())
        else {
            return;
        };

        let pixels_per_point = full_output.pixels_per_point;
        let paint_jobs = self.egui_ctx.tessellate(std::mem::take(&mut full_output.shapes), pixels_per_point);

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
