use egui::{Context, Window};
use crate::render::ui::UiState;
use crate::core::diagnostics::Diagnostics;

pub fn draw(ctx: &Context, state: &mut UiState, diagnostics: Option<&Diagnostics>) {
    if !state.show_profiler {
        return;
    }

    Window::new("Profiler")
        .open(&mut state.show_profiler)
        .show(ctx, |ui| {
            ui.label("System Execution Times (ms):");
            if let Some(diag) = diagnostics {
                let mut sorted_timings: Vec<_> = diag.timings.iter().collect();
                sorted_timings.sort_by_key(|(k, _)| *k);
                for (name, duration) in sorted_timings {
                    let ms = duration.as_secs_f64() * 1000.0;
                    ui.label(format!("{}: {:.3} ms", name, ms));
                }
            } else {
                ui.label("No diagnostics data available");
            }
        });
}
