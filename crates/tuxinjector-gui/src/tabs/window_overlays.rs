use tuxinjector_config::types::{ColorKeyConfig, WindowOverlayConfig};
use tuxinjector_config::Config;

pub fn render(
    ui: &imgui::Ui,
    config: &mut Config,
    dirty: &mut bool,
    selected: &mut Option<usize>,
) {
    if let Some(idx) = *selected {
        if idx >= config.overlays.window_overlays.len() {
            *selected = None;
        }
    }

    ui.columns(2, "wo_cols", true);

    ui.text("Window Overlay List");
    ui.separator();

    for (i, wo) in config.overlays.window_overlays.iter().enumerate() {
        let lbl = if wo.name.is_empty() {
            format!("Window Overlay {}", i)
        } else {
            wo.name.clone()
        };
        if ui.selectable_config(&lbl)
            .selected(*selected == Some(i))
            .build()
        {
            *selected = Some(i);
        }
    }

    ui.dummy([0.0, 8.0]);
    if ui.button("Add Window Overlay") {
        config.overlays.window_overlays.push(WindowOverlayConfig::default());
        *selected = Some(config.overlays.window_overlays.len() - 1);
        *dirty = true;
    }

    ui.next_column();

    if let Some(idx) = *selected {
        if idx < config.overlays.window_overlays.len() {
            wo_editor(ui, &mut config.overlays.window_overlays[idx], idx, dirty);
            ui.dummy([0.0, 12.0]);
            if ui.button("Remove Window Overlay") {
                config.overlays.window_overlays.remove(idx);
                *selected = None;
                *dirty = true;
            }
        }
    } else {
        ui.text("Select a window overlay to edit.");
    }

    ui.columns(1, "wo_cols_end", false);
}

fn wo_editor(
    ui: &imgui::Ui,
    wo: &mut WindowOverlayConfig,
    idx: usize,
    dirty: &mut bool,
) {
    ui.text("Name:");
    ui.same_line();
    ui.set_next_item_width(200.0);
    if ui.input_text(format!("##wo_name_{}", idx), &mut wo.name).build() {
        *dirty = true;
    }

    // window matching rules
    ui.dummy([0.0, 8.0]);
    ui.separator(); ui.text("Window Matching");

    ui.text("Title:");
    ui.same_line();
    ui.set_next_item_width(200.0);
    if ui.input_text(format!("##wo_title_{}", idx), &mut wo.window_title).build() {
        *dirty = true;
    }

    ui.text("Class:");
    ui.same_line();
    ui.set_next_item_width(200.0);
    if ui.input_text(format!("##wo_class_{}", idx), &mut wo.window_class).build() {
        *dirty = true;
    }

    ui.text("Executable:");
    ui.same_line();
    ui.set_next_item_width(200.0);
    if ui.input_text(format!("##wo_exec_{}", idx), &mut wo.executable_name).build() {
        *dirty = true;
    }

    ui.text("Match Priority:");
    ui.same_line();
    if let Some(_token) =
        ui.begin_combo(format!("##wo_priority_{}", idx), &wo.window_match_priority)
    {
        for opt in &["title", "class", "executable"] {
            if ui.selectable_config(*opt)
                .selected(wo.window_match_priority == *opt)
                .build()
            {
                wo.window_match_priority = (*opt).to_string();
                *dirty = true;
            }
        }
    }

    // position + scale
    ui.dummy([0.0, 8.0]);
    ui.separator(); ui.text("Position");

    ui.text("X:");
    ui.same_line();
    ui.set_next_item_width(80.0);
    if crate::widgets::slider_int(ui, &format!("##wo_x_{}", idx), &mut wo.x, -10000, 10000, "%d") {
        *dirty = true;
    }
    ui.same_line();
    ui.text("Y:");
    ui.same_line();
    ui.set_next_item_width(80.0);
    if crate::widgets::slider_int(ui, &format!("##wo_y_{}", idx), &mut wo.y, -10000, 10000, "%d") {
        *dirty = true;
    }

    ui.text("Scale:");
    ui.same_line();
    ui.set_next_item_width(200.0);
    if crate::widgets::slider_float(ui, &format!("##wo_scale_{}", idx), &mut wo.scale, 0.1, 10.0, "%.2f") {
        *dirty = true;
    }

    ui.text("Relative To:");
    ui.same_line();
    if let Some(_token) = ui.begin_combo(format!("##wo_rel_{}", idx), &wo.relative_to) {
        for anchor in &["topLeftScreen", "topRightScreen", "bottomLeftScreen", "bottomRightScreen", "center"] {
            if ui.selectable_config(*anchor)
                .selected(wo.relative_to == *anchor)
                .build()
            {
                wo.relative_to = (*anchor).to_string();
                *dirty = true;
            }
        }
    }

    // crop
    ui.dummy([0.0, 8.0]);
    ui.separator(); ui.text("Crop");

    ui.text("Top:");
    ui.same_line();
    ui.set_next_item_width(80.0);
    if crate::widgets::slider_int(ui, &format!("##wo_crop_top_{}", idx), &mut wo.crop_top, 0, 4320, "%d px") {
        *dirty = true;
    }
    ui.same_line();
    ui.text("Bottom:");
    ui.same_line();
    ui.set_next_item_width(80.0);
    if crate::widgets::slider_int(ui, &format!("##wo_crop_bottom_{}", idx), &mut wo.crop_bottom, 0, 4320, "%d px") {
        *dirty = true;
    }

    ui.text("Left:");
    ui.same_line();
    ui.set_next_item_width(80.0);
    if crate::widgets::slider_int(ui, &format!("##wo_crop_left_{}", idx), &mut wo.crop_left, 0, 7680, "%d px") {
        *dirty = true;
    }
    ui.same_line();
    ui.text("Right:");
    ui.same_line();
    ui.set_next_item_width(80.0);
    if crate::widgets::slider_int(ui, &format!("##wo_crop_right_{}", idx), &mut wo.crop_right, 0, 7680, "%d px") {
        *dirty = true;
    }

    // color key
    ui.dummy([0.0, 8.0]);
    ui.separator(); ui.text("Color Key");

    if ui.checkbox(
        format!("Enable Color Key##wo_ck_{}", idx),
        &mut wo.enable_color_key,
    ) {
        *dirty = true;
    }

    if wo.enable_color_key {
        let mut rgba = [wo.color_key.r, wo.color_key.g, wo.color_key.b, wo.color_key.a];
        ui.text("Key Color:");
        ui.same_line();
        if ui.color_edit4(format!("##wo_ck_color_{}", idx), &mut rgba) {
            wo.color_key.r = rgba[0];
            wo.color_key.g = rgba[1];
            wo.color_key.b = rgba[2];
            wo.color_key.a = rgba[3];
            *dirty = true;
        }

        ui.text("Sensitivity:");
        ui.same_line();
        ui.set_next_item_width(200.0);
        if crate::widgets::slider_float(ui, &format!("##wo_ck_sens_{}", idx), &mut wo.color_key_sensitivity, 0.0, 1.0, "%.3f") {
            *dirty = true;
        }

        ui.dummy([0.0, 4.0]);
        ui.text("Additional Color Keys:");
        let mut rm = None;
        for (i, ck) in wo.color_keys.iter_mut().enumerate() {
            let mut rgba = [ck.color.r, ck.color.g, ck.color.b, ck.color.a];
            if ui.color_edit4(format!("##wo_ack_color_{}_{}", idx, i), &mut rgba) {
                ck.color.r = rgba[0];
                ck.color.g = rgba[1];
                ck.color.b = rgba[2];
                ck.color.a = rgba[3];
                *dirty = true;
            }
            ui.same_line();
            ui.text("sens:");
            ui.same_line();
            ui.set_next_item_width(80.0);
            if crate::widgets::slider_float(ui, &format!("##wo_ack_sens_{}_{}", idx, i), &mut ck.sensitivity, 0.0, 1.0, "%.3f") {
                *dirty = true;
            }
            ui.same_line();
            if ui.small_button(format!("X##wo_ack_rm_{}_{}", idx, i)) {
                rm = Some(i);
                *dirty = true;
            }
        }
        if let Some(ri) = rm {
            wo.color_keys.remove(ri);
        }
        if ui.small_button(format!("Add Color Key##wo_ack_add_{}", idx)) {
            wo.color_keys.push(ColorKeyConfig::default());
            *dirty = true;
        }
    }

    // misc settings
    ui.dummy([0.0, 8.0]);
    ui.separator(); ui.text("Misc");

    ui.text("Opacity:");
    ui.same_line();
    ui.set_next_item_width(200.0);
    if crate::widgets::slider_float(ui, &format!("##wo_opacity_{}", idx), &mut wo.opacity, 0.0, 1.0, "%.2f") {
        *dirty = true;
    }

    ui.text("FPS:");
    ui.same_line();
    ui.set_next_item_width(80.0);
    if crate::widgets::slider_int(ui, &format!("##wo_fps_{}", idx), &mut wo.fps, 1, 240, "%d fps") {
        *dirty = true;
    }

    ui.text("Search Interval (ms):");
    ui.same_line();
    ui.set_next_item_width(80.0);
    if crate::widgets::slider_int(ui, &format!("##wo_search_{}", idx), &mut wo.search_interval, 100, 30000, "%d ms") {
        *dirty = true;
    }

    ui.text("Capture Method:");
    ui.same_line();
    ui.set_next_item_width(160.0);
    if ui.input_text(format!("##wo_capture_{}", idx), &mut wo.capture_method).build() {
        *dirty = true;
    }

    if ui.checkbox(format!("Enable Interaction##wo_interact_{}", idx), &mut wo.enable_interaction) {
        *dirty = true;
    }
    if ui.checkbox(format!("Pixelated Scaling##wo_pix_{}", idx), &mut wo.pixelated_scaling) {
        *dirty = true;
    }

    // background
    ui.dummy([0.0, 8.0]);
    ui.separator(); ui.text("Background");

    if ui.checkbox(format!("Enable Background##wo_bg_{}", idx), &mut wo.background.enabled) {
        *dirty = true;
    }

    if wo.background.enabled {
        let mut rgba = [
            wo.background.color.r, wo.background.color.g,
            wo.background.color.b, wo.background.color.a,
        ];
        ui.text("Color:");
        ui.same_line();
        if ui.color_edit4(format!("##wo_bg_color_{}", idx), &mut rgba) {
            wo.background.color.r = rgba[0];
            wo.background.color.g = rgba[1];
            wo.background.color.b = rgba[2];
            wo.background.color.a = rgba[3];
            *dirty = true;
        }

        ui.text("Opacity:");
        ui.same_line();
        ui.set_next_item_width(200.0);
        if crate::widgets::slider_float(ui, &format!("##wo_bg_opacity_{}", idx), &mut wo.background.opacity, 0.0, 1.0, "%.2f") {
            *dirty = true;
        }
    }

    // border
    ui.dummy([0.0, 8.0]);
    ui.separator(); ui.text("Border");

    if ui.checkbox(format!("Enable Border##wo_brd_{}", idx), &mut wo.border.enabled) {
        *dirty = true;
    }

    if wo.border.enabled {
        let mut rgba = [
            wo.border.color.r, wo.border.color.g,
            wo.border.color.b, wo.border.color.a,
        ];
        ui.text("Color:");
        ui.same_line();
        if ui.color_edit4(format!("##wo_brd_color_{}", idx), &mut rgba) {
            wo.border.color.r = rgba[0];
            wo.border.color.g = rgba[1];
            wo.border.color.b = rgba[2];
            wo.border.color.a = rgba[3];
            *dirty = true;
        }

        ui.text("Width:");
        ui.same_line();
        ui.set_next_item_width(80.0);
        if crate::widgets::slider_int(ui, &format!("##wo_brd_width_{}", idx), &mut wo.border.width, 0, 100, "%d px") {
            *dirty = true;
        }

        ui.text("Radius:");
        ui.same_line();
        ui.set_next_item_width(80.0);
        if crate::widgets::slider_int(ui, &format!("##wo_brd_radius_{}", idx), &mut wo.border.radius, 0, 200, "%d px") {
            *dirty = true;
        }
    }
}
