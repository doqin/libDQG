use std::collections::HashMap;
use std::path::{Path, PathBuf};

use libdqg::ecs::Entity;
use libdqg::glam;
use libdqg::world::{Renderable, Transform, World};

use crate::project::{Project, RenderableAsset, RenderableKind};

/// Side length, in points, of an asset tile's preview image in the Assets panel.
const ASSET_PREVIEW_SIZE: f32 = 56.0;

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

/// Top menu bar (project New/Open/Save) + bottom assets panel (collapsible; import/preview
/// project assets) + left hierarchy panel (add/remove/rename/select entities) + right inspector
/// panel for the selected entity's `Transform` and `Renderable` (attach/replace/remove),
/// editable live.
pub fn draw(
    ui: &mut egui::Ui,
    world: &mut World,
    selected: &mut Option<Entity>,
    renaming: &mut Option<Entity>,
    rename_buffer: &mut String,
    project: Option<&Project>,
    entity_assets: &mut HashMap<Entity, RenderableAsset>,
    assets_expanded: &mut bool,
    texture_previews: &mut HashMap<PathBuf, egui::TextureHandle>,
    requests: &mut UiRequests,
) {
    draw_menu_bar(ui, project, world, entity_assets, requests);
    draw_assets_panel(ui, project, assets_expanded, texture_previews);
    draw_hierarchy(ui, world, selected, renaming, rename_buffer, entity_assets, requests);
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
    requests: &mut UiRequests
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
                continue;
            }

            let is_selected = *selected == Some(entity);
            let response = ui.selectable_label(is_selected, label.clone());
            if response.clicked() {
                *selected = Some(entity);
            }
            if response.double_clicked() {
                *renaming = Some(entity);
                *rename_buffer = label.clone();
            }
            // Right-click menu is the only way to rename/delete now that the row has no
            // dedicated buttons of its own.
            response.context_menu(|ui| {
                if ui.button("Rename").clicked() {
                    *selected = Some(entity);
                    *renaming = Some(entity);
                    *rename_buffer = label.clone();
                    ui.close();
                }
                if ui.button("Duplicate").clicked() {
                    if let Some(name) = world.names.get(entity) {
                        let new_name = format!("{} (copy)", name.0);
                        let new_entity = world.spawn_empty(new_name, *world.transforms.get(entity).unwrap_or(&Transform::default()));
                        if let Some(asset) = entity_assets.get(&entity) {
                            requests.attach_renderable = Some((new_entity, asset.kind(), asset.path().to_path_buf()));
                        }
                    }
                    ui.close();
                }
                if ui.button("Delete").clicked() {
                    despawn_requested = Some(entity);
                    ui.close();
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

fn draw_assets_panel(
    ui: &mut egui::Ui,
    project: Option<&Project>,
    assets_expanded: &mut bool,
    texture_previews: &mut HashMap<PathBuf, egui::TextureHandle>,
) {
    egui::Panel::bottom("assets_panel").show(ui, |ui| {
        ui.horizontal(|ui| {
            // Plain ASCII rather than a triangle glyph (`\u{25BC}`/`\u{25B6}`) — egui's default
            // font doesn't cover those and renders a tofu box instead.
            let toggle_icon = if *assets_expanded { "v" } else { ">" };
            if ui.small_button(toggle_icon).clicked() {
                *assets_expanded = !*assets_expanded;
            }
            ui.heading("Assets");
        });

        if !*assets_expanded {
            return;
        }
        ui.separator();

        let Some(project) = project else {
            ui.label("Open or create a project to import assets.");
            return;
        };

        egui::ScrollArea::horizontal().show(ui, |ui| {
            ui.horizontal(|ui| {
                draw_texture_group(ui, project, texture_previews);
                ui.separator();
                draw_model_group(ui, project);
            });
        });
    });
}

fn draw_texture_group(ui: &mut egui::Ui, project: &Project, previews: &mut HashMap<PathBuf, egui::TextureHandle>) {
    ui.vertical(|ui| {
        draw_group_header(ui, "Textures", &["png", "jpg", "jpeg"], project);
        ui.horizontal_wrapped(|ui| {
            for path in project.list_assets(RenderableKind::Sprite) {
                draw_texture_tile(ui, project, &path, previews);
            }
        });
    });
}

fn draw_model_group(ui: &mut egui::Ui, project: &Project) {
    ui.vertical(|ui| {
        draw_group_header(ui, "Models", &["obj"], project);
        ui.horizontal_wrapped(|ui| {
            for path in project.list_assets(RenderableKind::Model) {
                draw_model_tile(ui, &path);
            }
        });
    });
}

fn draw_group_header(ui: &mut egui::Ui, heading: &str, import_filter: &[&str], project: &Project) {
    ui.horizontal(|ui| {
        ui.label(heading);
        if ui.small_button("Import...").clicked() {
            if let Some(path) = rfd::FileDialog::new().add_filter(heading, import_filter).pick_file() {
                if let Err(e) = project.import_asset(&path) {
                    eprintln!("Failed to import asset: {e}");
                }
            }
        }
    });
}

/// One tile: the texture's own thumbnail (loaded through `egui`'s own texture manager, and
/// cached in `previews` so it's decoded from disk once rather than every frame) with the file
/// name below it.
fn draw_texture_tile(ui: &mut egui::Ui, project: &Project, path: &Path, previews: &mut HashMap<PathBuf, egui::TextureHandle>) {
    let handle = previews
        .entry(path.to_path_buf())
        .or_insert_with(|| load_texture_preview(ui.ctx(), &project.root.join(path)));

    // Reserve a fixed square footprint so tiles still line up in a grid, but paint the
    // thumbnail at its own aspect ratio (scaled to fit) inside it rather than stretching a
    // non-square image to fill the square.
    let fitted_size = fit_within_square(handle.size_vec2(), ASSET_PREVIEW_SIZE);

    ui.vertical(|ui| {
        ui.set_width(ASSET_PREVIEW_SIZE);
        let (slot_rect, _) = ui.allocate_exact_size(egui::vec2(ASSET_PREVIEW_SIZE, ASSET_PREVIEW_SIZE), egui::Sense::hover());
        let image_rect = egui::Rect::from_center_size(slot_rect.center(), fitted_size);
        egui::Image::new((handle.id(), fitted_size)).paint_at(ui, image_rect);
        ui.add(egui::Label::new(asset_file_name(path)).wrap());
    });
}

/// Scales `size` down (never up) to fit within a `max_side`-by-`max_side` square, preserving
/// its aspect ratio.
fn fit_within_square(size: egui::Vec2, max_side: f32) -> egui::Vec2 {
    if size.x <= 0.0 || size.y <= 0.0 {
        return egui::Vec2::splat(max_side);
    }
    let scale = (max_side / size.x).min(max_side / size.y).min(1.0);
    size * scale
}

fn load_texture_preview(ctx: &egui::Context, full_path: &Path) -> egui::TextureHandle {
    let color_image = image::open(full_path)
        .map(|image| {
            let thumbnail = image.thumbnail(64, 64).to_rgba8();
            let size = [thumbnail.width() as usize, thumbnail.height() as usize];
            egui::ColorImage::from_rgba_unmultiplied(size, thumbnail.as_raw())
        })
        .unwrap_or_else(|e| {
            eprintln!("Failed to load texture preview for {}: {e}", full_path.display());
            egui::ColorImage::filled([1, 1], egui::Color32::from_gray(80))
        });

    ctx.load_texture(full_path.display().to_string(), color_image, egui::TextureOptions::LINEAR)
}

/// A model has no cheap thumbnail to render (that would need an offscreen 3D render pass per
/// asset), so its tile is a plain placeholder icon instead, sized to match a texture tile.
fn draw_model_tile(ui: &mut egui::Ui, path: &Path) {
    ui.vertical(|ui| {
        ui.set_width(ASSET_PREVIEW_SIZE);
        let (rect, _) = ui.allocate_exact_size(egui::vec2(ASSET_PREVIEW_SIZE, ASSET_PREVIEW_SIZE), egui::Sense::hover());
        ui.painter().rect_filled(rect, 4.0, egui::Color32::from_gray(55));
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "OBJ",
            egui::FontId::proportional(14.0),
            egui::Color32::from_gray(200),
        );
        ui.add(egui::Label::new(asset_file_name(path)).wrap());
    });
}

fn asset_file_name(path: &Path) -> String {
    path.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default()
}

fn draw_inspector(
    ui: &mut egui::Ui,
    world: &mut World,
    selected: &mut Option<Entity>,
    project: Option<&Project>,
    entity_assets: &mut HashMap<Entity, RenderableAsset>,
    requests: &mut UiRequests,
) {
    if let Some(entity) = *selected
        && !world.is_alive(entity)
    {
        *selected = None;
    }

    egui::Panel::right("inspector_panel").show(ui, |ui| {
        ui.heading("Inspector");
        ui.separator();

        let Some(entity) = *selected else {
            ui.label("No entity selected.");
            return;
        };

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
