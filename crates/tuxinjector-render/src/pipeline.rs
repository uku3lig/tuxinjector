// Vulkan graphics pipelines for overlay rendering.
// Each pipeline uses a different shader combo but shares the same
// common state (alpha blend, dynamic viewport/scissor, triangle list).

use ash::vk;

const SOLID_VERT_SPV: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/solid.vert.spv"));
const SOLID_FRAG_SPV: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/solid.frag.spv"));

const PASSTHROUGH_VERT_SPV: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/passthrough.vert.spv"));
const PASSTHROUGH_FRAG_SPV: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/passthrough.frag.spv"));

const IMAGE_VERT_SPV: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/image.vert.spv"));
const IMAGE_FRAG_SPV: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/image.frag.spv"));

const GRADIENT_VERT_SPV: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/gradient.vert.spv"));
const GRADIENT_FRAG_SPV: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/gradient.frag.spv"));

const FILTER_VERT_SPV: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/filter.vert.spv"));
const FILTER_FRAG_SPV: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/filter.frag.spv"));

const BORDER_VERT_SPV: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/border.vert.spv"));
const BORDER_FRAG_SPV: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/border.frag.spv"));

pub struct OverlayPipelines {
    pub solid_color_pipeline: vk::Pipeline,
    pub solid_color_layout: vk::PipelineLayout,

    pub passthrough_pipeline: vk::Pipeline,
    pub passthrough_layout: vk::PipelineLayout,
    pub passthrough_desc_set_layout: vk::DescriptorSetLayout,

    pub image_pipeline: vk::Pipeline,
    pub image_layout: vk::PipelineLayout,
    pub image_desc_set_layout: vk::DescriptorSetLayout,

    pub gradient_pipeline: vk::Pipeline,
    pub gradient_layout: vk::PipelineLayout,

    pub filter_pipeline: vk::Pipeline,
    pub filter_layout: vk::PipelineLayout,
    pub filter_desc_set_layout: vk::DescriptorSetLayout,

    pub border_pipeline: vk::Pipeline,
    pub border_layout: vk::PipelineLayout,
}

impl OverlayPipelines {
    pub fn new(
        device: &ash::Device,
        render_pass: vk::RenderPass,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let (solid_color_pipeline, solid_color_layout) =
            create_solid_color_pipeline(device, render_pass)?;
        let (passthrough_pipeline, passthrough_layout, passthrough_desc_set_layout) =
            create_passthrough_pipeline(device, render_pass)?;
        let (image_pipeline, image_layout, image_desc_set_layout) =
            create_image_pipeline(device, render_pass)?;
        let (gradient_pipeline, gradient_layout) =
            create_gradient_pipeline(device, render_pass)?;
        let (filter_pipeline, filter_layout, filter_desc_set_layout) =
            create_filter_pipeline(device, render_pass)?;
        let (border_pipeline, border_layout) =
            create_border_pipeline(device, render_pass)?;

        Ok(Self {
            solid_color_pipeline, solid_color_layout,
            passthrough_pipeline, passthrough_layout, passthrough_desc_set_layout,
            image_pipeline, image_layout, image_desc_set_layout,
            gradient_pipeline, gradient_layout,
            filter_pipeline, filter_layout, filter_desc_set_layout,
            border_pipeline, border_layout,
        })
    }

    pub fn destroy(&self, device: &ash::Device) {
        unsafe {
            device.destroy_pipeline(self.solid_color_pipeline, None);
            device.destroy_pipeline_layout(self.solid_color_layout, None);

            device.destroy_pipeline(self.passthrough_pipeline, None);
            device.destroy_pipeline_layout(self.passthrough_layout, None);
            device.destroy_descriptor_set_layout(self.passthrough_desc_set_layout, None);

            device.destroy_pipeline(self.image_pipeline, None);
            device.destroy_pipeline_layout(self.image_layout, None);
            device.destroy_descriptor_set_layout(self.image_desc_set_layout, None);

            device.destroy_pipeline(self.gradient_pipeline, None);
            device.destroy_pipeline_layout(self.gradient_layout, None);

            device.destroy_pipeline(self.filter_pipeline, None);
            device.destroy_pipeline_layout(self.filter_layout, None);
            device.destroy_descriptor_set_layout(self.filter_desc_set_layout, None);

            device.destroy_pipeline(self.border_pipeline, None);
            device.destroy_pipeline_layout(self.border_layout, None);
        }
    }
}

unsafe fn create_shader_module(
    device: &ash::Device,
    spv: &[u8],
) -> Result<vk::ShaderModule, vk::Result> {
    assert!(spv.len() % 4 == 0, "SPIR-V blob size not a multiple of 4");
    let code: Vec<u32> = spv
        .chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect();

    let ci = vk::ShaderModuleCreateInfo::default().code(&code);
    device.create_shader_module(&ci, None)
}

// -- shared pipeline state helpers --

fn common_rasterization() -> vk::PipelineRasterizationStateCreateInfo<'static> {
    vk::PipelineRasterizationStateCreateInfo::default()
        .depth_clamp_enable(false)
        .rasterizer_discard_enable(false)
        .polygon_mode(vk::PolygonMode::FILL)
        .cull_mode(vk::CullModeFlags::NONE)
        .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
        .depth_bias_enable(false)
        .line_width(1.0)
}

fn common_multisample() -> vk::PipelineMultisampleStateCreateInfo<'static> {
    vk::PipelineMultisampleStateCreateInfo::default()
        .rasterization_samples(vk::SampleCountFlags::TYPE_1)
        .sample_shading_enable(false)
}

fn alpha_blend_attachment() -> vk::PipelineColorBlendAttachmentState {
    vk::PipelineColorBlendAttachmentState::default()
        .blend_enable(true)
        .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
        .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
        .color_blend_op(vk::BlendOp::ADD)
        .src_alpha_blend_factor(vk::BlendFactor::ONE)
        .dst_alpha_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
        .alpha_blend_op(vk::BlendOp::ADD)
        .color_write_mask(vk::ColorComponentFlags::RGBA)
}

fn tri_list_input_assembly() -> vk::PipelineInputAssemblyStateCreateInfo<'static> {
    vk::PipelineInputAssemblyStateCreateInfo::default()
        .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
        .primitive_restart_enable(false)
}

fn sampler_desc_layout(
    device: &ash::Device,
) -> Result<vk::DescriptorSetLayout, vk::Result> {
    let binding = vk::DescriptorSetLayoutBinding::default()
        .binding(0)
        .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
        .descriptor_count(1)
        .stage_flags(vk::ShaderStageFlags::FRAGMENT);

    let ci = vk::DescriptorSetLayoutCreateInfo::default()
        .bindings(std::slice::from_ref(&binding));

    unsafe { device.create_descriptor_set_layout(&ci, None) }
}

// stride 16: two vec2s (pos + uv)
fn textured_vertex_bindings() -> [vk::VertexInputBindingDescription; 1] {
    [vk::VertexInputBindingDescription {
        binding: 0,
        stride: 16,
        input_rate: vk::VertexInputRate::VERTEX,
    }]
}

fn textured_vertex_attributes() -> [vk::VertexInputAttributeDescription; 2] {
    [
        vk::VertexInputAttributeDescription {
            location: 0, binding: 0,
            format: vk::Format::R32G32_SFLOAT, offset: 0,
        },
        vk::VertexInputAttributeDescription {
            location: 1, binding: 0,
            format: vk::Format::R32G32_SFLOAT, offset: 8,
        },
    ]
}

// Build a pipeline with dynamic viewport+scissor and standard alpha blending
unsafe fn build_pipeline(
    device: &ash::Device,
    render_pass: vk::RenderPass,
    stages: &[vk::PipelineShaderStageCreateInfo],
    vert_input: &vk::PipelineVertexInputStateCreateInfo,
    layout: vk::PipelineLayout,
) -> Result<vk::Pipeline, Box<dyn std::error::Error>> {
    let input_asm = tri_list_input_assembly();

    let vp_state = vk::PipelineViewportStateCreateInfo::default()
        .viewport_count(1)
        .scissor_count(1);

    let raster = common_rasterization();
    let ms = common_multisample();
    let blend_att = alpha_blend_attachment();

    let color_blend = vk::PipelineColorBlendStateCreateInfo::default()
        .logic_op_enable(false)
        .attachments(std::slice::from_ref(&blend_att));

    let dyn_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
    let dyn_state = vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dyn_states);

    let ci = vk::GraphicsPipelineCreateInfo::default()
        .stages(stages)
        .vertex_input_state(vert_input)
        .input_assembly_state(&input_asm)
        .viewport_state(&vp_state)
        .rasterization_state(&raster)
        .multisample_state(&ms)
        .color_blend_state(&color_blend)
        .dynamic_state(&dyn_state)
        .layout(layout)
        .render_pass(render_pass)
        .subpass(0);

    let pipelines = device
        .create_graphics_pipelines(
            vk::PipelineCache::null(),
            std::slice::from_ref(&ci),
            None,
        )
        .map_err(|(_, err)| err)?;

    Ok(pipelines[0])
}


// -- individual pipeline constructors --
// These are all structurally identical, just with different shaders and push constant sizes.
// TODO: could probably macro this, but it's not really worth the readability hit imo

fn create_solid_color_pipeline(
    device: &ash::Device,
    render_pass: vk::RenderPass,
) -> Result<(vk::Pipeline, vk::PipelineLayout), Box<dyn std::error::Error>> {
    let vs = unsafe { create_shader_module(device, SOLID_VERT_SPV)? };
    let fs = unsafe { create_shader_module(device, SOLID_FRAG_SPV)? };

    let entry = c"main";
    let stages = [
        vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::VERTEX).module(vs).name(entry),
        vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::FRAGMENT).module(fs).name(entry),
    ];

    let vert_input = vk::PipelineVertexInputStateCreateInfo::default();

    let pc = vk::PushConstantRange {
        stage_flags: vk::ShaderStageFlags::FRAGMENT,
        offset: 0, size: 16, // vec4
    };

    let layout_ci = vk::PipelineLayoutCreateInfo::default()
        .push_constant_ranges(std::slice::from_ref(&pc));
    let layout = unsafe { device.create_pipeline_layout(&layout_ci, None)? };

    let pipeline = unsafe { build_pipeline(device, render_pass, &stages, &vert_input, layout)? };

    unsafe {
        device.destroy_shader_module(vs, None);
        device.destroy_shader_module(fs, None);
    }

    Ok((pipeline, layout))
}

fn create_passthrough_pipeline(
    device: &ash::Device,
    render_pass: vk::RenderPass,
) -> Result<(vk::Pipeline, vk::PipelineLayout, vk::DescriptorSetLayout), Box<dyn std::error::Error>>
{
    let vs = unsafe { create_shader_module(device, PASSTHROUGH_VERT_SPV)? };
    let fs = unsafe { create_shader_module(device, PASSTHROUGH_FRAG_SPV)? };

    let entry = c"main";
    let stages = [
        vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::VERTEX).module(vs).name(entry),
        vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::FRAGMENT).module(fs).name(entry),
    ];

    let bindings = textured_vertex_bindings();
    let attrs = textured_vertex_attributes();
    let vert_input = vk::PipelineVertexInputStateCreateInfo::default()
        .vertex_binding_descriptions(&bindings)
        .vertex_attribute_descriptions(&attrs);

    let dsl = sampler_desc_layout(device)?;

    let pc = vk::PushConstantRange {
        stage_flags: vk::ShaderStageFlags::FRAGMENT,
        offset: 0, size: 4,
    };
    let layout_ci = vk::PipelineLayoutCreateInfo::default()
        .set_layouts(std::slice::from_ref(&dsl))
        .push_constant_ranges(std::slice::from_ref(&pc));
    let layout = unsafe { device.create_pipeline_layout(&layout_ci, None)? };

    let pipeline = unsafe { build_pipeline(device, render_pass, &stages, &vert_input, layout)? };

    unsafe {
        device.destroy_shader_module(vs, None);
        device.destroy_shader_module(fs, None);
    }

    Ok((pipeline, layout, dsl))
}

fn create_image_pipeline(
    device: &ash::Device,
    render_pass: vk::RenderPass,
) -> Result<(vk::Pipeline, vk::PipelineLayout, vk::DescriptorSetLayout), Box<dyn std::error::Error>>
{
    let vs = unsafe { create_shader_module(device, IMAGE_VERT_SPV)? };
    let fs = unsafe { create_shader_module(device, IMAGE_FRAG_SPV)? };

    let entry = c"main";
    let stages = [
        vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::VERTEX).module(vs).name(entry),
        vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::FRAGMENT).module(fs).name(entry),
    ];

    let bindings = textured_vertex_bindings();
    let attrs = textured_vertex_attributes();
    let vert_input = vk::PipelineVertexInputStateCreateInfo::default()
        .vertex_binding_descriptions(&bindings)
        .vertex_attribute_descriptions(&attrs);

    let dsl = sampler_desc_layout(device)?;

    // 48 bytes: opacity, enableColorKey, colorKey, colorKeySensitivity
    let pc = vk::PushConstantRange {
        stage_flags: vk::ShaderStageFlags::FRAGMENT,
        offset: 0, size: 48,
    };
    let layout_ci = vk::PipelineLayoutCreateInfo::default()
        .set_layouts(std::slice::from_ref(&dsl))
        .push_constant_ranges(std::slice::from_ref(&pc));
    let layout = unsafe { device.create_pipeline_layout(&layout_ci, None)? };

    let pipeline = unsafe { build_pipeline(device, render_pass, &stages, &vert_input, layout)? };

    unsafe {
        device.destroy_shader_module(vs, None);
        device.destroy_shader_module(fs, None);
    }

    Ok((pipeline, layout, dsl))
}

fn create_gradient_pipeline(
    device: &ash::Device,
    render_pass: vk::RenderPass,
) -> Result<(vk::Pipeline, vk::PipelineLayout), Box<dyn std::error::Error>> {
    let vs = unsafe { create_shader_module(device, GRADIENT_VERT_SPV)? };
    let fs = unsafe { create_shader_module(device, GRADIENT_FRAG_SPV)? };

    let entry = c"main";
    let stages = [
        vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::VERTEX).module(vs).name(entry),
        vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::FRAGMENT).module(fs).name(entry),
    ];

    let vert_input = vk::PipelineVertexInputStateCreateInfo::default();

    // 48 bytes: color1, color2, angle, time, animationType
    let pc = vk::PushConstantRange {
        stage_flags: vk::ShaderStageFlags::FRAGMENT,
        offset: 0, size: 48,
    };
    let layout_ci = vk::PipelineLayoutCreateInfo::default()
        .push_constant_ranges(std::slice::from_ref(&pc));
    let layout = unsafe { device.create_pipeline_layout(&layout_ci, None)? };

    let pipeline = unsafe { build_pipeline(device, render_pass, &stages, &vert_input, layout)? };

    unsafe {
        device.destroy_shader_module(vs, None);
        device.destroy_shader_module(fs, None);
    }

    Ok((pipeline, layout))
}

fn create_filter_pipeline(
    device: &ash::Device,
    render_pass: vk::RenderPass,
) -> Result<(vk::Pipeline, vk::PipelineLayout, vk::DescriptorSetLayout), Box<dyn std::error::Error>>
{
    let vs = unsafe { create_shader_module(device, FILTER_VERT_SPV)? };
    let fs = unsafe { create_shader_module(device, FILTER_FRAG_SPV)? };

    let entry = c"main";
    let stages = [
        vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::VERTEX).module(vs).name(entry),
        vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::FRAGMENT).module(fs).name(entry),
    ];

    let bindings = textured_vertex_bindings();
    let attrs = textured_vertex_attributes();
    let vert_input = vk::PipelineVertexInputStateCreateInfo::default()
        .vertex_binding_descriptions(&bindings)
        .vertex_attribute_descriptions(&attrs);

    let dsl = sampler_desc_layout(device)?;

    // 128 bytes: targetColors[4], outputColor, borderColor, etc
    let pc = vk::PushConstantRange {
        stage_flags: vk::ShaderStageFlags::FRAGMENT,
        offset: 0, size: 128,
    };
    let layout_ci = vk::PipelineLayoutCreateInfo::default()
        .set_layouts(std::slice::from_ref(&dsl))
        .push_constant_ranges(std::slice::from_ref(&pc));
    let layout = unsafe { device.create_pipeline_layout(&layout_ci, None)? };

    let pipeline = unsafe { build_pipeline(device, render_pass, &stages, &vert_input, layout)? };

    unsafe {
        device.destroy_shader_module(vs, None);
        device.destroy_shader_module(fs, None);
    }

    Ok((pipeline, layout, dsl))
}

fn create_border_pipeline(
    device: &ash::Device,
    render_pass: vk::RenderPass,
) -> Result<(vk::Pipeline, vk::PipelineLayout), Box<dyn std::error::Error>> {
    let vs = unsafe { create_shader_module(device, BORDER_VERT_SPV)? };
    let fs = unsafe { create_shader_module(device, BORDER_FRAG_SPV)? };

    let entry = c"main";
    let stages = [
        vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::VERTEX).module(vs).name(entry),
        vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::FRAGMENT).module(fs).name(entry),
    ];

    let vert_input = vk::PipelineVertexInputStateCreateInfo::default();

    // 48 bytes: color, rect, borderWidth, radius, resolution
    let pc = vk::PushConstantRange {
        stage_flags: vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
        offset: 0, size: 48,
    };
    let layout_ci = vk::PipelineLayoutCreateInfo::default()
        .push_constant_ranges(std::slice::from_ref(&pc));
    let layout = unsafe { device.create_pipeline_layout(&layout_ci, None)? };

    let pipeline = unsafe { build_pipeline(device, render_pass, &stages, &vert_input, layout)? };

    unsafe {
        device.destroy_shader_module(vs, None);
        device.destroy_shader_module(fs, None);
    }

    Ok((pipeline, layout))
}
