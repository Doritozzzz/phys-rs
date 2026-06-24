use bevy_ecs::prelude::Entity;
use crate::render::ui::EntityHudData;
use crate::render::ui::actions::UiAction;

pub fn draw(
    ctx: &egui::Context,
    entity: Entity,
    data: &EntityHudData,
    actions: &mut Vec<UiAction>,
) {
    let mut is_open = true;

    // S3.1: Selected Entity HUD
    egui::Window::new(format!("Entity: {}", data.name))
        .anchor(egui::Align2::RIGHT_TOP, [-10.0, 10.0])
        .default_width(320.0)
        .open(&mut is_open)
        .show(ctx, |ui| {
            egui::Grid::new("hud_grid")
                .striped(true)
                .show(ui, |ui| {
                    if let Some(bt) = &data.body_type {
                        ui.label("Type:");
                        ui.label(format!("{bt:?}"));
                        ui.end_row();
                    }
                    if let Some(m) = data.mass {
                        ui.label("Mass:");
                        ui.label(format!("{m:.6e} kg"));
                        ui.end_row();
                    }
                    if let Some(v) = data.velocity {
                        ui.label("Velocity:");
                        ui.label(format!("({:.6e}, {:.6e}, {:.6e}) m/s", v.x, v.y, v.z));
                        ui.end_row();
                    }
                    if let Some(p) = data.position {
                        ui.label("World Pos:");
                        ui.label(format!("({:.6e}, {:.6e}, {:.6e}) m", p.x, p.y, p.z));
                        ui.end_row();
                    }
                    if let Some(t) = data.temperature {
                        ui.label("Temperature:");
                        ui.label(format!("{t:.2} K"));
                        ui.end_row();
                    }
                });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                if ui.button("Enfocar Cámara").clicked() {
                    actions.push(UiAction::FocusCamera(entity));
                }
            });
        });

    // If the user closed the window via the 'X' button
    if !is_open {
        actions.push(UiAction::ClearSelection);
    }
}
