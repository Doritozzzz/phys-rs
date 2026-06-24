use egui::{Context, Window, ScrollArea, TextEdit};
use crate::render::ui::actions::UiAction;
use crate::render::ui::UiState;

pub fn draw(ctx: &Context, state: &mut UiState, actions: &mut Vec<UiAction>) {
    if !state.show_console {
        return;
    }

    Window::new("Console")
        .open(&mut state.show_console)
        .show(ctx, |ui| {
            ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                for line in &state.console_history {
                    ui.label(line);
                }
            });

            ui.horizontal(|ui| {
                ui.label(">");
                let response = ui.add(TextEdit::singleline(&mut state.console_input).desired_width(f32::INFINITY));
                if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    let cmd = state.console_input.trim().to_string();
                    if !cmd.is_empty() {
                        state.console_history.push(format!("> {}", cmd));
                        actions.push(UiAction::ExecuteCommand(cmd));
                        state.console_input.clear();
                    }
                    response.request_focus();
                }
            });
        });
}
