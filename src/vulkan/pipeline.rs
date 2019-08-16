pub mod stage;

use super::*;
use descriptor::DescriptorSetLayout;
use render_pass::RenderPass;

use std::ops::Range;

pub struct PipelineLayout<I, D, S> where
    I: Borrow<Instance> + Deref<Target = Instance>,
    D: Borrow<Device<I>> + Deref<Target = Device<I>>,
    S: Borrow<DescriptorSetLayout<I, D>> + Deref<Target = DescriptorSetLayout<I, D>>,
{
    device: D,
    set_layouts: Vec<S>,
    handle: vk::PipelineLayout,
    push_constants: Vec<vk::PushConstantRange>,
}

pub struct PipelineLayoutBuilder<S> {
    set_layouts: S,
    count: u32,
}

pub struct ComputePipeline;

pub struct GraphicsPipeline<I, D, R, S, L> where
    I: Borrow<Instance> + Deref<Target = Instance>,
    D: Borrow<Device<I>> + Deref<Target = Device<I>>,
    R: Borrow<RenderPass<I, D>> + Deref<Target = RenderPass<I, D>>,
    S: Borrow<DescriptorSetLayout<I, D>> + Deref<Target = DescriptorSetLayout<I, D>>,
    L: Borrow<PipelineLayout<I, D, S>> + Deref<Target = PipelineLayout<I, D, S>>,
{
    _marker: PhantomData<(I, D, S)>,
    render_pass: R,
    layout: L,
    handle: vk::Pipeline,
}

pub struct GraphicsPipelineBuilder<'a> {
    info: vk::GraphicsPipelineCreateInfoBuilder<'a>,
}

pub struct ShaderStages<I, D> where
    I: Borrow<Instance> + Deref<Target = Instance>,
    D: Borrow<Device<I>> + Deref<Target = Device<I>>,
{
    _marker: PhantomData<I>,
    device: D,
    shader_stages: Vec<vk::PipelineShaderStageCreateInfo>,
    shader_stage_holders: Vec<ShaderStageHolder>,
}

struct ShaderStageHolder {
    module: vk::ShaderModule,
    invoke_fn_name: CString,
    specialization_info: Option<Box<vk::SpecializationInfo>>,
    data_and_maps: Option<Specializations>,
}

pub struct Specializations {
    maps: Vec<vk::SpecializationMapEntry>,
    data: Vec<u8>,
}

pub trait Vertex {
    fn attributes() -> &'static [vk::VertexInputAttributeDescription];
    fn bindings() -> &'static [vk::VertexInputBindingDescription];
}

#[repr(C)]
pub struct VertexTest {
    position: [f32; 3],
}

impl<I, D, S> PipelineLayout<I, D, S> where
    I: Borrow<Instance> + Deref<Target = Instance>,
    D: Borrow<Device<I>> + Deref<Target = Device<I>>,
    S: Borrow<DescriptorSetLayout<I, D>> + Deref<Target = DescriptorSetLayout<I, D>>,
{
    pub fn new(
        device: D,
        set_layouts: Vec<S>,
        push_constants: Vec<vk::PushConstantRange>,
    ) -> Self {
        let layout_handles = set_layouts.iter()
            .map(|layout| layout.handle)
            .collect::<Vec<_>>();
        let info = vk::PipelineLayoutCreateInfo::builder()
            .push_constant_ranges(&push_constants[..])
            .set_layouts(&layout_handles[..]);

        let handle = unsafe {
            device.handle.create_pipeline_layout(&info, None).unwrap()
        };

        Self { device, handle, set_layouts, push_constants }
    }
}


impl<I, D, S> Drop for PipelineLayout<I, D, S> where
    I: Borrow<Instance> + Deref<Target = Instance>,
    D: Borrow<Device<I>> + Deref<Target = Device<I>>,
    S: Borrow<DescriptorSetLayout<I, D>> + Deref<Target = DescriptorSetLayout<I, D>>,
{
    fn drop(&mut self) {
        unsafe { self.device.handle.destroy_pipeline_layout(self.handle, None); }
    }
}

impl<I, D, R, S, L> GraphicsPipeline<I, D, R, S, L> where
    I: Borrow<Instance> + Deref<Target = Instance>,
    D: Borrow<Device<I>> + Deref<Target = Device<I>>,
    R: Borrow<RenderPass<I, D>> + Deref<Target = RenderPass<I, D>>,
    S: Borrow<DescriptorSetLayout<I, D>> + Deref<Target = DescriptorSetLayout<I, D>>,
    L: Borrow<PipelineLayout<I, D, S>> + Deref<Target = PipelineLayout<I, D, S>>,
{
    pub unsafe fn new(
        mut info: vk::GraphicsPipelineCreateInfo,
        shader_stages: ShaderStages<I, D>,
        render_pass: R,
        subpass: u32,
        layout: L,
        cache: Option<vk::PipelineCache>,
    ) -> Self {
        info.stage_count = shader_stages.shader_stages.len() as u32;
        info.p_stages = shader_stages.shader_stages.as_ptr();
        info.render_pass = render_pass.handle();
        info.subpass = subpass;
        info.layout = layout.handle;

        let handle = unsafe {
            render_pass.device().handle
                .create_graphics_pipelines(vk::PipelineCache::null(), &[info], None)
                .unwrap()[0]
        };

        GraphicsPipeline { _marker: PhantomData, render_pass, layout, handle }
    }

    #[inline]
    pub fn handle(&self) -> vk::Pipeline { self.handle }
}

impl<I, D, R, S, L> Drop for GraphicsPipeline<I, D, R, S, L> where
    I: Borrow<Instance> + Deref<Target = Instance>,
    D: Borrow<Device<I>> + Deref<Target = Device<I>>,
    R: Borrow<RenderPass<I, D>> + Deref<Target = RenderPass<I, D>>,
    S: Borrow<DescriptorSetLayout<I, D>> + Deref<Target = DescriptorSetLayout<I, D>>,
    L: Borrow<PipelineLayout<I, D, S>> + Deref<Target = PipelineLayout<I, D, S>>,
{
    #[inline]
    fn drop(&mut self) {
        unsafe { self.layout.device.handle.destroy_pipeline(self.handle, None); }
    }
}

impl<I, D> ShaderStages<I, D> where
    I: Borrow<Instance> + Deref<Target = Instance>,
    D: Borrow<Device<I>> + Deref<Target = Device<I>>,
{
    pub fn new(device: D) -> Self {
        ShaderStages {
            _marker: PhantomData,
            device,
            shader_stages: Vec::new(),
            shader_stage_holders: Vec::new(),
        }
    }

    pub fn shader_stage<Path, N>(
        mut self,
        path: Path,
        invoke_fn_name: N,
        stage_flag: vk::ShaderStageFlags,
        specializations: Option<Specializations>,
    ) -> Self where Path: AsRef<std::path::Path>, N: Into<Vec<u8>> {
        let module = {
            use std::io::Read;

            let mut spv = std::fs::File::open(path).unwrap();
            let mut buf = Vec::new();
            spv.read_to_end(&mut buf).unwrap();

            let info = vk::ShaderModuleCreateInfo {
                s_type: StructureType::SHADER_MODULE_CREATE_INFO,
                p_next: ptr::null(),
                flags: vk::ShaderModuleCreateFlags::empty(),
                code_size: buf.len(),
                p_code: buf.as_ptr() as _,
            };

            unsafe { self.device.handle.create_shader_module(&info, None).unwrap() }
        };

        let invoke_fn_name = CString::new(invoke_fn_name).unwrap();

        let mut holder = ShaderStageHolder {
            module,
            invoke_fn_name,
            specialization_info: None,
            data_and_maps: None
        };

        let p_specialization_info = specializations
            .map(|s| {
                let info = vk::SpecializationInfo {
                    map_entry_count: s.maps.len() as u32,
                    p_map_entries: s.maps.as_ptr(),
                    data_size: s.data.len(),
                    p_data: s.data.as_ptr() as _,
                };

                let info = Box::new(info);
                let ptr = info.as_ref() as *const _;
                holder.specialization_info = Some(info);
                holder.data_and_maps = Some(s);

                ptr
            })
            .unwrap_or(ptr::null());

        let stage_info = vk::PipelineShaderStageCreateInfo {
            s_type: StructureType::PIPELINE_SHADER_STAGE_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::PipelineShaderStageCreateFlags::empty(),
            stage: stage_flag,
            module,
            p_name: holder.invoke_fn_name.as_ptr(),
            p_specialization_info,
        };

        self.shader_stages.push(stage_info);
        self.shader_stage_holders.push(holder);

        self
    }
}

impl<I, D> Drop for ShaderStages<I, D> where
    I: Borrow<Instance> + Deref<Target = Instance>,
    D: Borrow<Device<I>> + Deref<Target = Device<I>>,
{
    fn drop(&mut self) {
        self.shader_stage_holders.iter()
            .for_each(|holder| {
                unsafe { self.device.handle.destroy_shader_module(holder.module, None); }
            })
    }
}

impl Specializations {
    /// offset
    pub fn new<T>(data: T) -> Self {
        let data = unsafe {
            let ptr = &data as *const _ as *mut u8;
            let size = std::mem::size_of::<T>();
            Vec::from_raw_parts(ptr, size, size)
        };

        Self {
            maps: vec![],
            data,
        }
    }

    pub fn constant(mut self, id: u32, range: std::ops::Range<usize>) -> Self {
        self.maps.push(vk::SpecializationMapEntry {
            constant_id: id,
            offset: range.start as u32,
            size: range.end - range.start,
        });
        self
    }
}

