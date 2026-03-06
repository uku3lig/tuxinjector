// "Our" own private Vulkan instance+device for rendering overlay frames.
// Completely independent from the game's GL context -- we just export
// the final image via EXT_memory_object_fd for GL to sample.

use std::ffi::CStr;

use ash::vk;
use tracing;

use crate::pipeline::OverlayPipelines;
use crate::render_target::ExportableRenderTarget;

pub struct VulkanRenderer {
    pub entry: ash::Entry,
    pub instance: ash::Instance,
    pub physical_device: vk::PhysicalDevice,
    pub device: ash::Device,
    pub graphics_queue: vk::Queue,
    pub queue_family_index: u32,
    pub command_pool: vk::CommandPool,
    pub render_pass: vk::RenderPass,
    pub pipelines: OverlayPipelines,
    pub descriptor_pool: vk::DescriptorPool,
    cb: vk::CommandBuffer,
    rt: Option<ExportableRenderTarget>,
    ext_mem_fd: ash::khr::external_memory_fd::Device,
    fence: vk::Fence,
    fence_submitted: bool,
}

impl VulkanRenderer {
    // Spin up a new renderer. Tries to pick a discrete GPU, falls back to whatever's available.
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let entry = unsafe { ash::Entry::load()? };

        let app_info = vk::ApplicationInfo::default()
            .application_name(c"tuxinjector-overlay")
            .application_version(vk::make_api_version(0, 0, 1, 0))
            .engine_name(c"tuxinjector")
            .engine_version(vk::make_api_version(0, 0, 1, 0))
            .api_version(vk::API_VERSION_1_1);

        #[allow(unused_mut)]
        let mut inst_exts: Vec<*const i8> = vec![];
        #[allow(unused_mut)]
        let mut layers: Vec<*const i8> = Vec::new();

        #[cfg(debug_assertions)]
        {
            let validation = c"VK_LAYER_KHRONOS_validation";
            let available = unsafe { entry.enumerate_instance_layer_properties()? };
            let has_it = available.iter().any(|l| unsafe {
                CStr::from_ptr(l.layer_name.as_ptr()) == validation
            });
            if has_it {
                layers.push(validation.as_ptr());
                inst_exts.push(ash::ext::debug_utils::NAME.as_ptr());
                tracing::debug!("validation layer enabled");
            } else {
                tracing::debug!("validation layer not found, skipping");
            }
        }

        let inst_ci = vk::InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_extension_names(&inst_exts)
            .enabled_layer_names(&layers);

        let instance = unsafe { entry.create_instance(&inst_ci, None)? };

        let phys_devs = unsafe { instance.enumerate_physical_devices()? };
        if phys_devs.is_empty() {
            return Err("no Vulkan physical devices".into());
        }

        // prefer discrete, settle for anything
        let physical_device = phys_devs
            .iter()
            .copied()
            .find(|&pd| {
                let props = unsafe { instance.get_physical_device_properties(pd) };
                props.device_type == vk::PhysicalDeviceType::DISCRETE_GPU
            })
            .unwrap_or(phys_devs[0]);

        let props = unsafe { instance.get_physical_device_properties(physical_device) };
        let name = unsafe { CStr::from_ptr(props.device_name.as_ptr()) };
        tracing::info!(device = %name.to_string_lossy(), "selected Vulkan device");

        let qf_idx = {
            let families =
                unsafe { instance.get_physical_device_queue_family_properties(physical_device) };
            families
                .iter()
                .enumerate()
                .find(|(_, p)| p.queue_flags.contains(vk::QueueFlags::GRAPHICS))
                .map(|(i, _)| i as u32)
                .ok_or("no graphics queue family")?
        };

        let prios = [1.0f32];
        let queue_ci = vk::DeviceQueueCreateInfo::default()
            .queue_family_index(qf_idx)
            .queue_priorities(&prios);

        let dev_exts = [ash::khr::external_memory_fd::NAME.as_ptr()];

        let dev_ci = vk::DeviceCreateInfo::default()
            .queue_create_infos(std::slice::from_ref(&queue_ci))
            .enabled_extension_names(&dev_exts);

        let device = unsafe { instance.create_device(physical_device, &dev_ci, None)? };

        let ext_mem_fd = ash::khr::external_memory_fd::Device::new(&instance, &device);
        let gfx_queue = unsafe { device.get_device_queue(qf_idx, 0) };

        let pool_ci = vk::CommandPoolCreateInfo::default()
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
            .queue_family_index(qf_idx);
        let command_pool = unsafe { device.create_command_pool(&pool_ci, None)? };

        let cb_alloc = vk::CommandBufferAllocateInfo::default()
            .command_pool(command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);
        let cbs = unsafe { device.allocate_command_buffers(&cb_alloc)? };
        let cb = cbs[0];

        // render pass: clear to transparent, end in GENERAL for export
        let color_att = vk::AttachmentDescription::default()
            .format(vk::Format::R8G8B8A8_UNORM)
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::GENERAL);

        let color_ref = vk::AttachmentReference::default()
            .attachment(0)
            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);

        let subpass = vk::SubpassDescription::default()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(std::slice::from_ref(&color_ref));

        // flush writes before GL reads them
        let dep = vk::SubpassDependency::default()
            .src_subpass(0)
            .dst_subpass(vk::SUBPASS_EXTERNAL)
            .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .dst_stage_mask(vk::PipelineStageFlags::BOTTOM_OF_PIPE)
            .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
            .dst_access_mask(vk::AccessFlags::empty());

        let rp_ci = vk::RenderPassCreateInfo::default()
            .attachments(std::slice::from_ref(&color_att))
            .subpasses(std::slice::from_ref(&subpass))
            .dependencies(std::slice::from_ref(&dep));

        let render_pass = unsafe { device.create_render_pass(&rp_ci, None)? };

        let pool_sizes = [vk::DescriptorPoolSize {
            ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
            descriptor_count: 16,
        }];
        let dp_ci = vk::DescriptorPoolCreateInfo::default()
            .max_sets(16)
            .pool_sizes(&pool_sizes);
        let descriptor_pool = unsafe { device.create_descriptor_pool(&dp_ci, None)? };

        let pipelines = OverlayPipelines::new(&device, render_pass)?;

        let fence = unsafe { device.create_fence(&vk::FenceCreateInfo::default(), None)? };

        tracing::info!("Vulkan renderer initialized (VK_KHR_external_memory_fd)");

        Ok(Self {
            entry,
            instance,
            physical_device,
            device,
            graphics_queue: gfx_queue,
            queue_family_index: qf_idx,
            command_pool,
            render_pass,
            pipelines,
            descriptor_pool,
            cb,
            rt: None,
            ext_mem_fd,
            fence,
            fence_submitted: false,
        })
    }

    // Make sure RT exists at the right size, recreating if needed
    pub fn ensure_render_target(
        &mut self,
        w: u32,
        h: u32,
    ) -> Result<&ExportableRenderTarget, Box<dyn std::error::Error>> {
        let stale = match &self.rt {
            Some(rt) => rt.width() != w || rt.height() != h,
            None => true,
        };

        if stale {
            if let Some(old) = self.rt.take() {
                unsafe { self.device.device_wait_idle()? };
                old.destroy(&self.device);
            }
            let rt = ExportableRenderTarget::new(
                &self.device, &self.instance, self.physical_device,
                self.render_pass, w, h,
            )?;
            self.rt = Some(rt);
            tracing::debug!(w, h, "render target (re)created");
        }

        Ok(self.rt.as_ref().unwrap())
    }

    // Begin recording a frame, returns the command buffer for draw calls
    pub fn begin_frame(
        &mut self,
        w: u32,
        h: u32,
    ) -> Result<vk::CommandBuffer, Box<dyn std::error::Error>> {
        self.ensure_render_target(w, h)?;
        let rt = self.rt.as_ref().unwrap();

        unsafe {
            self.device.reset_command_buffer(self.cb, vk::CommandBufferResetFlags::empty())?;

            let begin = vk::CommandBufferBeginInfo::default()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            self.device.begin_command_buffer(self.cb, &begin)?;

            let clear = [vk::ClearValue {
                color: vk::ClearColorValue { float32: [0.0, 0.0, 0.0, 0.0] },
            }];

            let rp_begin = vk::RenderPassBeginInfo::default()
                .render_pass(self.render_pass)
                .framebuffer(rt.framebuffer())
                .render_area(vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent: vk::Extent2D { width: w, height: h },
                })
                .clear_values(&clear);

            self.device.cmd_begin_render_pass(self.cb, &rp_begin, vk::SubpassContents::INLINE);

            let viewport = vk::Viewport {
                x: 0.0, y: 0.0,
                width: w as f32, height: h as f32,
                min_depth: 0.0, max_depth: 1.0,
            };
            self.device.cmd_set_viewport(self.cb, 0, &[viewport]);

            let scissor = vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: vk::Extent2D { width: w, height: h },
            };
            self.device.cmd_set_scissor(self.cb, 0, &[scissor]);
        }

        Ok(self.cb)
    }

    // Finish the frame and read back RGBA pixels from the staging buffer
    pub fn end_frame(&mut self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let (w, h, staging, img) = {
            let rt = self.rt.as_ref().ok_or("no render target")?;
            (rt.width(), rt.height(), rt.staging_buffer(), rt.image())
        };

        unsafe {
            self.device.cmd_end_render_pass(self.cb);

            // barrier: color writes -> transfer read
            let barrier = vk::ImageMemoryBarrier::default()
                .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
                .dst_access_mask(vk::AccessFlags::TRANSFER_READ)
                .old_layout(vk::ImageLayout::GENERAL)
                .new_layout(vk::ImageLayout::GENERAL)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .image(img)
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0, level_count: 1,
                    base_array_layer: 0, layer_count: 1,
                });

            self.device.cmd_pipeline_barrier(
                self.cb,
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                vk::PipelineStageFlags::TRANSFER,
                vk::DependencyFlags::empty(),
                &[], &[],
                std::slice::from_ref(&barrier),
            );

            let region = vk::BufferImageCopy::default()
                .buffer_offset(0)
                .buffer_row_length(0)
                .buffer_image_height(0)
                .image_subresource(vk::ImageSubresourceLayers {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    mip_level: 0, base_array_layer: 0, layer_count: 1,
                })
                .image_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
                .image_extent(vk::Extent3D { width: w, height: h, depth: 1 });

            self.device.cmd_copy_image_to_buffer(
                self.cb, img,
                vk::ImageLayout::GENERAL,
                staging,
                std::slice::from_ref(&region),
            );

            self.device.end_command_buffer(self.cb)?;

            let submit = vk::SubmitInfo::default()
                .command_buffers(std::slice::from_ref(&self.cb));
            self.device.queue_submit(
                self.graphics_queue,
                std::slice::from_ref(&submit),
                vk::Fence::null(),
            )?;
            self.device.queue_wait_idle(self.graphics_queue)?;
        }

        let pixels = unsafe { self.rt.as_ref().unwrap().read_pixels(&self.device) };
        Ok(pixels)
    }

    // End frame for interop path - no readback, just submit with a fence
    pub fn end_frame_no_readback(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let img = {
            let rt = self.rt.as_ref().ok_or("no render target")?;
            rt.image()
        };

        unsafe {
            self.device.cmd_end_render_pass(self.cb);

            // make color writes visible to GL
            let barrier = vk::ImageMemoryBarrier::default()
                .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
                .dst_access_mask(vk::AccessFlags::MEMORY_READ)
                .old_layout(vk::ImageLayout::GENERAL)
                .new_layout(vk::ImageLayout::GENERAL)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .image(img)
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0, level_count: 1,
                    base_array_layer: 0, layer_count: 1,
                });

            self.device.cmd_pipeline_barrier(
                self.cb,
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                vk::DependencyFlags::empty(),
                &[], &[],
                std::slice::from_ref(&barrier),
            );

            self.device.end_command_buffer(self.cb)?;

            self.device.reset_fences(std::slice::from_ref(&self.fence))?;

            let submit = vk::SubmitInfo::default()
                .command_buffers(std::slice::from_ref(&self.cb));
            self.device.queue_submit(
                self.graphics_queue,
                std::slice::from_ref(&submit),
                self.fence,
            )?;

            self.fence_submitted = true;
        }

        Ok(())
    }

    // Block until the interop fence signals. Must happen before next begin_frame.
    pub fn wait_for_interop_fence(&mut self) {
        if self.fence_submitted {
            unsafe {
                let _ = self.device.wait_for_fences(
                    std::slice::from_ref(&self.fence),
                    true,
                    u64::MAX,
                );
            }
            self.fence_submitted = false;
        }
    }

    // Get a fresh fd for GL to import our render target memory
    pub unsafe fn export_memory_fd(
        &self,
    ) -> Result<(i32, u64), Box<dyn std::error::Error>> {
        let rt = self.rt.as_ref().ok_or("no render target to export")?;
        rt.export_memory_fd(&self.ext_mem_fd)
    }

    pub fn render_target(&self) -> Option<&ExportableRenderTarget> {
        self.rt.as_ref()
    }

    pub fn destroy(&mut self) {
        unsafe {
            let _ = self.device.device_wait_idle();

            if let Some(rt) = self.rt.take() {
                rt.destroy(&self.device);
            }

            self.pipelines.destroy(&self.device);

            self.device.destroy_fence(self.fence, None);
            self.device.destroy_descriptor_pool(self.descriptor_pool, None);
            self.device.destroy_command_pool(self.command_pool, None);
            self.device.destroy_render_pass(self.render_pass, None);
            self.device.destroy_device(None);
            self.instance.destroy_instance(None);
        }
    }
}

impl Drop for VulkanRenderer {
    fn drop(&mut self) {
        self.destroy();
    }
}

// Find a memory type matching the required type bits and property flags.
pub fn find_memory_type_index(
    instance: &ash::Instance,
    phys_dev: vk::PhysicalDevice,
    type_bits: u32,
    props: vk::MemoryPropertyFlags,
) -> Option<u32> {
    let mem_props = unsafe { instance.get_physical_device_memory_properties(phys_dev) };
    (0..mem_props.memory_type_count).find(|&i| {
        let ok_type = (type_bits & (1 << i)) != 0;
        let ok_props = mem_props.memory_types[i as usize].property_flags.contains(props);
        ok_type && ok_props
    })
}
