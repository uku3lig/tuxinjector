use tuxinjector_config::Config;

pub fn render(ui: &imgui::Ui, config: &mut Config, dirty: &mut bool) {
    if ui.checkbox("Enable Custom Cursors", &mut config.theme.cursors.enabled) {
        *dirty = true;
    }

    if !config.theme.cursors.enabled {
        ui.dummy([0.0, 4.0]);
        ui.text("Enable custom cursors to configure per-state cursor themes.");
        return;
    }

    ui.dummy([0.0, 8.0]);
    ui.text("Set a cursor name from your system's cursor theme. Leave blank for the default cursor.");

    ui.dummy([0.0, 8.0]);
    cursor_input(ui, "Title Screen:", "##cursor_title",
        &mut config.theme.cursors.title.cursor_name,
        &mut config.theme.cursors.title.cursor_size, dirty);

    ui.dummy([0.0, 4.0]);
    cursor_input(ui, "Wall / Menu:", "##cursor_wall",
        &mut config.theme.cursors.wall.cursor_name,
        &mut config.theme.cursors.wall.cursor_size, dirty);

    ui.dummy([0.0, 4.0]);
    cursor_input(ui, "In-Game:", "##cursor_ingame",
        &mut config.theme.cursors.ingame.cursor_name,
        &mut config.theme.cursors.ingame.cursor_size, dirty);
}

fn cursor_input(
    ui: &imgui::Ui,
    label: &str,
    id: &str,
    name: &mut String,
    size: &mut i32,
    dirty: &mut bool,
) {
    ui.text(label);
    ui.text("Name:");
    ui.same_line();
    ui.set_next_item_width(180.0);
    if ui.input_text(format!("{id}_name"), name).hint("cursor name").build() {
        *dirty = true;
    }
    ui.same_line();
    ui.text("Size:");
    ui.same_line();
    ui.set_next_item_width(80.0);
    if crate::widgets::slider_int(ui, &format!("{id}_size"), size, 8, 256, "%d px") {
        *dirty = true;
    }
}
