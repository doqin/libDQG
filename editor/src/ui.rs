use std::collections::HashMap;
use std::path::PathBuf;

use libdqg::ecs::Entity;
use libdqg::glam;
use libdqg::world::{Renderable, Transform, World};

use crate::project::{Project, RenderableAsset, RenderableKind};

/// Requests raised by the UI that need something `draw` doesn't have access to (a live
/// `&mut Renderer` for loading assets, or `EditorScene`'s ability to replace its whole `World`
/// when a project is opened) — consumed by `EditorScene::update` right after `draw` runs, while
/// the renderer is still in scope for that frame.
#[derive(Default)]
pub struct UiRequests {
    pub new_project: Option<PathBuf>,
    pub open_project: Option<PathBuf>,
    pub attach_renderable: Option<(Entity, RenderableKind, PathBuf)>,
}

/// Top menu bar (project New/Open/Save) + left hierarchy panel (add/remove/rename/select
/// entities) + right assets panel (import/list project assets) + an inspector window for the
/// selected entity's `Transform` and `Renderable` (attach/replace/remove), editable live.
pub fn draw(
    ui: &mut egui::Ui,
    world: &mut World,
    selected: &mut Option<Entity>,
    renaming: &mut Option<Entity>,
    rename_buffer: &mut String,
    project: Option<&Project>,
    entity_assets: &mut HashMap<Entity, RenderableAsset>,
    requests: &mut UiRequests,
) {
    draw_menu_bar(ui, project, world, entity_assets, requests);
    draw_hierarchy(ui, world, selected, renaming, rename_buffer, entity_assets);
    draw_assets_panel(ui, project);
    draw_inspector(ui, world, selected, project, entity_assets, requests);
}

fn draw_menu_bar(
    ui: &mut egui::Ui,
    project: Option<&Project>,
    world: &World,
    entity_assets: &HashMap<Entity, RenderableAsset>,
    requests: &mut UiRequests,
) {
    egui::Panel::top("menu_bar_panel").show(ui, |ui| {
        ui.menu_button("File", |ui| {
            if ui.button("New Project...").clicked() {
                if let Some(root) = rfd::FileDialog::new().pick_folder() {
                    requests.new_project = Some(root);
                }
                ui.close();
            }
            if ui.button("Open Project...").clicked() {
                if let Some(root) = rfd::FileDialog::new().pick_folder() {
                    requests.open_project = Some(root);
                }
                ui.close();
            }
            ui.add_enabled_ui(project.is_some(), |ui| {
                if ui.button("Save").clicked() {
                    if let Some(project) = project {
                        if let Err(e) = project.save_scene(world, entity_assets) {
                            eprintln!("Failed to save project: {e}");
                        }
                    }
                    ui.close();
                }
            });
        });
        if let Some(project) = project {
            ui.label(format!("Project: {}", project.manifest.name));
        }
    });
}

fn draw_hierarchy(
    ui: &mut egui::Ui,
    world: &mut World,
    selected: &mut Option<Entity>,
    renaming: &mut Option<Entity>,
    rename_buffer: &mut String,
    entity_assets: &mut HashMap<Entity, RenderableAsset>,
) {
    egui::Panel::left("hierarchy_panel").show(ui, |ui| {
        ui.heading("Hierarchy");
        ui.separator();

        if ui.button("+ Add Entity").clicked() {
            let name = format!("Entity {}", world.iter_entities().count() + 1);
            let entity = world.spawn_empty(name, Transform::default());
            *selected = Some(entity);
            *renaming = None;
        }
        ui.separator();

        let entities: Vec<Entity> = world.iter_entities().collect();
        let mut despawn_requested: Option<Entity> = None;
        for entity in entities {
            let label = world
                .names
                .get(entity)
                .map(|name| name.0.clone())
                .unwrap_or_else(|| "<unnamed>".to_string());

            ui.horizontal(|ui| {
                if *renaming == Some(entity) {
                    let response = ui.text_edit_singleline(rename_buffer);
                    if response.lost_focus() {
                        if let Some(name) = world.names.get_mut(entity) {
                            name.0 = rename_buffer.clone();
                        }
                        *renaming = None;
                    } else {
                        response.request_focus();
                    }
                } else {
                    let is_selected = *selected == Some(entity);
                    let response = ui.selectable_label(is_selected, label.clone());
                    if response.clicked() {
                        *selected = Some(entity);
                    }
                    if response.double_clicked() {
                        *renaming = Some(entity);
                        *rename_buffer = label;
                    }
                }

                if ui.small_button("x").clicked() {
                    despawn_requested = Some(entity);
                }
            });
        }

        if let Some(entity) = despawn_requested {
            world.despawn(entity);
            entity_assets.remove(&entity);
            if *selected == Some(entity) {
                *selected = None;
            }
            if *renaming == Some(entity) {
                *renaming = None;
            }
        }
    });
}

fn draw_assets_panel(ui: &mut egui::Ui, project: Option<&Project>) {
    egui::Panel::right("assets_panel").show(ui, |ui| {
        ui.heading("Assets");
        ui.separator();

        let Some(project) = project else {
            ui.label("Open or create a project to import assets.");
            return;
        };

        draw_asset_list(ui, project, "Textures", RenderableKind::Sprite, &["png", "jpg", "jpeg"]);
        ui.separator();
        draw_asset_list(ui, project, "Models", RenderableKind::Model, &["obj"]);
    });
}

fn draw_asset_list(ui: &mut egui::Ui, project: &Project, heading: &str, kind: RenderableKind, filter: &[&str]) {
    ui.label(heading);
    if ui.button("Import...").clicked() {
        if let Some(path) = rfd::FileDialog::new().add_filter(heading, filter).pick_file() {
            if let Err(e) = project.import_asset(&path) {
                eprintln!("Failed to import asset: {e}");
            }
        }
    }
    for path in project.list_assets(kind) {
        ui.label(path.display().to_string());
    }
}

fn draw_inspector(
    ui: &mut egui::Ui,
    world: &mut World,
    selected: &mut Option<Entity>,
    project: Option<&Project>,
    entity_assets: &mut HashMap<Entity, RenderableAsset>,
    requests: &mut UiRequests,
) {
    let Some(entity) = *selected else { return };
    if !world.is_alive(entity) {
        *selected = None;
        return;
    }

    egui::Window::new("Inspector").show(ui.ctx(), |ui| {
        draw_transform_editor(ui, world, entity);
        ui.separator();
        draw_renderable_editor(ui, world, entity, project, entity_assets, requests);
    });
}

fn draw_transform_editor(ui: &mut egui::Ui, world: &mut World, entity: Entity) {
    let Some(transform) = world.transforms.get_mut(entity) else { return };

    ui.label("Position");
    ui.horizontal(|ui| {
        ui.add(egui::DragValue::new(&mut transform.position.x).speed(0.05).prefix("x: "));
        ui.add(egui::DragValue::new(&mut transform.position.y).speed(0.05).prefix("y: "));
        ui.add(egui::DragValue::new(&mut transform.position.z).speed(0.05).prefix("z: "));
    });

    let (mut yaw, mut pitch, mut roll) = transform.rotation.to_euler(glam::EulerRot::YXZ);
    yaw = yaw.to_degrees();
    pitch = pitch.to_degrees();
    roll = roll.to_degrees();
    ui.label("Rotation (deg)");
    let mut changed = false;
    ui.horizontal(|ui| {
        changed |= ui.add(egui::DragValue::new(&mut pitch).speed(0.5).prefix("x: ")).changed();
        changed |= ui.add(egui::DragValue::new(&mut yaw).speed(0.5).prefix("y: ")).changed();
        changed |= ui.add(egui::DragValue::new(&mut roll).speed(0.5).prefix("z: ")).changed();
    });
    if changed {
        transform.rotation = glam::Quat::from_euler(
            glam::EulerRot::YXZ,
            yaw.to_radians(),
            pitch.to_radians(),
            roll.to_radians(),
        );
    }

    ui.label("Scale");
    ui.horizontal(|ui| {
        ui.add(egui::DragValue::new(&mut transform.scale.x).speed(0.05).prefix("x: "));
        ui.add(egui::DragValue::new(&mut transform.scale.y).speed(0.05).prefix("y: "));
        ui.add(egui::DragValue::new(&mut transform.scale.z).speed(0.05).prefix("z: "));
    });
}

fn draw_renderable_editor(
    ui: &mut egui::Ui,
    world: &mut World,
    entity: Entity,
    project: Option<&Project>,
    entity_assets: &mut HashMap<Entity, RenderableAsset>,
    requests: &mut UiRequests,
) {
    let kind = world.renderables.get(entity).map(|renderable| match renderable {
        Renderable::Sprite(_) => "Sprite",
        Renderable::Model(_) => "Model",
    });
    ui.label(format!("Renderable: {}", kind.unwrap_or("<none>")));

    if let Some(asset) = entity_assets.get(&entity) {
        let path = match asset {
            RenderableAsset::Sprite { texture_path, .. } => texture_path,
            RenderableAsset::Model { model_path } => model_path,
        };
        ui.label(format!("Asset: {}", path.display()));
        if ui.button("Remove").clicked() {
            world.clear_renderable(entity);
            entity_assets.remove(&entity);
            return;
        }
        ui.label("Change to:");
    }

    let Some(project) = project else {
        ui.label("Open or create a project to attach a renderable.");
        return;
    };

    ui.collapsing("Attach Sprite", |ui| {
        for path in project.list_assets(RenderableKind::Sprite) {
            if ui.button(path.display().to_string()).clicked() {
                requests.attach_renderable = Some((entity, RenderableKind::Sprite, path));
            }
        }
    });
    ui.collapsing("Attach Model", |ui| {
        for path in project.list_assets(RenderableKind::Model) {
            if ui.button(path.display().to_string()).clicked() {
                requests.attach_renderable = Some((entity, RenderableKind::Model, path));
            }
        }
    });
}
