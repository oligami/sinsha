use super::*;
use device_memory::{ DeviceMemory, alloc::Allocator };
use buffer::{ Buffer, Data };
use image::ImageView;

pub struct DescriptorSetLayout<I, D> where D: Borrow<Device<I>> {
    _marker: PhantomData<I>,
    device: D,
    handle: vk::DescriptorSetLayout,
    bindings: Vec<Binding>,
}

pub struct DescriptorSetLayoutCreateInfo<'a> {
    info: vk::DescriptorSetLayoutCreateInfo,
    bindings: &'a [vk::DescriptorSetLayoutBindingBuilder<'a>],
}

#[derive(Copy, Clone, Debug)]
pub struct DescriptorSetLayoutBinding<S> {
    vk: vk::DescriptorSetLayoutBinding,
    samplers: S,
}

struct Binding {
    ty: vk::DescriptorType,
    count: u32,
}

pub struct DescriptorPool<I, D> where D: Borrow<Device<I>> {
    _marker: PhantomData<I>,
    device: D,
    handle: vk::DescriptorPool,
}

pub struct DescriptorSet<I, D, L, P, R> where
    D: Borrow<Device<I>>,
    L: Borrow<DescriptorSetLayout<I, D>>,
    P: Borrow<DescriptorPool<I, D>>,
{
    _marker: PhantomData<(I, D)>,
    layout: L,
    pool: P,
    handle: vk::DescriptorSet,
    resources: R,
}

pub struct BindResources<'b, R> {
    binding: &'b [Binding],
    write: vk::WriteDescriptorSet,
    resources: R,
}

impl<'a> DescriptorSetLayoutCreateInfo<'a> {
    pub unsafe fn create<I, D>(&self, device: D) -> DescriptorSetLayout<I, D> where
        I: Borrow<Instance>,
        D: Borrow<Device<I>>,
    {
        let handle = device.borrow().handle.create_descriptor_set_layout(&self.info, None).unwrap();
        DescriptorSetLayout {
            _marker: PhantomData,
            device,
            handle,
            bindings: unimplemented!(),
        }
    }
}

impl<I, D> DescriptorSetLayout<I, D> where
    I: Borrow<Instance>,
    D: Borrow<Device<I>>,
{
    pub fn new(device: D, bindings: &[vk::DescriptorSetLayoutBinding]) -> Self {
        let info = vk::DescriptorSetLayoutCreateInfo::builder().bindings(bindings);
        let handle = unsafe { device.borrow().handle.create_descriptor_set_layout(&info, None).unwrap() };
        // TODO: enable this
//        assert!(bindings.iter().is_sorted_by_key(|binding| binding.binding));
        let bindings = bindings.iter()
            .fold(Vec::with_capacity(bindings.len()), |mut bindings, binding| {
                while (bindings.len() as u32) < binding.binding {
                    bindings.push(Binding {
                        ty: vk::DescriptorType::SAMPLER,
                        count: 0,
                    });
                }
                bindings.push(Binding { ty: binding.descriptor_type, count: binding.descriptor_count});

                bindings
            });

        Self { _marker: PhantomData, device, handle, bindings }
    }
}
impl<I, D> DescriptorSetLayout<I, D> where D: Borrow<Device<I>>
{
    #[inline]
    pub fn handle(&self) -> vk::DescriptorSetLayout { self.handle }
}

impl<I, D> Drop for  DescriptorSetLayout<I, D> where D: Borrow<Device<I>> {
    fn drop(&mut self) {
        unsafe { self.device.borrow().handle.destroy_descriptor_set_layout(self.handle, None); }
    }
}

impl<I, D> DescriptorPool<I, D> where
    I: Borrow<Instance>,
    D: Borrow<Device<I>>,
{
    pub fn from_layouts<L>(layouts: &[L], max_sets: u32) -> Self where
        L: Borrow<DescriptorSetLayout<I, D>>,
        D: Clone,
    {
        let mut pool_sizes = Vec::new();
        layouts.iter()
            .for_each(|layout| {
                pool_sizes.reserve(layout.borrow().bindings.len());
                layout.borrow().bindings.iter()
                    .for_each(|binding| {
                        let pool_size = vk::DescriptorPoolSize::builder()
                            .ty(binding.ty)
                            .descriptor_count(binding.count)
                            .build();

                        pool_sizes.push(pool_size);
                    });
            });

        let info = vk::DescriptorPoolCreateInfo::builder()
            .pool_sizes(&pool_sizes[..])
            .max_sets(max_sets);

        let device = layouts[0].borrow().device.clone();
        let handle = unsafe { device.borrow().handle.create_descriptor_pool(&info, None).unwrap() };

        Self { _marker: PhantomData, device, handle }
    }
}

impl<I, D> Drop for DescriptorPool<I, D> where D: Borrow<Device<I>> {
    fn drop(&mut self) {
        unsafe { self.device.borrow().handle.destroy_descriptor_pool(self.handle, None); }
    }
}

impl<I, D, L, P> DescriptorSet<I, D, L, P, ()> where
    I: Borrow<Instance>,
    D: Borrow<Device<I>>,
    L: Borrow<DescriptorSetLayout<I, D>>,
    P: Borrow<DescriptorPool<I, D>>,
{
    pub fn new(layouts: &[L], pool: P) -> Vec<Self> where L: Clone, P: Clone {
        let layout_handles: Vec<_> = layouts.iter()
            .map(|l| l.borrow().handle)
            .collect();

        let info = vk::DescriptorSetAllocateInfo::builder()
            .set_layouts(&layout_handles[..])
            .descriptor_pool(pool.borrow().handle);

        let handles = unsafe { layouts[0].borrow().device.borrow().handle.allocate_descriptor_sets(&info).unwrap() };

        handles.into_iter()
            .zip(layouts.iter())
            .map(|(handle, layout)| {
                Self {
                    _marker: PhantomData,
                    handle,
                    layout: layout.clone(),
                    pool: pool.clone(),
                    resources: (),
                }
            })
            .collect()
    }

    #[inline]
    pub fn layout(&self) -> &DescriptorSetLayout<I, D> { &self.layout.borrow() }
}


impl<'b, R> BindResources<'b, R> {
    pub fn add_data<I, D, M, B, Da, BA, DA, T>(mut self, data: Da) -> BindResources<'b, (R, Da)> where
        D: Borrow<Device<I>>,
        M: Borrow<DeviceMemory<I, D, BA>>,
        B: Borrow<Buffer<I, D, M, BA, DA>>,
        Da: Borrow<Data<I, D, M, B, BA, DA, T>>,
        BA: Allocator,
        DA: Allocator,
    {
        let info = vk::WriteDescriptorSet::builder()
            .dst_set(unimplemented!());

        unimplemented!()
    }
}
