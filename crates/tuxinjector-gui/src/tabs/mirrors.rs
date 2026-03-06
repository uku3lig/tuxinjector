use tuxinjector_config::types::{MirrorBorderShape, MirrorBorderType, MirrorConfig};
use tuxinjector_config::Config;

pub fn render(
    ui: &imgui::Ui,
    config: &mut Config,
    dirty: &mut bool,
    selected: &mut Option<usize>,
) {
    ui.separator(); ui.text("Mirrors");
    ui.dummy([0.0, 4.0]);

    // bounds check in case mirrors got removed externally
    if let Some(idx) = *selected {
        if idx >= config.overlays.mirrors.len() {
            *selected = None;
        }
    }

    ui.columns(2, "mirror_cols", true);

    // left: mirror list
    ui.text("Mirror List");
    ui.separator();

    for (i, mirror) in config.overlays.mirrors.iter().enumerate() {
        let lbl = if mirror.name.is_empty() {
            format!("Mirror {}", i)
        } else {
            mirror.name.clone()
        };
        if ui.selectable_config(&lbl)
            .selected(*selected == Some(i))
            .build()
        {
            *selected = Some(i);
        }
    }

    ui.dummy([0.0, 8.0]);
    if ui.button("Add Mirror") {
        config.overlays.mirrors.push(MirrorConfig::default());
        *selected = Some(config.overlays.mirrors.len() - 1);
        *dirty = true;
    }

    // right: editor
    ui.next_column();

    if let Some(idx) = *selected {
        if idx < config.overlays.mirrors.len() {
            mirror_editor(ui, &mut config.overlays.mirrors[idx], idx, dirty);

            ui.dummy([0.0, 12.0]);
            if ui.button("Duplicate Mirror") {
                let mut copy = config.overlays.mirrors[idx].clone();
                copy.name = format!("{} (copy)", copy.name);
                config.overlays.mirrors.push(copy);
                *selected = Some(config.overlays.mirrors.len() - 1);
                *dirty = true;
            }
            ui.same_line();
            if ui.button("Remove Mirror") {
                config.overlays.mirrors.remove(idx);
                *selected = None;
                *dirty = true;
            }
        }
    } else {
        ui.text("Select a mirror to edit.");
    }

    ui.columns(1, "mirror_cols_end", false);
}

fn mirror_editor(
    ui: &imgui::Ui,
    mirror: &mut MirrorConfig,
    idx: usize,
    dirty: &mut bool,
) {
    ui.text("Name:");
    ui.same_line();
    ui.set_next_item_width(200.0);
    if ui
        .input_text(format!("##mirror_name_{}", idx), &mut mirror.name)
        .build()
    {
        *dirty = true;
    }

    // capture region
    ui.dummy([0.0, 8.0]);
    ui.separator(); ui.text("Capture");

    ui.text("Width:");
    ui.same_line();
    ui.set_next_item_width(80.0);
    if crate::widgets::slider_int(ui, &format!("##mirror_cw_{}", idx), &mut mirror.capture_width, 1, 7680, "%d px")
    {
        *dirty = true;
    }
    ui.same_line();
    ui.text("Height:");
    ui.same_line();
    ui.set_next_item_width(80.0);
    if crate::widgets::slider_int(ui, &format!("##mirror_ch_{}", idx), &mut mirror.capture_height, 1, 4320, "%d px")
    {
        *dirty = true;
    }

    // input positions (multi-input mirrors)
    ui.dummy([0.0, 4.0]);
    ui.text("Input Positions:");
    let mut rm_input = None;
    for (i, input) in mirror.input.iter_mut().enumerate() {
        ui.text(format!("  [{}]", i));
        ui.same_line();
        ui.text("X:");
        ui.same_line();
        ui.set_next_item_width(80.0);
        if crate::widgets::slider_int(ui, &format!("##mirror_ix_{}_{}", idx, i), &mut input.x, -10000, 10000, "%d")
        {
            *dirty = true;
        }
        ui.same_line();
        ui.text("Y:");
        ui.same_line();
        ui.set_next_item_width(80.0);
        if crate::widgets::slider_int(ui, &format!("##mirror_iy_{}_{}", idx, i), &mut input.y, -10000, 10000, "%d")
        {
            *dirty = true;
        }
        ui.same_line();
        if ui.small_button(format!("X##mirror_irm_{}_{}", idx, i)) {
            rm_input = Some(i);
            *dirty = true;
        }
    }
    if let Some(ri) = rm_input {
        mirror.input.remove(ri);
    }
    if ui.small_button(format!("Add Input##mirror_iadd_{}", idx)) {
        mirror
            .input
            .push(tuxinjector_config::types::MirrorCaptureConfig::default());
        *dirty = true;
    }

    ui.dummy([0.0, 8.0]);
    ui.separator(); ui.text("Output");
    mirror_output(ui, &mut mirror.output, idx, dirty);

    ui.dummy([0.0, 8.0]);
    ui.separator(); ui.text("Colors");
    mirror_colors(ui, &mut mirror.colors, idx, dirty);

    ui.text("Color Sensitivity:");
    ui.same_line();
    ui.set_next_item_width(200.0);
    if crate::widgets::slider_float(ui, &format!("##mirror_csens_{}", idx), &mut mirror.color_sensitivity, 0.0, 1.0, "%.3f")
    {
        *dirty = true;
    }

    ui.dummy([0.0, 8.0]);
    ui.separator(); ui.text("Border");
    mirror_border(ui, &mut mirror.border, idx, dirty);

    ui.dummy([0.0, 8.0]);
    ui.separator(); ui.text("Misc");

    ui.text("FPS:");
    ui.same_line();
    ui.set_next_item_width(80.0);
    if crate::widgets::slider_int(ui, &format!("##mirror_fps_{}", idx), &mut mirror.fps, 0, 240, "%d fps")
    {
        *dirty = true;
    }

    ui.text("Opacity:");
    ui.same_line();
    ui.set_next_item_width(200.0);
    if crate::widgets::slider_float(ui, &format!("##mirror_opacity_{}", idx), &mut mirror.opacity, 0.0, 1.0, "%.2f")
    {
        *dirty = true;
    }

    if ui.checkbox(
        format!("Raw Output##mirror_raw_{}", idx),
        &mut mirror.raw_output,
    ) {
        *dirty = true;
    }
    if ui.checkbox(
        format!("Color Passthrough##mirror_cp_{}", idx),
        &mut mirror.color_passthrough,
    ) {
        *dirty = true;
    }
}

fn mirror_output(
    ui: &imgui::Ui,
    out: &mut tuxinjector_config::types::MirrorRenderConfig,
    idx: usize,
    dirty: &mut bool,
) {
    if ui.checkbox(
        format!("Use Relative Position##mirror_relpos_{}", idx),
        &mut out.use_relative_position,
    ) {
        *dirty = true;
    }

    if out.use_relative_position {
        ui.text("Relative X:");
        ui.same_line();
        ui.set_next_item_width(200.0);
        if crate::widgets::slider_float(ui, &format!("##mirror_rx_{}", idx), &mut out.relative_x, 0.0, 1.0, "%.2f")
        {
            *dirty = true;
        }
        ui.text("Relative Y:");
        ui.same_line();
        ui.set_next_item_width(200.0);
        if crate::widgets::slider_float(ui, &format!("##mirror_ry_{}", idx), &mut out.relative_y, 0.0, 1.0, "%.2f")
        {
            *dirty = true;
        }
    } else {
        ui.text("X:");
        ui.same_line();
        ui.set_next_item_width(80.0);
        if crate::widgets::slider_int(ui, &format!("##mirror_ox_{}", idx), &mut out.x, -10000, 10000, "%d")
        {
            *dirty = true;
        }
        ui.same_line();
        ui.text("Y:");
        ui.same_line();
        ui.set_next_item_width(80.0);
        if crate::widgets::slider_int(ui, &format!("##mirror_oy_{}", idx), &mut out.y, -10000, 10000, "%d")
        {
            *dirty = true;
        }
    }

    ui.text("Scale:");
    ui.same_line();
    ui.set_next_item_width(200.0);
    if crate::widgets::slider_float(ui, &format!("##mirror_oscale_{}", idx), &mut out.scale, 0.1, 10.0, "%.2fx")
    {
        *dirty = true;
    }

    if ui.checkbox(
        format!("Separate X/Y Scale##mirror_sep_{}", idx),
        &mut out.separate_scale,
    ) {
        *dirty = true;
    }

    if out.separate_scale {
        ui.text("Scale X:");
        ui.same_line();
        ui.set_next_item_width(200.0);
        if crate::widgets::slider_float(ui, &format!("##mirror_osx_{}", idx), &mut out.scale_x, 0.1, 10.0, "%.2fx")
        {
            *dirty = true;
        }
        ui.text("Scale Y:");
        ui.same_line();
        ui.set_next_item_width(200.0);
        if crate::widgets::slider_float(ui, &format!("##mirror_osy_{}", idx), &mut out.scale_y, 0.1, 10.0, "%.2fx")
        {
            *dirty = true;
        }
    }

    ui.text("Relative To:");
    ui.same_line();
    if let Some(_token) = ui.begin_combo(format!("##mirror_rel_{}", idx), &out.relative_to) {
        for anchor in &[
            "topLeftScreen",
            "topRightScreen",
            "bottomLeftScreen",
            "bottomRightScreen",
            "center",
        ] {
            if ui.selectable_config(*anchor)
                .selected(out.relative_to == *anchor)
                .build()
            {
                out.relative_to = (*anchor).to_string();
                *dirty = true;
            }
        }
    }
}

fn mirror_colors(
    ui: &imgui::Ui,
    colors: &mut tuxinjector_config::types::MirrorColors,
    idx: usize,
    dirty: &mut bool,
) {
    // output tint
    let mut rgba = [colors.output.r, colors.output.g, colors.output.b, colors.output.a];
    ui.text("Output:");
    ui.same_line();
    if ui.color_edit4(format!("##mirror_cout_{}", idx), &mut rgba) {
        colors.output.r = rgba[0];
        colors.output.g = rgba[1];
        colors.output.b = rgba[2];
        colors.output.a = rgba[3];
        *dirty = true;
    }

    // border color
    let mut rgba = [colors.border.r, colors.border.g, colors.border.b, colors.border.a];
    ui.text("Border:");
    ui.same_line();
    if ui.color_edit4(format!("##mirror_cbrd_{}", idx), &mut rgba) {
        colors.border.r = rgba[0];
        colors.border.g = rgba[1];
        colors.border.b = rgba[2];
        colors.border.a = rgba[3];
        *dirty = true;
    }

    // target colors for visibility matching
    ui.text("Target Colors:");
    let mut rm_tc = None;
    for (i, tc) in colors.target_colors.iter_mut().enumerate() {
        let mut rgba = [tc.r, tc.g, tc.b, tc.a];
        if ui.color_edit4(format!("##mirror_tc_{}_{}", idx, i), &mut rgba) {
            tc.r = rgba[0];
            tc.g = rgba[1];
            tc.b = rgba[2];
            tc.a = rgba[3];
            *dirty = true;
        }
        ui.same_line();
        if ui.small_button(format!("X##mirror_tcrm_{}_{}", idx, i)) {
            rm_tc = Some(i);
            *dirty = true;
        }
    }
    if let Some(ri) = rm_tc {
        colors.target_colors.remove(ri);
    }
    if ui.small_button(format!("Add Target Color##mirror_tcadd_{}", idx)) {
        colors.target_colors.push(tuxinjector_core::Color::WHITE);
        *dirty = true;
    }
}

fn mirror_border(
    ui: &imgui::Ui,
    border: &mut tuxinjector_config::types::MirrorBorderConfig,
    idx: usize,
    dirty: &mut bool,
) {
    let type_str = format!("{:?}", border.r#type);
    ui.text("Type:");
    ui.same_line();
    if let Some(_token) = ui.begin_combo(format!("##mborder_type_{}", idx), &type_str) {
        if ui.selectable_config("Dynamic")
            .selected(border.r#type == MirrorBorderType::Dynamic)
            .build()
        {
            border.r#type = MirrorBorderType::Dynamic;
            *dirty = true;
        }
        if ui.selectable_config("Static")
            .selected(border.r#type == MirrorBorderType::Static)
            .build()
        {
            border.r#type = MirrorBorderType::Static;
            *dirty = true;
        }
    }

    match border.r#type {
        MirrorBorderType::Dynamic => {
            ui.text("Thickness:");
            ui.same_line();
            ui.set_next_item_width(80.0);
            if crate::widgets::slider_int(ui, &format!("##mborder_dthick_{}", idx), &mut border.dynamic_thickness, 0, 50, "%d px")
            {
                *dirty = true;
            }
        }
        MirrorBorderType::Static => {
            let shape_str = format!("{:?}", border.static_shape);
            ui.text("Shape:");
            ui.same_line();
            if let Some(_token) =
                ui.begin_combo(format!("##mborder_shape_{}", idx), &shape_str)
            {
                if ui.selectable_config("Rectangle")
                    .selected(border.static_shape == MirrorBorderShape::Rectangle)
                    .build()
                {
                    border.static_shape = MirrorBorderShape::Rectangle;
                    *dirty = true;
                }
                if ui.selectable_config("Circle")
                    .selected(border.static_shape == MirrorBorderShape::Circle)
                    .build()
                {
                    border.static_shape = MirrorBorderShape::Circle;
                    *dirty = true;
                }
            }

            let mut rgba = [
                border.static_color.r,
                border.static_color.g,
                border.static_color.b,
                border.static_color.a,
            ];
            ui.text("Color:");
            ui.same_line();
            if ui.color_edit4(format!("##mborder_scolor_{}", idx), &mut rgba) {
                border.static_color.r = rgba[0];
                border.static_color.g = rgba[1];
                border.static_color.b = rgba[2];
                border.static_color.a = rgba[3];
                *dirty = true;
            }

            ui.text("Thickness:");
            ui.same_line();
            ui.set_next_item_width(80.0);
            if crate::widgets::slider_int(ui, &format!("##mborder_sthick_{}", idx), &mut border.static_thickness, 0, 50, "%d px")
            {
                *dirty = true;
            }

            ui.text("Radius:");
            ui.same_line();
            ui.set_next_item_width(80.0);
            if crate::widgets::slider_int(ui, &format!("##mborder_sradius_{}", idx), &mut border.static_radius, 0, 200, "%d px")
            {
                *dirty = true;
            }

            ui.text("Offset X:");
            ui.same_line();
            ui.set_next_item_width(80.0);
            if crate::widgets::slider_int(ui, &format!("##mborder_sox_{}", idx), &mut border.static_offset_x, -10000, 10000, "%d")
            {
                *dirty = true;
            }
            ui.same_line();
            ui.text("Y:");
            ui.same_line();
            ui.set_next_item_width(80.0);
            if crate::widgets::slider_int(ui, &format!("##mborder_soy_{}", idx), &mut border.static_offset_y, -10000, 10000, "%d")
            {
                *dirty = true;
            }

            ui.text("Size W:");
            ui.same_line();
            ui.set_next_item_width(80.0);
            if crate::widgets::slider_int(ui, &format!("##mborder_sw_{}", idx), &mut border.static_width, 0, 7680, "%d px")
            {
                *dirty = true;
            }
            ui.same_line();
            ui.text("H:");
            ui.same_line();
            ui.set_next_item_width(80.0);
            if crate::widgets::slider_int(ui, &format!("##mborder_sh_{}", idx), &mut border.static_height, 0, 4320, "%d px")
            {
                *dirty = true;
            }
        }
    }
}
