use libdqg::ecs::Entity;
use libdqg::glam;
use libdqg::world::{Renderable, World};

/// Left hierarchy panel (click a name to select) + an inspector window for the selected entity's
/// `Renderable` kind and `Transform`, editable live.
pub fn draw(ui: &mut egui::Ui, world: &mut World, selected: &mut Option<Entity>) {
    egui::Panel::left("hierarchy_panel").show(ui, |ui| {
        ui.heading("Hierarchy");
        ui.separator();
        let entities: Vec<Entity> = world.renderables.iter().map(|(entity, _)| entity).collect();
        for entity in entities {
            let label = world
                .names
                .get(entity)
                .map(|name| name.0.clone())
                .unwrap_or_else(|| "<unnamed>".to_string());
            let is_selected = *selected == Some(entity);
            if ui.selectable_label(is_selected, label).clicked() {
                *selected = Some(entity);
            }
        }
    });

    if let Some(entity) = *selected {
        if !world.is_alive(entity) {
            *selected = None;
            return;
        }

        egui::Window::new("Inspector").show(ui.ctx(), |ui| {
            let kind = world.renderables.get(entity).map(|renderable| match renderable {
                Renderable::Sprite(_) => "Sprite",
                Renderable::Model(_) => "Model",
            });
            ui.label(format!("Kind: {}", kind.unwrap_or("<none>")));
            ui.separator();

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
        });
    }
}
