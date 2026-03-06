use tuxinjector_config::types::ImageConfig;
use tuxinjector_config::Config;

pub fn render(
    ui: &imgui::Ui,
    config: &mut Config,
    dirty: &mut bool,
    selected: &mut Option<usize>,
) {
    ui.separator(); ui.text("Images");
    ui.dummy([0.0, 4.0]);

    if let Some(idx) = *selected {
        if idx >= config.overlays.images.len() {
            *selected = None;
        }
    }

    ui.columns(2, "img_cols", true);

    ui.text("Image List");
    ui.separator();

    for (i, img) in config.overlays.images.iter().enumerate() {
        let lbl = if img.name.is_empty() {
            format!("Image {}", i)
        } else {
            img.name.clone()
        };
        if ui.selectable_config(&lbl)
            .selected(*selected == Some(i))
            .build()
        {
            *selected = Some(i);
        }
    }

    ui.dummy([0.0, 8.0]);
    if ui.button("Add Image") {
        config.overlays.images.push(ImageConfig::default());
        *selected = Some(config.overlays.images.len() - 1);
        *dirty = true;
    }

    ui.next_column();

    if let Some(idx) = *selected {
        if idx < config.overlays.images.len() {
            image_editor(ui, &mut config.overlays.images[idx], idx, dirty);

            ui.dummy([0.0, 12.0]);
            if ui.button("Remove Image") {
                config.overlays.images.remove(idx);
                *selected = None;
                *dirty = true;
            }
        }
    } else {
        ui.text("Select an image to edit.");
    }

    ui.columns(1, "img_cols_end", false);
}

fn image_editor(
    ui: &imgui::Ui,
    img: &mut ImageConfig,
    idx: usize,
    dirty: &mut bool,
) {
    ui.text("Name:");
    ui.same_line();
    ui.set_next_item_width(200.0);
    if ui.input_text(format!("##img_name_{}", idx), &mut img.name).build() {
        *dirty = true;
    }

    ui.text("Path:");
    ui.same_line();
    ui.set_next_item_width(250.0);
    if ui.input_text(format!("##img_path_{}", idx), &mut img.path).build() {
        *dirty = true;
    }

    // position
    ui.dummy([0.0, 8.0]);
    ui.separator(); ui.text("Position");

    ui.text("X:");
    ui.same_line();
    ui.set_next_item_width(80.0);
    if crate::widgets::slider_int(ui, &format!("##img_x_{}", idx), &mut img.x, -10000, 10000, "%d") {
        *dirty = true;
    }
    ui.same_line();
    ui.text("Y:");
    ui.same_line();
    ui.set_next_item_width(80.0);
    if crate::widgets::slider_int(ui, &format!("##img_y_{}", idx), &mut img.y, -10000, 10000, "%d") {
        *dirty = true;
    }

    ui.text("Scale:");
    ui.same_line();
    ui.set_next_item_width(200.0);
    if crate::widgets::slider_float(ui, &format!("##img_scale_{}", idx), &mut img.scale, 0.1, 10.0, "%.2f") {
        *dirty = true;
    }

    ui.text("Relative To:");
    ui.same_line();
    if let Some(_token) = ui.begin_combo(format!("##img_rel_{}", idx), &img.relative_to) {
        for anchor in &["topLeftScreen", "topRightScreen", "bottomLeftScreen", "bottomRightScreen", "center"] {
            if ui.selectable_config(*anchor)
                .selected(img.relative_to == *anchor)
                .build()
            {
                img.relative_to = (*anchor).to_string();
                *dirty = true;
            }
        }
    }

    ui.text("Opacity:");
    ui.same_line();
    ui.set_next_item_width(200.0);
    if crate::widgets::slider_float(ui, &format!("##img_opacity_{}", idx), &mut img.opacity, 0.0, 1.0, "%.2f") {
        *dirty = true;
    }

    // crop
    ui.dummy([0.0, 8.0]);
    ui.separator(); ui.text("Crop");

    ui.text("Top:");
    ui.same_line();
    ui.set_next_item_width(80.0);
    if crate::widgets::slider_int(ui, &format!("##img_crop_top_{}", idx), &mut img.crop_top, 0, 4320, "%d px") {
        *dirty = true;
    }
    ui.same_line();
    ui.text("Bottom:");
    ui.same_line();
    ui.set_next_item_width(80.0);
    if crate::widgets::slider_int(ui, &format!("##img_crop_bottom_{}", idx), &mut img.crop_bottom, 0, 4320, "%d px") {
        *dirty = true;
    }

    ui.text("Left:");
    ui.same_line();
    ui.set_next_item_width(80.0);
    if crate::widgets::slider_int(ui, &format!("##img_crop_left_{}", idx), &mut img.crop_left, 0, 7680, "%d px") {
        *dirty = true;
    }
    ui.same_line();
    ui.text("Right:");
    ui.same_line();
    ui.set_next_item_width(80.0);
    if crate::widgets::slider_int(ui, &format!("##img_crop_right_{}", idx), &mut img.crop_right, 0, 7680, "%d px") {
        *dirty = true;
    }

    // color keying
    ui.dummy([0.0, 8.0]);
    ui.separator(); ui.text("Color Key");

    if ui.checkbox("Enable Color Key", &mut img.enable_color_key) {
        *dirty = true;
    }

    if img.enable_color_key {
        let mut rgba = [
            img.color_key.r, img.color_key.g,
            img.color_key.b, img.color_key.a,
        ];
        ui.text("Key Color:");
        ui.same_line();
        if ui.color_edit4(format!("##img_ck_color_{}", idx), &mut rgba) {
            img.color_key.r = rgba[0];
            img.color_key.g = rgba[1];
            img.color_key.b = rgba[2];
            img.color_key.a = rgba[3];
            *dirty = true;
        }

        ui.text("Sensitivity:");
        ui.same_line();
        ui.set_next_item_width(200.0);
        if crate::widgets::slider_float(ui, &format!("##img_ck_sens_{}", idx), &mut img.color_key_sensitivity, 0.0, 1.0, "%.3f") {
            *dirty = true;
        }

        // additional color keys
        ui.dummy([0.0, 4.0]);
        ui.text("Additional Color Keys:");
        let mut rm_ck = None;
        for (i, ck) in img.color_keys.iter_mut().enumerate() {
            let mut rgba = [ck.color.r, ck.color.g, ck.color.b, ck.color.a];
            if ui.color_edit4(format!("##img_ack_color_{}_{}", idx, i), &mut rgba) {
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
            if crate::widgets::slider_float(ui, &format!("##img_ack_sens_{}_{}", idx, i), &mut ck.sensitivity, 0.0, 1.0, "%.3f") {
                *dirty = true;
            }
            ui.same_line();
            if ui.small_button(format!("X##img_ack_rm_{}_{}", idx, i)) {
                rm_ck = Some(i);
                *dirty = true;
            }
        }
        if let Some(ri) = rm_ck {
            img.color_keys.remove(ri);
        }
        if ui.small_button(format!("Add Color Key##img_ack_add_{}", idx)) {
            img.color_keys
                .push(tuxinjector_config::types::ColorKeyConfig::default());
            *dirty = true;
        }
    }

    // background
    ui.dummy([0.0, 8.0]);
    ui.separator(); ui.text("Background");

    if ui.checkbox(
        format!("Enable Background##img_bg_{}", idx),
        &mut img.background.enabled,
    ) {
        *dirty = true;
    }

    if img.background.enabled {
        let mut rgba = [
            img.background.color.r, img.background.color.g,
            img.background.color.b, img.background.color.a,
        ];
        ui.text("Color:");
        ui.same_line();
        if ui.color_edit4(format!("##img_bg_color_{}", idx), &mut rgba) {
            img.background.color.r = rgba[0];
            img.background.color.g = rgba[1];
            img.background.color.b = rgba[2];
            img.background.color.a = rgba[3];
            *dirty = true;
        }

        ui.text("Opacity:");
        ui.same_line();
        ui.set_next_item_width(200.0);
        if crate::widgets::slider_float(ui, &format!("##img_bg_opacity_{}", idx), &mut img.background.opacity, 0.0, 1.0, "%.2f") {
            *dirty = true;
        }
    }

    // border
    ui.dummy([0.0, 8.0]);
    ui.separator(); ui.text("Border");
    image_border(ui, &mut img.border, idx, dirty);

    ui.dummy([0.0, 8.0]);
    if ui.checkbox(
        format!("Pixelated Scaling##img_pix_{}", idx),
        &mut img.pixelated_scaling,
    ) {
        *dirty = true;
    }
}

fn image_border(
    ui: &imgui::Ui,
    border: &mut tuxinjector_config::types::BorderConfig,
    idx: usize,
    dirty: &mut bool,
) {
    if ui.checkbox(format!("Enable Border##img_brd_{}", idx), &mut border.enabled) {
        *dirty = true;
    }

    if border.enabled {
        let mut rgba = [border.color.r, border.color.g, border.color.b, border.color.a];
        ui.text("Color:");
        ui.same_line();
        if ui.color_edit4(format!("##img_brd_color_{}", idx), &mut rgba) {
            border.color.r = rgba[0];
            border.color.g = rgba[1];
            border.color.b = rgba[2];
            border.color.a = rgba[3];
            *dirty = true;
        }

        ui.text("Width:");
        ui.same_line();
        ui.set_next_item_width(80.0);
        if crate::widgets::slider_int(ui, &format!("##img_brd_width_{}", idx), &mut border.width, 0, 100, "%d px") {
            *dirty = true;
        }

        ui.text("Radius:");
        ui.same_line();
        ui.set_next_item_width(80.0);
        if crate::widgets::slider_int(ui, &format!("##img_brd_radius_{}", idx), &mut border.radius, 0, 200, "%d px") {
            *dirty = true;
        }
    }
}
