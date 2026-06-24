use egui::{Context, Window, Slider, ComboBox};
use glam::DVec3;
use crate::components::BodyType;
use crate::render::ui::actions::UiAction;
use crate::render::ui::UiState;

pub fn draw(ctx: &Context, state: &mut UiState, actions: &mut Vec<UiAction>) {
    if !state.show_spawner {
        return;
    }

    Window::new("Entity Spawner")
        .open(&mut state.show_spawner)
        .show(ctx, |ui| {
            ui.add(Slider::new(&mut state.spawner_mass_log, -5.0..=35.0).text("Log10 Mass"));
            let mass = 10_f64.powf(state.spawner_mass_log as f64);
            ui.label(format!("Mass: {:.2e} kg", mass));

            ui.horizontal(|ui| {
                ui.label("Velocity:");
                ui.add(egui::DragValue::new(&mut state.spawner_vel[0]).speed(1.0).prefix("X: "));
                ui.add(egui::DragValue::new(&mut state.spawner_vel[1]).speed(1.0).prefix("Y: "));
                ui.add(egui::DragValue::new(&mut state.spawner_vel[2]).speed(1.0).prefix("Z: "));
            });

            ui.add(Slider::new(&mut state.spawner_temp, 0.0..=50000.0).text("Temperature"));

            ComboBox::from_label("Type")
                .selected_text(format!("{:?}", state.spawner_type))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut state.spawner_type, BodyType::Asteroid, "Asteroid");
                    ui.selectable_value(&mut state.spawner_type, BodyType::Moon, "Moon");
                    ui.selectable_value(&mut state.spawner_type, BodyType::Planet, "Planet");
                    ui.selectable_value(&mut state.spawner_type, BodyType::Star, "Star");
                    ui.selectable_value(&mut state.spawner_type, BodyType::BlackHole, "BlackHole");
                    ui.selectable_value(&mut state.spawner_type, BodyType::Particle, "Particle");
                });

            if ui.button("Spawn at Origin").clicked() {
                actions.push(UiAction::SpawnEntity {
                    mass,
                    position: DVec3::ZERO,
                    velocity: DVec3::new(
                        state.spawner_vel[0] as f64,
                        state.spawner_vel[1] as f64,
                        state.spawner_vel[2] as f64,
                    ),
                    temperature: state.spawner_temp as f64,
                    body_type: state.spawner_type,
                });
            }
        });
}
