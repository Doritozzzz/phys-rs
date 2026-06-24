use egui::{Context, Window};
use crate::render::ui::actions::UiAction;
use crate::render::ui::UiState;

pub fn draw(ctx: &Context, state: &mut UiState, actions: &mut Vec<UiAction>) {
    Window::new("World State")
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Quick Save").clicked() {
                    actions.push(UiAction::QuickSave);
                    state.console_history.push("Simulation saved to saves/quicksave.bin".to_string());
                }
                if ui.button("Quick Load").clicked() {
                    actions.push(UiAction::QuickLoad);
                    state.console_history.push("Simulation loaded from saves/quicksave.bin".to_string());
                }
            });
        });
}
