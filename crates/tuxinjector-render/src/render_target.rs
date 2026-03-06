// Vulkan render target with a HOST_VISIBLE staging buffer for CPU readback,
// plus OPAQUE_FD export so GL can sample the image directly.

use ash::vk;
use tracing;

use crate::renderer::find_memory_type_index;

pub struct ExportableRenderTarget {
    image: vk::Image,
    memory: vk::DeviceMemory,
    view: vk::ImageView,
    fb: vk::Framebuffer,
    staging_buf: vk::Buffer,
    staging_mem: vk::DeviceMemory,
    alloc_size: u64,
    width: u32,
    height: u32,
}

impl ExportableRenderTarget {
    pub fn new(
        device: &ash::Device,
        instance: &ash::Instance,
        phys_dev: vk::PhysicalDevice,
        render_pass: vk::RenderPass,
        width: u32,
        height: u32,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // tag image for OPAQUE_FD export so GL can import it
        let mut ext_info = vk::ExternalMemoryImageCreateInfo::default()
            .handle_types(vk::ExternalMemoryHandleTypeFlags::OPAQUE_FD);

        let img_ci = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .format(vk::Format::R8G8B8A8_UNORM)
            .extent(vk::Extent3D { width, height, depth: 1 })
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_SRC)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .push_next(&mut ext_info);

        let image = unsafe { device.create_image(&img_ci, None)? };
        let mem_reqs = unsafe { device.get_image_memory_requirements(image) };

        let mem_type = find_memory_type_index(
            instance, phys_dev,
            mem_reqs.memory_type_bits,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        ).ok_or("no suitable memory type for render image")?;

        // make it exportable as a POSIX fd
        let mut export_info = vk::ExportMemoryAllocateInfo::default()
            .handle_types(vk::ExternalMemoryHandleTypeFlags::OPAQUE_FD);

        let alloc_ci = vk::MemoryAllocateInfo::default()
            .allocation_size(mem_reqs.size)
            .memory_type_index(mem_type)
            .push_next(&mut export_info);

        let alloc_size = mem_reqs.size;
        let memory = unsafe { device.allocate_memory(&alloc_ci, None)? };
        unsafe { device.bind_image_memory(image, memory, 0)? };

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

        let view = unsafe { device.create_image_view(&view_ci, None)? };

        let fb_ci = vk::FramebufferCreateInfo::default()
            .render_pass(render_pass)
            .attachments(std::slice::from_ref(&view))
            .width(width)
            .height(height)
            .layers(1);

        let fb = unsafe { device.create_framebuffer(&fb_ci, None)? };

        // staging buffer for CPU pixel readback
        let px_bytes = (width * height * 4) as u64;
        let buf_ci = vk::BufferCreateInfo::default()
            .size(px_bytes)
            .usage(vk::BufferUsageFlags::TRANSFER_DST)
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

        tracing::debug!(width, height, "render target created with staging buffer");

        Ok(Self {
            image, memory, view, fb,
            staging_buf, staging_mem,
            alloc_size, width, height,
        })
    }

    pub fn width(&self) -> u32 { self.width }
    pub fn height(&self) -> u32 { self.height }
    pub fn framebuffer(&self) -> vk::Framebuffer { self.fb }
    pub fn image(&self) -> vk::Image { self.image }
    pub fn image_view(&self) -> vk::ImageView { self.view }
    pub fn staging_buffer(&self) -> vk::Buffer { self.staging_buf }
    pub fn allocation_size(&self) -> u64 { self.alloc_size }

    // Export image memory as a new POSIX fd. GL takes ownership of the fd.
    pub unsafe fn export_memory_fd(
        &self,
        ext_mem: &ash::khr::external_memory_fd::Device,
    ) -> Result<(i32, u64), Box<dyn std::error::Error>> {
        let fd_info = vk::MemoryGetFdInfoKHR::default()
            .memory(self.memory)
            .handle_type(vk::ExternalMemoryHandleTypeFlags::OPAQUE_FD);

        let fd = ext_mem.get_memory_fd(&fd_info)?;
        Ok((fd, self.alloc_size))
    }

    // Map the staging buffer and copy pixels out. GPU must be idle first!
    pub unsafe fn read_pixels(&self, device: &ash::Device) -> Vec<u8> {
        let row_sz = (self.width * 4) as usize;
        let total = row_sz * self.height as usize;

        let ptr = device
            .map_memory(self.staging_mem, 0, total as u64, vk::MemoryMapFlags::empty())
            .expect("failed to map staging buffer");

        let src = std::slice::from_raw_parts(ptr as *const u8, total);
        let mut px = vec![0u8; total];
        px.copy_from_slice(src);

        device.unmap_memory(self.staging_mem);
        px
    }

    pub fn destroy(self, device: &ash::Device) {
        unsafe {
            device.destroy_framebuffer(self.fb, None);
            device.destroy_image_view(self.view, None);
            device.destroy_image(self.image, None);
            device.free_memory(self.memory, None);
            device.destroy_buffer(self.staging_buf, None);
            device.free_memory(self.staging_mem, None);
        }
    }
}
