use std::collections::HashMap;
use std::path::{Path, PathBuf};

use libdqg::ecs::Entity;
use libdqg::glam;
use libdqg::scripting::{ScriptAttachment, ScriptList};
use libdqg::world::{Renderable, Transform, World};

use crate::editor_settings::EditorSettings;
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
    /// Play/Stop was clicked — starting/stopping the [`libdqg::scripting::ScriptRuntime`] needs
    /// `EditorScene`'s help (it owns the runtime and the pre-Play transform snapshot), unlike a
    /// script attachment/removal/reorder, which `ui.rs` can just apply to `world.scripts`
    /// directly since it needs no GPU load step.
    pub toggle_play: bool,
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
    is_playing: bool,
    script_errors: &[String],
    settings: &mut EditorSettings,
    renaming_script: &mut Option<PathBuf>,
    script_rename_buffer: &mut String,
    requests: &mut UiRequests,
) {
    draw_menu_bar(ui, project, world, entity_assets, is_playing, requests);
    draw_assets_panel(ui, project, assets_expanded, texture_previews, world, settings, renaming_script, script_rename_buffer);
    draw_hierarchy(ui, world, selected, renaming, rename_buffer, entity_assets, requests);
    draw_inspector(ui, world, selected, project, entity_assets, is_playing, requests);
    draw_script_error_overlay(ui, script_errors);
}

fn draw_menu_bar(
    ui: &mut egui::Ui,
    project: Option<&Project>,
    world: &World,
    entity_assets: &HashMap<Entity, RenderableAsset>,
    is_playing: bool,
    requests: &mut UiRequests,
) {
    egui::Panel::top("menu_bar_panel").show(ui, |ui| {
        ui.horizontal(|ui| {
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
                // Also disabled during Play: scripts can now spawn/despawn/rename entities and
                // attach scripts, all of which are meant to be ephemeral and revert on Stop (see
                // EditorScene::stop_play) — saving mid-Play would bake that Play-time state into
                // the scene file instead.
                ui.add_enabled_ui(project.is_some() && !is_playing, |ui| {
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

            // Plain text rather than a play/stop glyph — egui's default font doesn't cover
            // those and renders a tofu box instead.
            ui.add_enabled_ui(project.is_some(), |ui| {
                if ui.button(if is_playing { "Stop" } else { "Play" }).clicked() {
                    requests.toggle_play = true;
                }
            });

            if let Some(project) = project {
                ui.label(format!("Project: {}", project.manifest.name));
            }
        });
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
    world: &mut World,
    settings: &mut EditorSettings,
    renaming_script: &mut Option<PathBuf>,
    script_rename_buffer: &mut String,
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
                ui.separator();
                draw_script_group(ui, project, world, settings, renaming_script, script_rename_buffer);
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

fn draw_script_group(
    ui: &mut egui::Ui,
    project: &Project,
    world: &mut World,
    settings: &mut EditorSettings,
    renaming_script: &mut Option<PathBuf>,
    script_rename_buffer: &mut String,
) {
    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            draw_group_header(ui, "Scripts", &["rhai"], project);
            // Unlike Textures/Models, a script has nothing to import from outside the project —
            // it's authored here, so this creates one from scratch instead.
            if ui.small_button("New Script").clicked() {
                if let Err(e) = project.create_script() {
                    eprintln!("Failed to create script: {e}");
                }
            }
        });
        ui.horizontal_wrapped(|ui| {
            for path in project.list_scripts() {
                draw_script_tile(ui, project, &path, world, settings, renaming_script, script_rename_buffer);
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

/// A script has no thumbnail either (see `draw_model_tile`'s doc comment for the same reasoning),
/// so its tile is a plain placeholder icon, sized to match a texture/model tile. Unlike the
/// other asset tiles, this one is interactive: click opens `path` in an external text editor
/// (see [`open_script_in_editor`]) and the right-click context menu offers Rename (inline, like
/// the Hierarchy panel's entity rename) and a way to change the remembered editor.
fn draw_script_tile(
    ui: &mut egui::Ui,
    project: &Project,
    path: &Path,
    world: &mut World,
    settings: &mut EditorSettings,
    renaming_script: &mut Option<PathBuf>,
    script_rename_buffer: &mut String,
) {
    ui.vertical(|ui| {
        ui.set_width(ASSET_PREVIEW_SIZE);

        if renaming_script.as_deref() == Some(path) {
            let response = ui.text_edit_singleline(script_rename_buffer);
            if response.lost_focus() {
                rename_script(project, world, path, script_rename_buffer);
                *renaming_script = None;
            } else {
                response.request_focus();
            }
            return;
        }

        let (rect, response) =
            ui.allocate_exact_size(egui::vec2(ASSET_PREVIEW_SIZE, ASSET_PREVIEW_SIZE), egui::Sense::click());
        ui.painter().rect_filled(rect, 4.0, egui::Color32::from_gray(55));
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "RHAI",
            egui::FontId::proportional(12.0),
            egui::Color32::from_gray(200),
        );

        // A single click opens the script — no `double_clicked()` handling here, since a
        // double-click's first press already fires `clicked()` too, and rename lives in the
        // context menu instead (see the doc comment above) precisely to avoid that collision:
        // double-clicking to rename would otherwise also launch the editor once first.
        if response.clicked() {
            open_script_in_editor(project, path, settings);
        }

        response.context_menu(|ui| {
            if ui.button("Rename").clicked() {
                *renaming_script = Some(path.to_path_buf());
                *script_rename_buffer = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
                ui.close();
            }
            if ui.button("Change Editor...").clicked() {
                pick_and_save_editor(settings);
                ui.close();
            }
        });

        ui.add(egui::Label::new(asset_file_name(path)).wrap());
    });
}

/// Renames `old_path` (project-relative) to `new_stem` in place on disk, keeping its extension,
/// then fixes up every [`ScriptAttachment`] anywhere in `world` that referenced the old path —
/// otherwise every entity that had this script attached would silently start pointing at a file
/// that no longer exists. Refuses (and reports, rather than silently overwriting) if something
/// is already using the target name.
fn rename_script(project: &Project, world: &mut World, old_path: &Path, new_stem: &str) {
    let new_stem = new_stem.trim();
    if new_stem.is_empty() {
        return;
    }

    let extension = old_path.extension().map(|ext| ext.to_string_lossy().into_owned()).unwrap_or_default();
    let new_path = old_path.with_file_name(format!("{new_stem}.{extension}"));
    if new_path == old_path {
        return;
    }

    let old_absolute = project.root.join(old_path);
    let new_absolute = project.root.join(&new_path);
    if new_absolute.exists() {
        eprintln!("Failed to rename script: {} already exists", new_path.display());
        return;
    }

    if let Err(e) = std::fs::rename(&old_absolute, &new_absolute) {
        eprintln!("Failed to rename script: {e}");
        return;
    }

    for (_, list) in world.scripts.iter_mut() {
        for attachment in list.0.iter_mut() {
            if attachment.path == old_path {
                attachment.path = new_path.clone();
            }
        }
    }
}

/// Opens `path` (project-relative) in [`EditorSettings::preferred_editor`], prompting for one
/// first (and remembering the choice — see [`pick_and_save_editor`]) if none is set yet.
fn open_script_in_editor(project: &Project, path: &Path, settings: &mut EditorSettings) {
    if settings.preferred_editor.is_none() {
        pick_and_save_editor(settings);
    }
    let Some(editor) = settings.preferred_editor.as_ref() else { return };

    if let Err(e) = std::process::Command::new(editor).arg(project.root.join(path)).spawn() {
        eprintln!("Failed to launch {}: {e}", editor.display());
    }
}

/// Prompts for an executable via a native file picker and, if one is chosen, saves it as the
/// remembered editor (persisted next to the editor executable — see [`EditorSettings::save`]) so
/// future script clicks don't need to ask again.
fn pick_and_save_editor(settings: &mut EditorSettings) {
    let Some(editor) = rfd::FileDialog::new().set_title("Choose a text editor").pick_file() else { return };
    settings.preferred_editor = Some(editor);
    if let Err(e) = settings.save() {
        eprintln!("Failed to save editor preference: {e}");
    }
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
    is_playing: bool,
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

        // Editing a Transform in the Inspector while a script is writing to the same entity
        // every frame would visibly fight with it, so disable the numeric fields during Play.
        ui.add_enabled_ui(!is_playing, |ui| {
            draw_transform_editor(ui, world, entity);
        });
        ui.separator();
        draw_renderable_editor(ui, world, entity, project, entity_assets, requests);
        ui.separator();
        draw_script_editor(ui, world, entity, project);
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

/// The selected entity's stacked `.rhai` scripts: a reorderable/removable list of what's
/// attached (execution order top-to-bottom), plus a picker to attach more from the project's
/// `scripts/` folder. Unlike [`draw_renderable_editor`], attaching/detaching/reordering needs no
/// GPU load step, so this mutates `world.scripts` directly rather than going through
/// [`UiRequests`].
fn draw_script_editor(ui: &mut egui::Ui, world: &mut World, entity: Entity, project: Option<&Project>) {
    ui.label("Scripts");

    let mut move_up: Option<usize> = None;
    let mut move_down: Option<usize> = None;
    let mut remove: Option<usize> = None;

    if let Some(list) = world.scripts.get_mut(entity) {
        let last = list.0.len().saturating_sub(1);
        for (index, attachment) in list.0.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                ui.checkbox(&mut attachment.enabled, "");
                ui.label(asset_file_name(&attachment.path));
                // Plain text rather than up/down arrow glyphs — egui's default font doesn't
                // cover those and renders a tofu box instead.
                ui.add_enabled_ui(index > 0, |ui| {
                    if ui.small_button("Up").clicked() {
                        move_up = Some(index);
                    }
                });
                ui.add_enabled_ui(index < last, |ui| {
                    if ui.small_button("Down").clicked() {
                        move_down = Some(index);
                    }
                });
                if ui.small_button("Remove").clicked() {
                    remove = Some(index);
                }
            });
        }
    }

    if let Some(list) = world.scripts.get_mut(entity) {
        if let Some(index) = move_up.filter(|&i| i > 0) {
            list.0.swap(index, index - 1);
        }
        if let Some(index) = move_down.filter(|&i| i + 1 < list.0.len()) {
            list.0.swap(index, index + 1);
        }
        if let Some(index) = remove {
            list.0.remove(index);
        }
    }

    let Some(project) = project else {
        ui.label("Open or create a project to attach a script.");
        return;
    };

    ui.collapsing("Attach Script", |ui| {
        let scripts = project.list_scripts();
        if scripts.is_empty() {
            ui.label("No .rhai files in this project's scripts/ folder.");
        }
        for path in scripts {
            if ui.button(path.display().to_string()).clicked() {
                let attachment = ScriptAttachment { path, enabled: true };
                match world.scripts.get_mut(entity) {
                    Some(list) => list.0.push(attachment),
                    None => world.scripts.insert(entity, ScriptList(vec![attachment])),
                }
            }
        }
    });
}

/// A small transient toast listing recent script compile/runtime errors (see
/// `EditorScene`'s `script_errors`), so a broken script fails loudly without a full log panel.
fn draw_script_error_overlay(ui: &mut egui::Ui, script_errors: &[String]) {
    if script_errors.is_empty() {
        return;
    }

    egui::Area::new(egui::Id::new("script_error_overlay"))
        .anchor(egui::Align2::LEFT_BOTTOM, egui::vec2(8.0, -8.0))
        .show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_max_width(420.0);
                for message in script_errors {
                    ui.colored_label(egui::Color32::from_rgb(220, 90, 90), message);
                }
            });
        });
}
