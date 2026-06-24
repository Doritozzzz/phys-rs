use egui::{Context, Window, Slider};
use crate::render::ui::actions::UiAction;
use crate::render::ui::UiState;

pub fn draw(ctx: &Context, _state: &mut UiState, actions: &mut Vec<UiAction>) {
    Window::new("Time Control")
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("⏯ Play/Pause").clicked() {
                    actions.push(UiAction::TogglePause);
                }
                if ui.button("⏭ Step").clicked() {
                    actions.push(UiAction::StepTick);
                }
            });
            
            let mut speed = 1.0; // We'd ideally bind this to actual UniverseConfig speed
            if ui.add(Slider::new(&mut speed, 0.1..=1000.0).logarithmic(true).text("Time Scale")).changed() {
                actions.push(UiAction::SetTimeScale(speed));
            }
        });
}
