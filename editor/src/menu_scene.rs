use libdqg::input::{InputState, MouseState};
use libdqg::renderer::{DrawPass, Renderer};
use libdqg::scene::{Scene, SceneTransition};

use crate::editor_scene::{EditorScene, PendingAction};
use crate::egui_layer::EguiLayer;
use crate::recent_projects::RecentProjects;

/// The editor's landing screen: create a new project, open an existing one, or reopen one from
/// the recent-projects list. Replaces itself with an `EditorScene` once a project is chosen.
pub struct MenuScene {
    egui: EguiLayer,
    recent: RecentProjects,
}

impl MenuScene {
    pub fn new() -> Self {
        Self { egui: EguiLayer::new(), recent: RecentProjects::load() }
    }
}

impl Scene for MenuScene {
    fn update(
        &mut self,
        _delta_time: f32,
        _input_state: &InputState,
        _mouse_state: &MouseState,
        renderer: Option<&mut Renderer>,
    ) -> SceneTransition {
        let Some(renderer) = renderer else { return SceneTransition::None };

        let recent = &self.recent;
        let mut action = None;
        self.egui.run(renderer, |ui| {
            action = draw(ui, recent);
        });

        match action {
            Some(action) => SceneTransition::Replace(Box::new(EditorScene::opening(action))),
            None => SceneTransition::None,
        }
    }

    fn render(&mut self, _pass: &mut DrawPass) {}

    fn on_window_event(&mut self, event: &winit::event::WindowEvent) {
        self.egui.on_window_event(event);
    }

    fn render_overlay(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        self.egui.render_overlay(device, queue, encoder, view);
    }
}

fn draw(ui: &mut egui::Ui, recent: &RecentProjects) -> Option<PendingAction> {
    let mut action = None;

    egui::CentralPanel::default().show(ui, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(48.0);
            ui.heading("libDQG Editor");
            ui.add_space(24.0);

            ui.horizontal(|ui| {
                if ui.button("New Project...").clicked() {
                    if let Some(root) = rfd::FileDialog::new().pick_folder() {
                        action = Some(PendingAction::New(root));
                    }
                }
                if ui.button("Open Project...").clicked() {
                    if let Some(root) = rfd::FileDialog::new().pick_folder() {
                        action = Some(PendingAction::Open(root));
                    }
                }
            });

            ui.add_space(32.0);
            ui.separator();
            ui.add_space(8.0);
            ui.label("Recent Projects");

            if recent.roots.is_empty() {
                ui.weak("No recent projects.");
            }

            egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                for root in &recent.roots {
                    let name = root
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_else(|| root.display().to_string());
                    ui.group(|ui| {
                        ui.vertical(|ui| {
                            if ui.selectable_label(false, name).clicked() {
                                action = Some(PendingAction::Open(root.clone()));
                            }
                            ui.weak(root.display().to_string());
                        });
                    });
                }
            });
        });
    });

    action
}
