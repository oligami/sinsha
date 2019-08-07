pub mod usage;
pub mod extent;

pub use usage::{ ImageUsages, ImageUsage };
pub use extent::*;

use super::*;

use std::ops;

pub struct Image<I, D, M, A, P, E, U>
    where I: Borrow<Instance> + Deref<Target = Instance>,
          D: Borrow<Device<I>> + Deref<Target = Device<I>>,
          M: Borrow<DeviceMemory<I, D, A, P>> + Deref<Target = DeviceMemory<I, D, A, P>>,
          A: Allocator,
          P: MemoryProperty,
          E: Extent,
          U: ImageUsage,
{
    _marker: PhantomData<(I, D, A, P, U)>,
    memory: M,
    handle: vk::Image,
    range: ops::Range<u64>,
    ident: A::Identifier,
    extent: E,
    format: vk::Format,
    samples: vk::SampleCountFlags,
    mip_levels: u32,
    array_layers: u32,
}

pub trait ImageAbs {
    type Instance: Borrow<Instance> + Deref<Target = Instance>;
    type Device: Borrow<Device<Self::Instance>> + Deref<Target = Device<Self::Instance>>;
    type Memory: Borrow<DeviceMemory<
        Self::Instance,
        Self::Device,
        Self::Allocator,
        Self::MemoryProperty
    >> + Deref<Target = DeviceMemory<
        Self::Instance,
        Self::Device,
        Self::Allocator,
        Self::MemoryProperty
    >>;
    type Allocator: Allocator;
    type MemoryProperty: MemoryProperty;
    type Extent: Extent;
    type Usage: ImageUsage;

    fn instance(&self) -> &Instance;
    fn device(&self) -> &Device<Self::Instance>;
    fn memory(&self) -> &DeviceMemory<Self::Instance, Self::Device, Self::Allocator, Self::MemoryProperty>;
    fn handle(&self) -> vk::Image;
    fn extent(&self) -> &Self::Extent;
    fn format(&self) -> vk::Format;
    fn samples(&self) -> vk::SampleCountFlags;
    fn mip_levels(&self) -> u32;
    fn array_layers(&self) -> u32;
}


pub struct ImageView<I, D, M, Im, A, P, E, U>
    where I: Borrow<Instance> + Deref<Target = Instance>,
          D: Borrow<Device<I>> + Deref<Target = Device<I>>,
          M: Borrow<DeviceMemory<I, D, A, P>> + Deref<Target = DeviceMemory<I, D, A, P>>,
          Im: Borrow<Image<I, D, M, A, P, E, U>> + Deref<Target = Image<I, D, M, A, P, E, U>>,
          A: Allocator,
          P: MemoryProperty,
          E: Extent,
          U: ImageUsage,
{
    _marker: PhantomData<(I, D, M, A, P, E, U)>,
    image: Im,
    handle: vk::ImageView,
    mip_range: ops::Range<u32>,
    layer_range: ops::Range<u32>,
}

pub trait ImageViewAbs {
    type Instance: Borrow<Instance> + Deref<Target = Instance>;
    type Device: Borrow<Device<Self::Instance>> + Deref<Target = Device<Self::Instance>>;
    type Memory: Borrow<DeviceMemory<
        Self::Instance,
        Self::Device,
        Self::Allocator,
        Self::MemoryProperty
    >> + Deref<Target = DeviceMemory<
        Self::Instance,
        Self::Device,
        Self::Allocator,
        Self::MemoryProperty
    >>;
    type Image: Borrow<Image<
        Self::Instance,
        Self::Device,
        Self::Memory,
        Self::Allocator,
        Self::MemoryProperty,
        Self::Extent,
        Self::Usage,
    >> + Deref<Target = Image<
        Self::Instance,
        Self::Device,
        Self::Memory,
        Self::Allocator,
        Self::MemoryProperty,
        Self::Extent,
        Self::Usage,
    >>;
    type Allocator: Allocator;
    type MemoryProperty: MemoryProperty;
    type Extent: Extent;
    type Usage: ImageUsage;

    fn instance(&self) -> &Instance;
    fn device(&self) -> &Device<Self::Instance>;
    fn memory(&self) -> &DeviceMemory<
        Self::Instance,
        Self::Device,
        Self::Allocator,
        Self::MemoryProperty
    >;
    fn image(&self) -> &Image<
        Self::Instance,
        Self::Device,
        Self::Memory,
        Self::Allocator,
        Self::MemoryProperty,
        Self::Extent,
        Self::Usage,
    >;
    fn handle(&self) -> vk::ImageView;
    fn extent(&self) -> &Self::Extent;
    fn format(&self) -> vk::Format;
    fn samples(&self) -> vk::SampleCountFlags;
    fn mip_range(&self) -> &ops::Range<u32>;
    fn layer_range(&self) -> &ops::Range<u32>;
}

impl<I, D, M, A, P, E, U> Image<I, D, M, A, P, E, U>
    where I: Borrow<Instance> + Deref<Target = Instance>,
          D: Borrow<Device<I>> + Deref<Target = Device<I>>,
          M: Borrow<DeviceMemory<I, D, A, P>> + Deref<Target = DeviceMemory<I, D, A, P>>,
          A: Allocator,
          P: MemoryProperty,
          E: Extent,
          U: ImageUsage,
{
    pub fn new(
        memory: M,
        queue_families: &[u32],
        extent: E,
        format: vk::Format,
        samples: vk::SampleCountFlags,
        _usage: U,
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
            usage: U::image_usage(),
            format,
            image_type: E::image_type(),
            extent: extent.extent(),
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
        println!("{:?}", requirements);

        let (range, ident) = match memory.allocator.lock().unwrap().alloc(layout) {
            Ok(range_and_ident) => range_and_ident,
            Err(e) => {
                unsafe { device.handle.destroy_image(handle, None); }
                panic!("Can't allocate image to memory.");
            }
        };

        unsafe {
            device.handle
                .bind_image_memory(handle, memory.handle, range.start)
                .unwrap()
        }

        Self {
            _marker: PhantomData,
            memory,
            handle,
            range,
            ident,
            extent,
            format,
            samples,
            mip_levels,
            array_layers,
        }
    }

    #[inline]
    pub fn extent(&self) -> &E { &self.extent }
}

impl<I, D, M, A, P, E, U> ImageAbs for Image<I, D, M, A, P, E, U>
    where I: Borrow<Instance> + Deref<Target = Instance>,
          D: Borrow<Device<I>> + Deref<Target = Device<I>>,
          M: Borrow<DeviceMemory<I, D, A, P>> + Deref<Target = DeviceMemory<I, D, A, P>>,
          A: Allocator,
          P: MemoryProperty,
          E: Extent,
          U: ImageUsage,
{
    type Instance = I;
    type Device = D;
    type Memory = M;
    type Allocator = A;
    type MemoryProperty = P;
    type Extent = E;
    type Usage = U;

    #[inline]
    fn instance(&self) -> &Instance { &self.memory.device.instance }
    #[inline]
    fn device(&self) -> &Device<Self::Instance> { &self.memory.device }
    #[inline]
    fn memory(&self) -> &DeviceMemory<
        Self::Instance,
        Self::Device,
        Self::Allocator,
        Self::MemoryProperty
    > { &self.memory }
    #[inline]
    fn handle(&self) -> vk::Image { self.handle }
    #[inline]
    fn extent(&self) -> &Self::Extent { &self.extent }
    #[inline]
    fn format(&self) -> vk::Format { self.format }
    #[inline]
    fn samples(&self) -> vk::SampleCountFlags { self.samples }
    #[inline]
    fn mip_levels(&self) -> u32 { self.mip_levels }
    #[inline]
    fn array_layers(&self) -> u32 { self.array_layers }
}

impl<I, D, M, A, P, E, U> Drop for Image<I, D, M, A, P, E, U>
    where I: Borrow<Instance> + Deref<Target = Instance>,
          D: Borrow<Device<I>> + Deref<Target = Device<I>>,
          M: Borrow<DeviceMemory<I, D, A, P>> + Deref<Target = DeviceMemory<I, D, A, P>>,
          A: Allocator,
          P: MemoryProperty,
          E: Extent,
          U: ImageUsage,
{
    fn drop(&mut self) {
        unsafe { self.device().handle.destroy_image(self.handle, None); }
        self.memory().allocator.lock().unwrap().dealloc(&self.ident);
    }
}

impl<I, D, M, Im, A, P, E, U> ImageView<I, D, M, Im, A, P, E, U>
    where I: Borrow<Instance> + Deref<Target = Instance>,
          D: Borrow<Device<I>> + Deref<Target = Device<I>>,
          M: Borrow<DeviceMemory<I, D, A, P>> + Deref<Target = DeviceMemory<I, D, A, P>>,
          Im: Borrow<Image<I, D, M, A, P, E, U>> + Deref<Target = Image<I, D, M, A, P, E, U>>,
          A: Allocator,
          P: MemoryProperty,
          E: Extent,
          U: ImageUsage,
{
    pub fn new(
        image: Im,
        aspect: vk::ImageAspectFlags,
        mip_range: ops::Range<u32>,
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
}

impl<I, D, M, Im, A, P, E, U> ImageViewAbs for ImageView<I, D, M, Im, A, P, E, U>
    where I: Borrow<Instance> + Deref<Target = Instance>,
          D: Borrow<Device<I>> + Deref<Target = Device<I>>,
          M: Borrow<DeviceMemory<I, D, A, P>> + Deref<Target = DeviceMemory<I, D, A, P>>,
          Im: Borrow<Image<I, D, M, A, P, E, U>> + Deref<Target = Image<I, D, M, A, P, E, U>>,
          A: Allocator,
          P: MemoryProperty,
          E: Extent,
          U: ImageUsage,
{
    type Instance = I;
    type Device = D;
    type Memory = M;
    type Image = Im;
    type Allocator = A;
    type MemoryProperty = P;
    type Extent = E;
    type Usage = U;

    #[inline]
    fn instance(&self) -> &Instance { &self.image.memory.device.instance }
    #[inline]
    fn device(&self) -> &Device<Self::Instance> { &self.image.memory.device }
    #[inline]
    fn memory(&self) -> &DeviceMemory<
        Self::Instance,
        Self::Device,
        Self::Allocator,
        Self::MemoryProperty
    > { &self.image.memory }
    #[inline]
    fn image(&self) -> &Image<
        Self::Instance,
        Self::Device,
        Self::Memory,
        Self::Allocator,
        Self::MemoryProperty,
        Self::Extent,
        Self::Usage,
    > { &self.image }
    #[inline]
    fn handle(&self) -> vk::ImageView { self.handle }
    #[inline]
    fn extent(&self) -> &Self::Extent { &self.image.extent }
    #[inline]
    fn format(&self) -> vk::Format { self.image.format }
    #[inline]
    fn samples(&self) -> vk::SampleCountFlags { self.image.samples }
    #[inline]
    fn mip_range(&self) -> &ops::Range<u32> { &self.mip_range }
    #[inline]
    fn layer_range(&self) -> &ops::Range<u32> { &self.layer_range }
}

impl<I, D, M, Im, A, P, E, U> Drop for ImageView<I, D, M, Im, A, P, E, U>
    where I: Borrow<Instance> + Deref<Target = Instance>,
          D: Borrow<Device<I>> + Deref<Target = Device<I>>,
          M: Borrow<DeviceMemory<I, D, A, P>> + Deref<Target = DeviceMemory<I, D, A, P>>,
          Im: Borrow<Image<I, D, M, A, P, E, U>> + Deref<Target = Image<I, D, M, A, P, E, U>>,
          A: Allocator,
          P: MemoryProperty,
          E: Extent,
          U: ImageUsage,
{
    fn drop(&mut self) {
        unsafe { self.image.memory.device.handle.destroy_image_view(self.handle, None); }
    }
}