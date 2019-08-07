pub mod ty;


use super::*;
use device_memory::buffer::DataAbs;
use device_memory::buffer::usage;
use device_memory::image::ImageAbs;

pub use ty::DescriptorType;

use ty::{ Buffer, Image, BufferView, PNext };

pub struct DescriptorSetLayout<I, D> where
    I: Borrow<Instance> + Deref<Target = Instance>,
    D: Borrow<Device<I>> + Deref<Target = Device<I>>,
{
    _marker: PhantomData<I>,
    device: D,
    handle: vk::DescriptorSetLayout,
    bindings: Vec<Binding>,
}

struct Binding {
    ty: vk::DescriptorType,
    count: u32,
}

pub struct DescriptorPool<I, D> where
    I: Borrow<Instance> + Deref<Target = Instance>,
    D: Borrow<Device<I>> + Deref<Target = Device<I>>,
{
    _marker: PhantomData<I>,
    device: D,
    handle: vk::DescriptorPool,
}

pub struct DescriptorSet<I, D, L, P, R> where
    I: Borrow<Instance> + Deref<Target = Instance>,
    D: Borrow<Device<I>> + Deref<Target = Device<I>>,
    L: Borrow<DescriptorSetLayout<I, D>> + Deref<Target = DescriptorSetLayout<I, D>>,
    P: Borrow<DescriptorPool<I, D>> + Deref<Target = DescriptorPool<I, D>>,
{
    _marker: PhantomData<(I, D)>,
    layout: L,
    pool: P,
    handle: vk::DescriptorSet,
    resources: R,
}

pub struct DescriptorSetUpdate<I, D, L, P, R, R1> where
    I: Borrow<Instance> + Deref<Target = Instance>,
    D: Borrow<Device<I>> + Deref<Target = Device<I>>,
    L: Borrow<DescriptorSetLayout<I, D>> + Deref<Target = DescriptorSetLayout<I, D>>,
    P: Borrow<DescriptorPool<I, D>> + Deref<Target = DescriptorPool<I, D>>,
{
    _marker: PhantomData<(I, D, L, P, R)>,
    set: DescriptorSet<I, D, L, P, R>,
    writes: Vec<vk::WriteDescriptorSet>,
    copies: Vec<vk::CopyDescriptorSet>,
    buffer_infos: Vec<Vec<vk::DescriptorBufferInfo>>,
    image_infos: Vec<Vec<vk::DescriptorImageInfo>>,
    buffer_view: Vec<vk::BufferView>,
    resources: R1,
}

pub struct DataInfos<R> {
    infos: Vec<vk::DescriptorBufferInfo>,
    resources: R,
}

impl<I, D> DescriptorSetLayout<I, D> where
    I: Borrow<Instance> + Deref<Target = Instance>,
    D: Borrow<Device<I>> + Deref<Target = Device<I>>,
{
    pub fn new(device: D, bindings: &[vk::DescriptorSetLayoutBinding]) -> Self {
        let info = vk::DescriptorSetLayoutCreateInfo::builder().bindings(bindings);
        let handle = unsafe { device.handle.create_descriptor_set_layout(&info, None).unwrap() };
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

impl<I, D> Drop for  DescriptorSetLayout<I, D> where
    I: Borrow<Instance> + Deref<Target = Instance>,
    D: Borrow<Device<I>> + Deref<Target = Device<I>>,
{
    fn drop(&mut self) {
        unsafe { self.device.handle.destroy_descriptor_set_layout(self.handle, None); }
    }
}

impl<I, D> DescriptorPool<I, D> where
    I: Borrow<Instance> + Deref<Target = Instance>,
    D: Borrow<Device<I>> + Deref<Target = Device<I>>,
{
    pub fn from_layouts<L>(layouts: &[L], max_sets: u32) -> Self where
        L: Borrow<DescriptorSetLayout<I, D>> + Deref<Target = DescriptorSetLayout<I, D>>,
        D: Clone,
    {
        let mut pool_sizes = Vec::new();
        layouts.iter()
            .for_each(|layout| {
                pool_sizes.reserve(layout.bindings.len());
                layout.bindings.iter()
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

        let device = layouts[0].device.clone();
        let handle = unsafe { device.handle.create_descriptor_pool(&info, None).unwrap() };

        Self { _marker: PhantomData, device, handle }
    }
}

impl<I, D> Drop for DescriptorPool<I, D> where
    I: Borrow<Instance> + Deref<Target = Instance>,
    D: Borrow<Device<I>> + Deref<Target = Device<I>>,
{
    fn drop(&mut self) { unsafe { self.device.handle.destroy_descriptor_pool(self.handle, None); } }
}

impl<I, D, L, P> DescriptorSet<I, D, L, P, ()> where
    I: Borrow<Instance> + Deref<Target = Instance>,
    D: Borrow<Device<I>> + Deref<Target = Device<I>>,
    L: Borrow<DescriptorSetLayout<I, D>> + Deref<Target = DescriptorSetLayout<I, D>>,
    P: Borrow<DescriptorPool<I, D>> + Deref<Target = DescriptorPool<I, D>>,
{
    pub fn new(layouts: &[L], pool: P) -> Vec<Self> where L: Clone, P: Clone {
        let layout_handles: Vec<_> = layouts.iter()
            .map(|l| l.handle)
            .collect();

        let info = vk::DescriptorSetAllocateInfo::builder()
            .set_layouts(&layout_handles[..])
            .descriptor_pool(pool.handle);

        let handles = unsafe { layouts[0].device.handle.allocate_descriptor_sets(&info).unwrap() };

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
}
impl<I, D, L, P, R> DescriptorSet<I, D, L, P, R> where
    I: Borrow<Instance> + Deref<Target = Instance>,
    D: Borrow<Device<I>> + Deref<Target = Device<I>>,
    L: Borrow<DescriptorSetLayout<I, D>> + Deref<Target = DescriptorSetLayout<I, D>>,
    P: Borrow<DescriptorPool<I, D>> + Deref<Target = DescriptorPool<I, D>>,
{
    pub fn updater(self) -> DescriptorSetUpdate<I, D, L, P, R, ()> {
        DescriptorSetUpdate {
            _marker: PhantomData,
            set: self,
            writes: Vec::new(),
            copies: Vec::new(),
            buffer_infos: Vec::new(),
            image_infos: Vec::new(),
            buffer_view: Vec::new(),
            resources: (),
        }
    }
}

impl<I, D, L, P, R, R1> DescriptorSetUpdate<I, D, L, P, R, R1> where
    I: Borrow<Instance> + Deref<Target = Instance>,
    D: Borrow<Device<I>> + Deref<Target = Device<I>>,
    L: Borrow<DescriptorSetLayout<I, D>> + Deref<Target = DescriptorSetLayout<I, D>> + Clone,
    P: Borrow<DescriptorPool<I, D>> + Deref<Target = DescriptorPool<I, D>> + Clone,
{
    pub fn write_data<R2>(mut self, binding: u32, array_element: u32, data_infos: DataInfos<R2>)
        -> DescriptorSetUpdate<I, D, L, P, R, (R1, R2)> where
    {
        let write = vk::WriteDescriptorSet::builder()
            .dst_set(self.set.handle)
            .dst_binding(binding)
            .dst_array_element(array_element)
            .descriptor_type(self.set.layout.bindings[binding as usize].ty)
            .buffer_info(&data_infos.infos[..])
            .build();

        self.writes.push(write);
        self.buffer_infos.push(data_infos.infos);

        DescriptorSetUpdate {
            _marker: PhantomData,
            set: self.set,
            writes: self.writes,
            copies: self.copies,
            buffer_infos: self.buffer_infos,
            image_infos: self.image_infos,
            buffer_view: self.buffer_view,
            resources: (self.resources, data_infos.resources),
        }
    }

    pub fn update(self) -> DescriptorSet<I, D, L, P, R1> {
        unsafe {
            self.set.layout.device.handle.update_descriptor_sets(&self.writes[..], &self.copies[..])
        }

        DescriptorSet {
            _marker: PhantomData,
            layout: self.set.layout.clone(),
            pool: self.set.pool.clone(),
            handle: self.set.handle,
            resources: self.resources,
        }
    }
}

impl DataInfos<()>{
    pub fn new() -> Self { DataInfos { infos: Vec::new(), resources: () } }
}
impl<R> DataInfos<R> {
    pub fn add_data<Ref, Data>(mut self, data: Ref) -> DataInfos<(R, Ref)> where
        Ref: Borrow<Data> + Deref<Target =Data>,
        Data: DataAbs,
        Data::Usage: usage::UniformBuffer,
    {
        let info = vk::DescriptorBufferInfo::builder()
            .buffer(data.handle())
            .offset(data.offset())
            .range(data.size())
            .build();
        self.infos.push(info);

        DataInfos {
            infos: self.infos,
            resources: (self.resources, data),
        }
    }
}
