use bevy_ecs::prelude::Entity;
use crate::render::ui::actions::UiAction;

pub fn draw(
    ctx: &egui::Context,
    entity_list: &[(Entity, String)],
    selected_entity: Option<Entity>,
    actions: &mut Vec<UiAction>,
) {
    // S3.8: Entity Inspector Panel (left)
    #[allow(deprecated)]
    egui::SidePanel::left("entity_inspector")
        .default_width(200.0)
        .resizable(true)
        .show(ctx, |ui| {
            ui.heading("Entities");
            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);
            egui::ScrollArea::vertical().show(ui, |ui| {
                for (entity, name) in entity_list {
                    let is_selected = selected_entity.map_or(false, |s| s == *entity);
                    let label = if name.is_empty() {
                        format!("Entity #{}", entity.index())
                    } else {
                        format!("{} [{}]", name, entity.index())
                    };
                    if ui.selectable_label(is_selected, label).clicked() {
                        actions.push(UiAction::SelectEntity(*entity));
                    }
                }
            });
        });
}
