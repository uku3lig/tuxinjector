use tuxinjector_config::Config;

pub fn render(
    ui: &imgui::Ui,
    config: &mut Config,
    dirty: &mut bool,
    font_cache: &mut Option<Vec<(String, String)>>,
) {
    let ez = &mut config.overlays.eyezoom;

    // capture region
    ui.separator();
    ui.text("Clone / Capture");

    ui.text("Clone Width:");
    ui.same_line();
    ui.set_next_item_width(100.0);
    if crate::widgets::slider_int(ui, "##clone_width", &mut ez.clone_width, 1, 7680, "%d px") {
        *dirty = true;
    }
    ui.same_line();
    ui.text("Clone Height:");
    ui.same_line();
    ui.set_next_item_width(100.0);
    if crate::widgets::slider_int(ui, "##clone_height", &mut ez.clone_height, 1, 16384, "%d px") {
        *dirty = true;
    }

    // overlay output sizing
    ui.dummy([0.0, 8.0]);
    ui.separator();
    ui.text("Overlay / Output");

    ui.text("Overlay Width:");
    ui.same_line();
    ui.set_next_item_width(100.0);
    if crate::widgets::slider_int(ui, "##overlay_width", &mut ez.overlay_width, 0, 30, "%d px") {
        *dirty = true;
    }

    ui.text("Stretch Width:");
    ui.same_line();
    ui.set_next_item_width(100.0);
    if crate::widgets::slider_int(ui, "##stretch_width", &mut ez.stretch_width, 0, 7680, "%d px") {
        *dirty = true;
    }

    ui.text("Horizontal Margin:");
    ui.same_line();
    ui.set_next_item_width(100.0);
    if crate::widgets::slider_int(ui, "##h_margin", &mut ez.horizontal_margin, -10000, 10000, "%d px") {
        *dirty = true;
    }
    ui.same_line();
    ui.text("Vertical Margin:");
    ui.same_line();
    ui.set_next_item_width(100.0);
    if crate::widgets::slider_int(ui, "##v_margin", &mut ez.vertical_margin, -10000, 10000, "%d px") {
        *dirty = true;
    }

    // text / font settings
    ui.dummy([0.0, 8.0]);
    ui.separator();
    ui.text("Text / Font");

    if ui.checkbox("Auto Font Size", &mut ez.auto_font_size) {
        *dirty = true;
    }
    if !ez.auto_font_size {
        ui.text("Font Size:");
        ui.same_line();
        ui.set_next_item_width(100.0);
        if crate::widgets::slider_int(ui, "##text_font_size", &mut ez.text_font_size, 4, 200, "%d px") {
            *dirty = true;
        }
    }
    ui.text("Font:");
    ui.same_line();
    if crate::widgets::font_combo(ui, "##ez_font", &mut ez.text_font_path, font_cache) {
        *dirty = true;
    }

    ui.text("Rect Height:");
    ui.same_line();
    ui.set_next_item_width(100.0);
    if crate::widgets::slider_int(ui, "##rect_height", &mut ez.rect_height, 1, 200, "%d px") {
        *dirty = true;
    }
    if ui.checkbox("Link Rect To Font", &mut ez.link_rect_to_font) {
        *dirty = true;
    }

    // color pickers
    ui.dummy([0.0, 8.0]);
    ui.separator();
    ui.text("Colors");

    color_picker(ui, "Grid Color 1:", "##grid1", &mut ez.grid_color1, &mut ez.grid_color1_opacity, dirty);
    color_picker(ui, "Grid Color 2:", "##grid2", &mut ez.grid_color2, &mut ez.grid_color2_opacity, dirty);
    color_picker(ui, "Highlight:", "##highlight", &mut ez.highlight_color, &mut ez.highlight_color_opacity, dirty);

    ui.text("Highlight Every:");
    ui.same_line();
    ui.set_next_item_width(100.0);
    if crate::widgets::slider_int(ui, "##highlight_interval", &mut ez.highlight_interval, 0, 50, "%d cells") {
        *dirty = true;
    }
    ui.same_line();
    ui.text_disabled("(0 = off)");

    color_picker(ui, "Center Line:", "##center_line", &mut ez.center_line_color, &mut ez.center_line_color_opacity, dirty);
    color_picker(ui, "Text Color:", "##text_color", &mut ez.text_color, &mut ez.text_color_opacity, dirty);

    // number style dropdown
    ui.dummy([0.0, 8.0]);
    ui.separator();
    ui.text("Number Style");

    let styles = ["stacked", "compact", "slackow", "horizontal"];
    let labels = ["Stacked Digits", "Compact (Small First Digit)", "Slackow (Only Stack 10s)", "Horizontal"];
    let cur_i = styles.iter().position(|s| *s == ez.number_style).unwrap_or(0);
    ui.set_next_item_width(280.0);
    if let Some(_token) = ui.begin_combo("##number_style", labels[cur_i]) {
        for (i, &lbl) in labels.iter().enumerate() {
            if ui.selectable_config(lbl).selected(i == cur_i).build() {
                ez.number_style = styles[i].to_string();
                *dirty = true;
            }
        }
    }

    // transition toggles
    ui.dummy([0.0, 8.0]);
    ui.separator();
    ui.text("Transitions");

    if ui.checkbox("Slide Zoom In", &mut ez.slide_zoom_in) {
        *dirty = true;
    }
    if ui.checkbox("Slide Mirrors In", &mut ez.slide_mirrors_in) {
        *dirty = true;
    }
}

fn color_picker(
    ui: &imgui::Ui,
    label: &str,
    id: &str,
    color: &mut tuxinjector_core::Color,
    opacity: &mut f32,
    dirty: &mut bool,
) {
    ui.text(label);
    ui.same_line();
    let mut rgba = [color.r, color.g, color.b, color.a];
    if ui.color_edit4(format!("{id}_color"), &mut rgba) {
        color.r = rgba[0];
        color.g = rgba[1];
        color.b = rgba[2];
        color.a = rgba[3];
        *dirty = true;
    }
    ui.same_line();
    ui.text("Opacity:");
    ui.same_line();
    ui.set_next_item_width(150.0);
    if crate::widgets::slider_float(ui, &format!("{id}_opacity"), opacity, 0.0, 1.0, "%.2f") {
        *dirty = true;
    }
}
