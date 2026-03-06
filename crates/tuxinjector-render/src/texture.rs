// GPU-resident RGBA texture, uploaded from CPU pixels through a staging buffer.
// This is all Vulkan and was used before the GL-direct rewrite -- kept around
// because the plugin API still references it for image overlays.

use ash::vk;

use crate::renderer::find_memory_type_index;

pub struct VulkanTexture {
    pub image: vk::Image,
    pub memory: vk::DeviceMemory,
    pub image_view: vk::ImageView,
    pub sampler: vk::Sampler,
    pub width: u32,
    pub height: u32,
}

impl VulkanTexture {
    // Upload RGBA pixels into a new GPU texture via a staging buffer
    pub fn upload(
        device: &ash::Device,
        instance: &ash::Instance,
        phys_dev: vk::PhysicalDevice,
        queue: vk::Queue,
        cmd_pool: vk::CommandPool,
        width: u32,
        height: u32,
        pixels: &[u8],
        filter: vk::Filter,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let expected = (width as usize) * (height as usize) * 4;
        assert_eq!(
            pixels.len(), expected,
            "pixel data size mismatch: expected {expected}, got {}", pixels.len()
        );

        // -- create the destination image --

        let img_ci = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .format(vk::Format::R8G8B8A8_UNORM)
            .extent(vk::Extent3D { width, height, depth: 1 })
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .initial_layout(vk::ImageLayout::UNDEFINED);

        let image = unsafe { device.create_image(&img_ci, None)? };
        let mem_reqs = unsafe { device.get_image_memory_requirements(image) };

        let mem_type = find_memory_type_index(
            instance, phys_dev,
            mem_reqs.memory_type_bits,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        ).ok_or("no device-local memory type for texture image")?;

        let alloc = vk::MemoryAllocateInfo::default()
            .allocation_size(mem_reqs.size)
            .memory_type_index(mem_type);

        let memory = unsafe { device.allocate_memory(&alloc, None)? };
        unsafe { device.bind_image_memory(image, memory, 0)? };

        // -- staging buffer --

        let buf_size = pixels.len() as u64;
        let buf_ci = vk::BufferCreateInfo::default()
            .size(buf_size)
            .usage(vk::BufferUsageFlags::TRANSFER_SRC)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let staging_buf = unsafe { device.create_buffer(&buf_ci, None)? };
        let buf_reqs = unsafe { device.get_buffer_memory_requirements(staging_buf) };

        let staging_type = find_memory_type_index(
            instance, phys_dev,
            buf_reqs.memory_type_bits,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        ).ok_or("no host-visible memory for staging buffer")?;

        let staging_alloc = vk::MemoryAllocateInfo::default()
            .allocation_size(buf_reqs.size)
            .memory_type_index(staging_type);

        let staging_mem = unsafe { device.allocate_memory(&staging_alloc, None)? };
        unsafe { device.bind_buffer_memory(staging_buf, staging_mem, 0)? };

        // memcpy into staging
        unsafe {
            let ptr = device.map_memory(staging_mem, 0, buf_size, vk::MemoryMapFlags::empty())?;
            std::ptr::copy_nonoverlapping(pixels.as_ptr(), ptr as *mut u8, pixels.len());
            device.unmap_memory(staging_mem);
        }

        // -- record + submit the copy command --

        let cb_alloc = vk::CommandBufferAllocateInfo::default()
            .command_pool(cmd_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);

        let cbs = unsafe { device.allocate_command_buffers(&cb_alloc)? };
        let cb = cbs[0];

        unsafe {
            let begin = vk::CommandBufferBeginInfo::default()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            device.begin_command_buffer(cb, &begin)?;

            // transition to TRANSFER_DST
            let to_transfer = vk::ImageMemoryBarrier::default()
                .src_access_mask(vk::AccessFlags::empty())
                .dst_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                .old_layout(vk::ImageLayout::UNDEFINED)
                .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .image(image)
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0, level_count: 1,
                    base_array_layer: 0, layer_count: 1,
                });

            device.cmd_pipeline_barrier(
                cb,
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::TRANSFER,
                vk::DependencyFlags::empty(),
                &[], &[],
                std::slice::from_ref(&to_transfer),
            );

            let region = vk::BufferImageCopy {
                buffer_offset: 0,
                buffer_row_length: 0,
                buffer_image_height: 0,
                image_subresource: vk::ImageSubresourceLayers {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    mip_level: 0, base_array_layer: 0, layer_count: 1,
                },
                image_offset: vk::Offset3D { x: 0, y: 0, z: 0 },
                image_extent: vk::Extent3D { width, height, depth: 1 },
            };

            device.cmd_copy_buffer_to_image(
                cb, staging_buf, image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                std::slice::from_ref(&region),
            );

            // transition to SHADER_READ
            let to_shader = vk::ImageMemoryBarrier::default()
                .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                .dst_access_mask(vk::AccessFlags::SHADER_READ)
                .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .image(image)
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0, level_count: 1,
                    base_array_layer: 0, layer_count: 1,
                });

            device.cmd_pipeline_barrier(
                cb,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::FRAGMENT_SHADER,
                vk::DependencyFlags::empty(),
                &[], &[],
                std::slice::from_ref(&to_shader),
            );

            device.end_command_buffer(cb)?;

            let submit = vk::SubmitInfo::default()
                .command_buffers(std::slice::from_ref(&cb));

            device.queue_submit(queue, std::slice::from_ref(&submit), vk::Fence::null())?;
            device.queue_wait_idle(queue)?;

            // cleanup staging resources
            device.free_command_buffers(cmd_pool, &[cb]);
            device.destroy_buffer(staging_buf, None);
            device.free_memory(staging_mem, None);
        }

        // -- image view + sampler --

        let view_ci = vk::ImageViewCreateInfo::default()
            .image(image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(vk::Format::R8G8B8A8_UNORM)
            .components(vk::ComponentMapping::default())
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0, level_count: 1,
                base_array_layer: 0, layer_count: 1,
            });

        let image_view = unsafe { device.create_image_view(&view_ci, None)? };

        let mip_mode = if filter == vk::Filter::NEAREST {
            vk::SamplerMipmapMode::NEAREST
        } else {
            vk::SamplerMipmapMode::LINEAR
        };

        let sampler_ci = vk::SamplerCreateInfo::default()
            .mag_filter(filter)
            .min_filter(filter)
            .mipmap_mode(mip_mode)
            .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .mip_lod_bias(0.0)
            .anisotropy_enable(false)
            .max_anisotropy(1.0)
            .compare_enable(false)
            .min_lod(0.0)
            .max_lod(0.0)
            .border_color(vk::BorderColor::FLOAT_TRANSPARENT_BLACK)
            .unnormalized_coordinates(false);

        let sampler = unsafe { device.create_sampler(&sampler_ci, None)? };

        Ok(Self { image, memory, image_view, sampler, width, height })
    }

    // Write this texture into a descriptor set at the given binding
    pub fn write_descriptor_set(
        &self,
        device: &ash::Device,
        desc_set: vk::DescriptorSet,
        binding: u32,
    ) {
        let info = vk::DescriptorImageInfo::default()
            .sampler(self.sampler)
            .image_view(self.image_view)
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);

        let write = vk::WriteDescriptorSet::default()
            .dst_set(desc_set)
            .dst_binding(binding)
            .dst_array_element(0)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .image_info(std::slice::from_ref(&info));

        unsafe {
            device.update_descriptor_sets(std::slice::from_ref(&write), &[]);
        }
    }

    pub fn destroy(&self, device: &ash::Device) {
        unsafe {
            device.destroy_sampler(self.sampler, None);
            device.destroy_image_view(self.image_view, None);
            device.destroy_image(self.image, None);
            device.free_memory(self.memory, None);
        }
    }
}
