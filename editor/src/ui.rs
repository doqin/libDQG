use std::collections::HashMap;
use std::path::{Path, PathBuf};

use libdqg::ecs::Entity;
use libdqg::glam;
use libdqg::scripting::{ScriptAttachment, ScriptList};
use libdqg::world::{CameraComponent, Renderable, Transform, World};

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
    /// "Export..." was clicked and a destination folder chosen — building the standalone game
    /// needs `EditorScene`'s open [`Project`], which `ui.rs` doesn't own.
    pub export_project: Option<PathBuf>,
    /// A scene tile in the Assets panel's Scenes section was double-clicked, or a tab in the
    /// scene tab strip was clicked — `EditorScene`'s own `switch_or_open_scene` method needs to
    /// run (it owns every open tab's `World`/`project`), same reason `toggle_play`/
    /// `export_project` are requests instead of handled here directly.
    pub switch_scene: Option<PathBuf>,
    /// A tab's close button was clicked — `EditorScene`'s own `close_scene_tab` needs to run (it
    /// owns the tab list and has to autosave before dropping one).
    pub close_scene_tab: Option<PathBuf>,
    /// A scene tile's inline rename was committed (`(old_path, new_stem)`) — needs `EditorScene`'s
    /// help since renaming may also need to update `ProjectManifest.start_scene` and any open
    /// tab's identity, neither of which `ui.rs` owns.
    pub rename_scene: Option<(PathBuf, String)>,
    /// A script tile's inline rename was committed (`(old_path, new_stem)`) — deferred to
    /// `EditorScene::rename_script`, which fixes up every open tab's `World` and every scene file
    /// on disk that references the old path, not just whichever `World` happens to be active —
    /// `ui.rs` only ever sees one `World` at a time.
    pub rename_script: Option<(PathBuf, String)>,
    /// A scene tile's "Set as Start Scene" context-menu item was clicked — needs `&mut Project` to
    /// persist `ProjectManifest.start_scene`, which `ui.rs` only ever sees as `&Project`.
    pub set_start_scene: Option<PathBuf>,
    /// Something in the active tab's `World`/entity-asset map was mutated this frame — set by
    /// every editing widget below (transform drag, hierarchy add/rename/duplicate/delete, camera/
    /// renderable/script editing) so `EditorScene::update` can mark that tab dirty. Not raised for
    /// `attach_renderable`, which is itself a request `EditorScene` marks dirty when it applies it.
    pub edited: bool,
    /// "Save" was clicked — deferred to `EditorScene` (rather than saved directly here, unlike
    /// before tabs existed) because `ui.rs` doesn't own the dirty flag it needs to clear on
    /// success.
    pub save_scene: bool,
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
    renaming_scene: &mut Option<PathBuf>,
    scene_rename_buffer: &mut String,
    export_status: Option<&str>,
    current_scene: &Path,
    current_scene_dirty: bool,
    open_scenes: &[(PathBuf, bool)],
    active_scene: usize,
    requests: &mut UiRequests,
) {
    draw_menu_bar(ui, project, is_playing, current_scene_dirty, requests);
    draw_scene_tabs(ui, open_scenes, active_scene, requests);
    draw_assets_panel(
        ui,
        project,
        assets_expanded,
        texture_previews,
        settings,
        renaming_script,
        script_rename_buffer,
        renaming_scene,
        scene_rename_buffer,
        current_scene,
        requests,
    );
    draw_hierarchy(ui, world, selected, renaming, rename_buffer, entity_assets, requests);
    draw_inspector(ui, world, selected, project, entity_assets, is_playing, requests);
    draw_script_error_overlay(ui, script_errors);
    draw_export_status_overlay(ui, export_status);
}

/// Tab strip for the project's open scenes (`EditorScene::open_scenes`), one tab per scene kept
/// loaded in memory — clicking a tab switches to it instantly (no reload: unlike the old
/// single-scene behavior, an opened scene stays fully loaded until its tab is closed). Each tab's
/// `bool` marks it dirty (unsaved edits — see `EditorScene::OpenScene::dirty`), shown as a
/// trailing `*` on its label. The close button ("x") removes a tab (autosaving it first, handled
/// by `EditorScene::close_scene_tab`), except when it's the only one open — closing while Playing
/// (if it's the active tab) stops Play first, same as switching does, so no separate disabled
/// state is needed here.
fn draw_scene_tabs(ui: &mut egui::Ui, open_scenes: &[(PathBuf, bool)], active_scene: usize, requests: &mut UiRequests) {
    egui::Panel::top("scene_tabs_panel").show(ui, |ui| {
        ui.horizontal(|ui| {
            for (index, (path, dirty)) in open_scenes.iter().enumerate() {
                let is_active = index == active_scene;
                let label = if *dirty { format!("{} *", asset_file_name(path)) } else { asset_file_name(path) };
                if ui.selectable_label(is_active, label).clicked() && !is_active {
                    requests.switch_scene = Some(path.clone());
                }
                if open_scenes.len() > 1 && ui.small_button("x").clicked() {
                    requests.close_scene_tab = Some(path.clone());
                }
                ui.separator();
            }
        });
    });
}

fn draw_menu_bar(ui: &mut egui::Ui, project: Option<&Project>, is_playing: bool, current_scene_dirty: bool, requests: &mut UiRequests) {
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
                        requests.save_scene = true;
                        ui.close();
                    }
                    if ui.button("Export...").clicked() {
                        if let Some(output_dir) = rfd::FileDialog::new().pick_folder() {
                            requests.export_project = Some(output_dir);
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
                let dirty_marker = if current_scene_dirty { " *" } else { "" };
                ui.label(format!("Project: {}{dirty_marker}", project.manifest.name));
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

        ui.menu_button("+ Add Entity", |ui| {
            if ui.button("Empty").clicked() {
                let name = format!("Entity {}", world.iter_entities().count() + 1);
                let entity = world.spawn_empty(name, Transform::default());
                *selected = Some(entity);
                *renaming = None;
                requests.edited = true;
                ui.close();
            }
            if ui.button("Camera").clicked() {
                let name = format!("Camera {}", world.iter_entities().count() + 1);
                let entity = world.spawn_empty(name, Transform::default());
                world.set_camera(entity, CameraComponent::default());
                *selected = Some(entity);
                *renaming = None;
                requests.edited = true;
                ui.close();
            }
        });
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
                    requests.edited = true;
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
                        if let Some(component) = world.cameras.get(entity).copied() {
                            world.set_camera(new_entity, component);
                        }
                        requests.edited = true;
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
            requests.edited = true;
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
    settings: &mut EditorSettings,
    renaming_script: &mut Option<PathBuf>,
    script_rename_buffer: &mut String,
    renaming_scene: &mut Option<PathBuf>,
    scene_rename_buffer: &mut String,
    current_scene: &Path,
    requests: &mut UiRequests,
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
                draw_script_group(ui, project, settings, renaming_script, script_rename_buffer, requests);
                ui.separator();
                draw_scene_group(ui, project, current_scene, renaming_scene, scene_rename_buffer, requests);
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
    settings: &mut EditorSettings,
    renaming_script: &mut Option<PathBuf>,
    script_rename_buffer: &mut String,
    requests: &mut UiRequests,
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
                draw_script_tile(ui, project, &path, settings, renaming_script, script_rename_buffer, requests);
            }
        });
    });
}

/// Mirrors [`draw_script_group`]'s shape: a project can hold several scene `.ron` files (see
/// `Project::list_scenes`), listed here with a way to create new ones from scratch (a scene, like
/// a script, has nothing to import from outside the project) and switch which one is currently
/// open for editing.
fn draw_scene_group(
    ui: &mut egui::Ui,
    project: &Project,
    current_scene: &Path,
    renaming_scene: &mut Option<PathBuf>,
    scene_rename_buffer: &mut String,
    requests: &mut UiRequests,
) {
    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            ui.label("Scenes");
            if ui.small_button("New Scene").clicked() {
                if let Err(e) = project.create_scene() {
                    eprintln!("Failed to create scene: {e}");
                }
            }
        });
        ui.horizontal_wrapped(|ui| {
            for path in project.list_scenes() {
                draw_scene_tile(ui, &path, current_scene, &project.manifest.start_scene, renaming_scene, scene_rename_buffer, requests);
            }
        });
    });
}

/// One tile: a placeholder icon (see `draw_model_tile`'s doc comment — no cheap thumbnail for a
/// scene either), highlighted if it's the scene currently open for editing and starred if it's
/// the project's start scene. Double-click switches to it (opening a new tab if it isn't already
/// open — see `EditorScene::switch_or_open_scene` — or just activating its existing tab, no
/// reload, if it is); the context menu offers Rename (inline, like the Scripts section's) and
/// "Set as Start Scene". Renaming only touches the file on disk, `ProjectManifest.start_scene`,
/// and any open tab's identity — unlike a `ScriptAttachment` path (structured ECS data
/// `rename_script` can fix up everywhere it's referenced), a `scene.change("path")` call is a
/// free-text string literal inside `.rhai` source that can't be auto-fixed, so a renamed scene's
/// incoming `scene.change(...)` calls (if any) are left dangling and need updating by hand.
fn draw_scene_tile(
    ui: &mut egui::Ui,
    path: &Path,
    current_scene: &Path,
    start_scene: &Path,
    renaming_scene: &mut Option<PathBuf>,
    scene_rename_buffer: &mut String,
    requests: &mut UiRequests,
) {
    ui.vertical(|ui| {
        ui.set_width(ASSET_PREVIEW_SIZE);

        if renaming_scene.as_deref() == Some(path) {
            let response = ui.text_edit_singleline(scene_rename_buffer);
            if response.lost_focus() {
                requests.rename_scene = Some((path.to_path_buf(), scene_rename_buffer.clone()));
                *renaming_scene = None;
            } else {
                response.request_focus();
            }
            return;
        }

        let (rect, response) =
            ui.allocate_exact_size(egui::vec2(ASSET_PREVIEW_SIZE, ASSET_PREVIEW_SIZE), egui::Sense::click());
        let is_open = path == current_scene;
        let fill = if is_open { egui::Color32::from_gray(80) } else { egui::Color32::from_gray(55) };
        ui.painter().rect_filled(rect, 4.0, fill);
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "RON",
            egui::FontId::proportional(12.0),
            egui::Color32::from_gray(200),
        );
        if path == start_scene {
            ui.painter().text(
                rect.right_top(),
                egui::Align2::RIGHT_TOP,
                "*",
                egui::FontId::proportional(14.0),
                egui::Color32::YELLOW,
            );
        }

        if response.double_clicked() {
            requests.switch_scene = Some(path.to_path_buf());
        }
        response.context_menu(|ui| {
            if ui.button("Rename").clicked() {
                *renaming_scene = Some(path.to_path_buf());
                *scene_rename_buffer = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
                ui.close();
            }
            if ui.button("Set as Start Scene").clicked() {
                requests.set_start_scene = Some(path.to_path_buf());
                ui.close();
            }
        });

        ui.add(egui::Label::new(asset_file_name(path)).wrap());
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
    settings: &mut EditorSettings,
    renaming_script: &mut Option<PathBuf>,
    script_rename_buffer: &mut String,
    requests: &mut UiRequests,
) {
    ui.vertical(|ui| {
        ui.set_width(ASSET_PREVIEW_SIZE);

        if renaming_script.as_deref() == Some(path) {
            let response = ui.text_edit_singleline(script_rename_buffer);
            if response.lost_focus() {
                // Deferred to `EditorScene::rename_script`, not handled here like the old
                // single-scene version of this rename was — a `ScriptAttachment` can be
                // referenced by entities in *any* scene file, not just whichever one is the
                // active tab's `World`, so fixing it up needs `Project`/every open tab together,
                // neither of which `ui.rs` owns.
                requests.rename_script = Some((path.to_path_buf(), script_rename_buffer.clone()));
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
            draw_transform_editor(ui, world, entity, requests);
        });
        ui.separator();
        draw_camera_editor(ui, world, entity, requests);
        ui.separator();
        draw_renderable_editor(ui, world, entity, project, entity_assets, requests);
        ui.separator();
        draw_script_editor(ui, world, entity, project, requests);
    });
}

fn draw_transform_editor(ui: &mut egui::Ui, world: &mut World, entity: Entity, requests: &mut UiRequests) {
    let Some(transform) = world.transforms.get_mut(entity) else { return };
    let mut edited = false;

    ui.label("Position");
    ui.horizontal(|ui| {
        edited |= ui.add(egui::DragValue::new(&mut transform.position.x).speed(0.05).prefix("x: ")).changed();
        edited |= ui.add(egui::DragValue::new(&mut transform.position.y).speed(0.05).prefix("y: ")).changed();
        edited |= ui.add(egui::DragValue::new(&mut transform.position.z).speed(0.05).prefix("z: ")).changed();
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
    edited |= changed;

    ui.label("Scale");
    ui.horizontal(|ui| {
        edited |= ui.add(egui::DragValue::new(&mut transform.scale.x).speed(0.05).prefix("x: ")).changed();
        edited |= ui.add(egui::DragValue::new(&mut transform.scale.y).speed(0.05).prefix("y: ")).changed();
        edited |= ui.add(egui::DragValue::new(&mut transform.scale.z).speed(0.05).prefix("z: ")).changed();
    });

    if edited {
        requests.edited = true;
    }
}

/// The selected entity's camera lens, if it has one: a checkbox to attach/detach a
/// [`CameraComponent`] (mirrors `draw_renderable_editor`'s Remove button), then FOV/near/far
/// fields and an "Active Camera" checkbox once attached. No GPU-load `UiRequests` indirection is
/// needed here like `draw_renderable_editor`'s attach flow — `requests` is only used to flag
/// `edited`.
fn draw_camera_editor(ui: &mut egui::Ui, world: &mut World, entity: Entity, requests: &mut UiRequests) {
    let mut enabled = world.cameras.get(entity).is_some();
    if ui.checkbox(&mut enabled, "Camera").changed() {
        if enabled {
            world.set_camera(entity, CameraComponent::default());
        } else {
            world.clear_camera(entity);
        }
        requests.edited = true;
    }
    if !enabled {
        return;
    }

    let mut activate: Option<bool> = None;
    if let Some(component) = world.cameras.get_mut(entity) {
        let mut edited = false;
        ui.horizontal(|ui| {
            edited |= ui.add(egui::DragValue::new(&mut component.fov).speed(0.5).prefix("FOV: ").range(1.0..=179.0)).changed();
        });
        ui.horizontal(|ui| {
            edited |= ui.add(egui::DragValue::new(&mut component.znear).speed(0.01).prefix("Near: ")).changed();
            edited |= ui.add(egui::DragValue::new(&mut component.zfar).speed(0.5).prefix("Far: ")).changed();
        });

        let mut active = component.active;
        if ui.checkbox(&mut active, "Active Camera (used in Play)").changed() {
            activate = Some(active);
            edited = true;
        }
        if edited {
            requests.edited = true;
        }
    }

    // Radio-button semantics: clear every other camera's flag when this one turns on, so
    // World::active_camera never has to arbitrate between two "active" entities in practice.
    match activate {
        Some(true) => {
            for (other, component) in world.cameras.iter_mut() {
                component.active = other == entity;
            }
        }
        Some(false) => {
            if let Some(component) = world.cameras.get_mut(entity) {
                component.active = false;
            }
        }
        None => {}
    }
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
            requests.edited = true;
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
/// [`UiRequests`] (beyond flagging `edited`).
fn draw_script_editor(ui: &mut egui::Ui, world: &mut World, entity: Entity, project: Option<&Project>, requests: &mut UiRequests) {
    ui.label("Scripts");

    let mut move_up: Option<usize> = None;
    let mut move_down: Option<usize> = None;
    let mut remove: Option<usize> = None;

    if let Some(list) = world.scripts.get_mut(entity) {
        let last = list.0.len().saturating_sub(1);
        for (index, attachment) in list.0.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                if ui.checkbox(&mut attachment.enabled, "").changed() {
                    requests.edited = true;
                }
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
            requests.edited = true;
        }
        if let Some(index) = move_down.filter(|&i| i + 1 < list.0.len()) {
            list.0.swap(index, index + 1);
            requests.edited = true;
        }
        if let Some(index) = remove {
            list.0.remove(index);
            requests.edited = true;
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
                requests.edited = true;
            }
        }
    });
}

/// A small transient toast reporting the outcome of the last Export (see `EditorScene`'s
/// `export_status`), success or failure — mirrors [`draw_script_error_overlay`]'s shape, just
/// for one message instead of a list.
fn draw_export_status_overlay(ui: &mut egui::Ui, export_status: Option<&str>) {
    let Some(message) = export_status else { return };

    egui::Area::new(egui::Id::new("export_status_overlay"))
        .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-8.0, -8.0))
        .show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_max_width(420.0);
                ui.label(message);
            });
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
