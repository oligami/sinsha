mod extent;

pub use usage::ImageUsage;
pub use extent::*;

use super::*;
use std::ops::Range;

pub struct Image<I, D, M, A, E> where
    I: Borrow<Instance> + Deref<Target = Instance>,
    D: Borrow<Device<I>> + Deref<Target = Device<I>>,
    M: Borrow<DeviceMemory<I, D, A>> + Deref<Target = DeviceMemory<I, D, A>>,
    A: Allocator,
    E: Extent,
{
    _marker: PhantomData<(I, D, A)>,
    memory: M,
    handle: vk::Image,
    offset: u64,
    size: u64,
    ident: A::Identifier,
    extent: E,
    format: vk::Format,
    samples: vk::SampleCountFlags,
    mip_levels: u32,
    array_layers: u32,
}

pub struct ImageView<I, D, M, Im, A, E> where
    I: Borrow<Instance> + Deref<Target = Instance>,
    D: Borrow<Device<I>> + Deref<Target = Device<I>>,
    M: Borrow<DeviceMemory<I, D, A>> + Deref<Target = DeviceMemory<I, D, A>>,
    Im: Borrow<Image<I, D, M, A, E>> + Deref<Target = Image<I, D, M, A, E>>,
    A: Allocator,
    E: Extent,
{
    _marker: PhantomData<(I, D, M, A, E)>,
    image: Im,
    handle: vk::ImageView,
    mip_range: Range<u32>,
    layer_range: Range<u32>,
}

impl<I, D, M, A, E> Image<I, D, M, A, E> where
    I: Borrow<Instance> + Deref<Target = Instance>,
    D: Borrow<Device<I>> + Deref<Target = Device<I>>,
    M: Borrow<DeviceMemory<I, D, A>> + Deref<Target = DeviceMemory<I, D, A>>,
    A: Allocator,
    E: Extent,
{
    pub fn new(
        memory: M,
        queue_families: &[u32],
        extent: E,
        format: vk::Format,
        samples: vk::SampleCountFlags,
        usage: vk::ImageUsageFlags,
        mip_levels: u32,
        array_layers: u32,
        initial_layout: vk::ImageLayout,
    ) -> Self {
        let sharing_mode = if queue_families.len() == 1 {
            vk::SharingMode::EXCLUSIVE
        } else {
            vk::SharingMode::CONCURRENT
        };
        let info = vk::ImageCreateInfo {
            s_type: StructureType::IMAGE_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::ImageCreateFlags::empty(),
            usage,
            format,
            image_type: E::image_type(),
            extent: extent.to_vk_extent_3d(),
            samples,
            mip_levels,
            array_layers,
            initial_layout,
            sharing_mode,
            tiling: vk::ImageTiling::OPTIMAL,
            queue_family_index_count: queue_families.len() as u32,
            p_queue_family_indices: queue_families.as_ptr(),
        };

        let device = &memory.device;
        let handle = unsafe { device.handle.create_image(&info, None).unwrap() };

        // TODO: Bind image to device memory and alloc from allocator in device memory.
        let requirements = unsafe { device.handle.get_image_memory_requirements(handle) };

        assert_ne!(1 << memory.type_index & requirements.memory_type_bits, 0);

        let layout = Layout::from_size_align(
            requirements.size as usize,
            requirements.alignment as usize,
        ).unwrap();

        let (offset, ident) = match memory.allocator.alloc(layout) {
            Ok(ok) => ok,
            Err(e) => {
                unsafe { device.handle.destroy_image(handle, None); }
                panic!("Can't allocate image to memory.");
            }
        };

        unsafe {
            device.handle
                .bind_image_memory(handle, memory.handle, offset)
                .unwrap()
        }

        Self {
            _marker: PhantomData,
            memory,
            handle,
            offset,
            size: requirements.size,
            ident,
            extent,
            format,
            samples,
            mip_levels,
            array_layers,
        }
    }

    #[inline]
    pub fn extent(&self) -> vk::Extent3D { self.extent.to_vk_extent_3d() }
    #[inline]
    pub fn memory(&self) -> &DeviceMemory<I, D, A> { &self.memory }
    #[inline]
    pub fn handle(&self) -> vk::Image { self.handle }
    #[inline]
    pub fn format(&self) -> vk::Format { self.format }
    #[inline]
    pub fn samples(&self) -> vk::SampleCountFlags { self.samples }
    #[inline]
    pub fn mip_levels(&self) -> u32 { self.mip_levels }
    #[inline]
    pub fn array_layers(&self) -> u32 { self.array_layers }
}

impl<I, D, M, A, E> Drop for Image<I, D, M, A, E> where
    I: Borrow<Instance> + Deref<Target = Instance>,
    D: Borrow<Device<I>> + Deref<Target = Device<I>>,
    M: Borrow<DeviceMemory<I, D, A>> + Deref<Target = DeviceMemory<I, D, A>>,
    A: Allocator,
    E: Extent,
{
    fn drop(&mut self) {
        unsafe { self.memory.device.handle.destroy_image(self.handle, None); }
        self.memory.allocator.dealloc(&self.ident);
    }
}

impl<I, D, M, Im, A, E> ImageView<I, D, M, Im, A, E> where
    I: Borrow<Instance> + Deref<Target = Instance>,
    D: Borrow<Device<I>> + Deref<Target = Device<I>>,
    M: Borrow<DeviceMemory<I, D, A>> + Deref<Target = DeviceMemory<I, D, A>>,
    Im: Borrow<Image<I, D, M, A, E>> + Deref<Target = Image<I, D, M, A, E>>,
    A: Allocator,
    E: Extent,
{
    pub fn new(
        image: Im,
        aspect: vk::ImageAspectFlags,
        mip_range: Range<u32>,
        layer_range: E::ArrayLayers,
    ) -> Self {
        // TODO: consider component mapping.

        let (base_array_layer, layer_count) = layer_range.base_layer_and_count();

        assert!(mip_range.end <= image.mip_levels);
        assert!(base_array_layer + layer_count <= image.array_layers);

        let info = vk::ImageViewCreateInfo {
            s_type: StructureType::IMAGE_VIEW_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::ImageViewCreateFlags::empty(),
            image: image.handle,
            view_type: layer_range.view_type(),
            format: image.format,
            components: vk::ComponentMapping {
                r: vk::ComponentSwizzle::IDENTITY,
                g: vk::ComponentSwizzle::IDENTITY,
                b: vk::ComponentSwizzle::IDENTITY,
                a: vk::ComponentSwizzle::IDENTITY,
            },
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask: aspect,
                base_mip_level: mip_range.start,
                level_count: mip_range.end - mip_range.start,
                base_array_layer,
                layer_count,
            },
        };

        let handle = unsafe {
            image.memory.device.handle.create_image_view(&info, None).unwrap()
        };

        Self {
            _marker: PhantomData,
            image,
            handle,
            mip_range,
            layer_range: layer_range.layer_range()
        }
    }

    #[inline]
    pub fn image(&self) -> &Image<I, D, M, A, E> { &self.image }
    #[inline]
    pub fn handle(&self) -> vk::ImageView { self.handle }
    #[inline]
    pub fn extent(&self) -> vk::Extent3D { self.image.extent.to_vk_extent_3d() }
    #[inline]
    pub fn format(&self) -> vk::Format { self.image.format }
    #[inline]
    pub fn samples(&self) -> vk::SampleCountFlags { self.image.samples }
    #[inline]
    pub fn mip_range(&self) -> &Range<u32> { &self.mip_range }
    #[inline]
    pub fn layer_range(&self) -> &Range<u32> { &self.layer_range }
}

impl<I, D, M, Im, A, E> Drop for ImageView<I, D, M, Im, A, E> where
    I: Borrow<Instance> + Deref<Target = Instance>,
    D: Borrow<Device<I>> + Deref<Target = Device<I>>,
    M: Borrow<DeviceMemory<I, D, A>> + Deref<Target = DeviceMemory<I, D, A>>,
    Im: Borrow<Image<I, D, M, A, E>> + Deref<Target = Image<I, D, M, A, E>>,
    A: Allocator,
    E: Extent,
{
    fn drop(&mut self) {
        unsafe { self.image.memory.device.handle.destroy_image_view(self.handle, None); }
    }
}


mod usage {
    use ash::vk;

    pub struct ImageUsage {
        flags: vk::ImageUsageFlags,
    }

    impl ImageUsage {
        pub fn builder() -> Self { ImageUsage { flags: vk::ImageUsageFlags::empty() } }
        pub fn transfer_src(self) -> Self {
            ImageUsage {
                flags: self.flags | vk::ImageUsageFlags::TRANSFER_SRC,
            }
        }
        pub fn transfer_dst(self) -> Self {
            ImageUsage {
                flags: self.flags | vk::ImageUsageFlags::TRANSFER_DST,
            }
        }
        pub fn sampled(self) -> Self {
            ImageUsage {
                flags: self.flags | vk::ImageUsageFlags::SAMPLED,
            }
        }
        pub fn storage(self) -> Self {
            ImageUsage {
                flags: self.flags | vk::ImageUsageFlags::STORAGE,
            }
        }
        pub fn color_attachment(self) -> Self {
            ImageUsage {
                flags: self.flags | vk::ImageUsageFlags::COLOR_ATTACHMENT,
            }
        }
        pub fn depth_stencil_attachment(self) -> Self {
            ImageUsage {
                flags: self.flags | vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
            }
        }
        pub fn transient_attachment(self) -> Self {
            ImageUsage {
                flags: self.flags | vk::ImageUsageFlags::TRANSIENT_ATTACHMENT,
            }
        }
        pub fn input_attachment(self) -> Self {
            ImageUsage {
                flags: self.flags | vk::ImageUsageFlags::INPUT_ATTACHMENT,
            }
        }
        pub fn shading_rate_image_nv(self) -> Self {
            ImageUsage {
                flags: self.flags | vk::ImageUsageFlags::SHADING_RATE_IMAGE_NV,
            }
        }
        pub fn fragment_density_map_ext(self) -> Self {
            ImageUsage {
                flags: self.flags | vk::ImageUsageFlags::FRAGMENT_DENSITY_MAP_EXT,
            }
        }
        pub fn build(self) -> vk::ImageUsageFlags { self.flags }
    }
}