pub mod stage;

use super::*;
use descriptor::DescriptorSetLayout;
use render_pass::RenderPass;

use std::ops::Range;

pub struct PipelineLayout<I, D, S> where D: Borrow<Device<I>> {
    _marker: PhantomData<I>,
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

pub struct GraphicsPipelineCreateInfo<'a> {
    info: vk::GraphicsPipelineCreateInfo,
    vertex_input: Option<&'a vk::PipelineVertexInputStateCreateInfoBuilder<'a>>,
    input_assembly: Option<&'a vk::PipelineInputAssemblyStateCreateInfoBuilder<'a>>,
    tessellation: Option<&'a vk::PipelineTessellationStateCreateInfoBuilder<'a>>,
    viewport: Option<&'a vk::PipelineViewportStateCreateInfoBuilder<'a>>,
    rasterization: Option<&'a vk::PipelineRasterizationStateCreateInfoBuilder<'a>>,
    multisample: Option<&'a vk::PipelineMultisampleStateCreateInfoBuilder<'a>>,
    depth_stencil: Option<&'a vk::PipelineDepthStencilStateCreateInfoBuilder<'a>>,
    color_blend: Option<&'a vk::PipelineColorBlendStateCreateInfoBuilder<'a>>,
    dynamic: Option<&'a vk::PipelineDynamicStateCreateInfoBuilder<'a>>,
}

pub struct GraphicsPipeline<I, D, R, L> where
    D: Borrow<Device<I>>,
    R: Borrow<RenderPass<I, D>>,
{
    _marker: PhantomData<(I, D)>,
    render_pass: R,
    layout: L,
    handle: vk::Pipeline,
}

pub struct ShaderStages<I, D> where D: Borrow<Device<I>> {
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
    I: Borrow<Instance>,
    D: Borrow<Device<I>>,
    S: Borrow<DescriptorSetLayout<I, D>>,
{
    pub fn new(
        device: D,
        set_layouts: Vec<S>,
        push_constants: Vec<vk::PushConstantRange>,
    ) -> Self {
        let layout_handles = set_layouts.iter()
            .map(|layout| layout.borrow().handle())
            .collect::<Vec<_>>();
        let info = vk::PipelineLayoutCreateInfo::builder()
            .push_constant_ranges(&push_constants[..])
            .set_layouts(&layout_handles[..]);

        let handle = unsafe {
            device.borrow().handle.create_pipeline_layout(&info, None).unwrap()
        };

        Self { _marker: PhantomData, device, handle, set_layouts, push_constants }
    }
}


impl<I, D, S> Drop for PipelineLayout<I, D, S> where D: Borrow<Device<I>> {
    fn drop(&mut self) {
        unsafe { self.device.borrow().handle.destroy_pipeline_layout(self.handle, None); }
    }
}

impl<'a> GraphicsPipelineCreateInfo<'a> {
    pub fn vertex_input(
        &mut self,
        state: &'a vk::PipelineVertexInputStateCreateInfoBuilder<'a>,
    ) -> &mut Self {
        self.info.p_vertex_input_state = state as *const _;
        self.vertex_input = Some(state);
        self
    }
    pub fn input_assembly(
        &mut self,
        state: &'a vk::PipelineInputAssemblyStateCreateInfoBuilder<'a>,
    ) -> &mut Self {
        self.info.p_input_assembly_state = state as _;
        self.input_assembly = Some(state);
        self
    }
    pub fn tessellation(
        &mut self,
        state: &'a vk::PipelineTessellationStateCreateInfoBuilder<'a>,
    ) -> &mut Self {
        self.info.p_tessellation_state = state as _;
        self.tessellation = Some(state);
        self
    }
    pub fn viewport(
        &mut self,
        state: &'a vk::PipelineViewportStateCreateInfoBuilder<'a>,
    ) -> &mut Self {
        self.info.p_viewport_state = state as _;
        self.viewport = Some(state);
        self
    }
    pub fn rasterization(
        &mut self,
        state: &'a vk::PipelineRasterizationStateCreateInfoBuilder<'a>,
    ) -> &mut Self {
        self.info.p_rasterization_state = state as _;
        self.rasterization = Some(state);
        self
    }
    pub fn multisample(
        &mut self,
        state: &'a vk::PipelineMultisampleStateCreateInfoBuilder<'a>,
    ) -> &mut Self {
        self.info.p_multisample_state = state as _;
        self.multisample = Some(state);
        self
    }
    pub fn depth_stencil(
        &mut self,
        state: &'a vk::PipelineDepthStencilStateCreateInfoBuilder<'a>,
    ) -> &mut Self {
        self.info.p_depth_stencil_state = state as _;
        self.depth_stencil = Some(state);
        self
    }
    pub fn color_blend(
        &mut self,
        state: &'a vk::PipelineColorBlendStateCreateInfoBuilder<'a>,
    ) -> &mut Self {
        self.info.p_color_blend_state = state as _;
        self.color_blend = Some(state);
        self
    }
    pub fn dynamic(
        &mut self,
        state: &'a vk::PipelineDynamicStateCreateInfoBuilder<'a>,
    ) -> &mut Self {
        self.info.p_dynamic_state = state as _;
        self.dynamic = Some(state);
        self
    }
    pub unsafe fn create<I, D, R, S, L>(
        &self,
        shader_stages: ShaderStages<I, D>,
        (render_pass, subpass): (R, u32),
        layout: L,
        cache: Option<vk::PipelineCache>,
    ) -> GraphicsPipeline<I, D, R, L> where
        I: Borrow<Instance>,
        D: Borrow<Device<I>>,
        R: Borrow<RenderPass<I, D>>,
        S: Borrow<DescriptorSetLayout<I, D>>,
        L: Borrow<PipelineLayout<I, D, S>>,
    {
        let device = render_pass.borrow().device().handle.create_graphics_pipelines();
        unimplemented!()
    }
}

impl<I, D, R, L> GraphicsPipeline<I, D, R, L> where
    I: Borrow<Instance>,
    D: Borrow<Device<I>>,
    R: Borrow<RenderPass<I, D>>,
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
        info.render_pass = render_pass.borrow().handle();
        info.subpass = subpass;
        info.layout = layout.borrow().handle;

        let handle = unsafe {
            render_pass.borrow().device().handle
                .create_graphics_pipelines(vk::PipelineCache::null(), &[info], None)
                .unwrap()[0]
        };

        GraphicsPipeline { _marker: PhantomData, render_pass, layout, handle }
    }

    #[inline]
    pub fn handle(&self) -> vk::Pipeline { self.handle }
}

impl<I, D, R, L> Drop for GraphicsPipeline<I, D, R, L> where
    D: Borrow<Device<I>>,
    R: Borrow<RenderPass<I, D>>,
{
    #[inline]
    fn drop(&mut self) {
        unsafe { self.render_pass.borrow().device().handle.destroy_pipeline(self.handle, None); }
    }
}

impl<I, D> ShaderStages<I, D> where
    I: Borrow<Instance>,
    D: Borrow<Device<I>>,
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

            unsafe { self.device.borrow().handle.create_shader_module(&info, None).unwrap() }
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

impl<I, D> Drop for ShaderStages<I, D> where D: Borrow<Device<I>> {
    fn drop(&mut self) {
        self.shader_stage_holders.iter()
            .for_each(|holder| {
                unsafe { self.device.borrow().handle.destroy_shader_module(holder.module, None); }
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

