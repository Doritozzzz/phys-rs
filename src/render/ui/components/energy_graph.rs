use egui::{Context, Window};
use egui_plot::{Line, Plot, PlotPoints};
use crate::physics::energy::EnergyMonitor;
use crate::render::ui::UiState;

pub fn draw(ctx: &Context, state: &mut UiState, energy_data: &EnergyMonitor) {
    if !state.show_energy_graph {
        return;
    }

    // Update history
    state.current_time += 1.0; // Assume 1 tick per frame roughly for the graph x-axis
    state.energy_history.push_back((
        state.current_time,
        energy_data.kinetic_energy,
        energy_data.potential_energy,
        energy_data.total_energy,
    ));

    // Keep history bounded
    if state.energy_history.len() > 1000 {
        state.energy_history.pop_front();
    }

    Window::new("Energy Monitor")
        .open(&mut state.show_energy_graph)
        .default_pos([800.0, 20.0])
        .show(ctx, |ui| {
            let kinetic: PlotPoints = state.energy_history.iter()
                .map(|&(t, k, _, _)| [t, k])
                .collect();
            let potential: PlotPoints = state.energy_history.iter()
                .map(|&(t, _, p, _)| [t, p])
                .collect();
            let total: PlotPoints = state.energy_history.iter()
                .map(|&(t, _, _, tot)| [t, tot])
                .collect();

            let plot = Plot::new("energy_plot")
                .legend(egui_plot::Legend::default())
                .height(200.0);

            plot.show(ui, |plot_ui| {
                plot_ui.line(Line::new("Kinetic", kinetic).color(egui::Color32::from_rgb(0, 255, 0)));
                plot_ui.line(Line::new("Potential", potential).color(egui::Color32::from_rgb(255, 0, 0)));
                plot_ui.line(Line::new("Total", total).color(egui::Color32::from_rgb(0, 200, 255)));
            });
        });
}
