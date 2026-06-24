use egui::{Context, Window, Slider};
use crate::render::ui::actions::UiAction;
use crate::render::ui::UiState;

pub fn draw(ctx: &Context, state: &mut UiState, actions: &mut Vec<UiAction>) {
    if !state.show_time_control {
        return;
    }

    Window::new("Time Control")
        .open(&mut state.show_time_control)
        .default_pos([20.0, 20.0])
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("⏯ Play/Pause").clicked() {
                    actions.push(UiAction::TogglePause);
                }
                if ui.button("⏭ Step").clicked() {
                    actions.push(UiAction::StepTick);
                }
            });
            
            let mut speed = state.time_scale;
            if ui.add(Slider::new(&mut speed, 0.1..=1000.0).logarithmic(true).text("Time Scale")).changed() {
                state.time_scale = speed;
                actions.push(UiAction::SetTimeScale(speed));
            }
        });
}
