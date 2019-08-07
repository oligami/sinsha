use super::*;

/// Host access to CommandPool must be externally synchronized.
/// Synchronization take some cost, so this struct is not Send.
pub struct CommandPool {
    device: Arc<Device>,
    handle: vk::CommandPool,
    _not_send: PhantomData<*const ()>,
}

pub struct CommandBuffer<L, S, PS> {
    pool: Arc<CommandPool>,
    handle: vk::CommandBuffer,
    _level: PhantomData<L>,
    _state: PhantomData<S>,
    pipeline_stater: PS,
}

// TODO: impl Send for Recorded CommandBuffer.
// unsafe impl<L> Send for CommandBuffer<L, ReadyToSubmit> {}

pub trait CommandBufferLevel {
    fn level() -> vk::CommandBufferLevel;
}
pub struct Primary;
pub struct Secondary;
impl CommandBufferLevel for Primary {
    fn level() -> vk::CommandBufferLevel { vk::CommandBufferLevel::PRIMARY }
}
impl CommandBufferLevel for Secondary {
    fn level() -> vk::CommandBufferLevel { vk::CommandBufferLevel::SECONDARY }
}

pub struct PipelineStater<SLs, SL, PCs, PC, DSs, DS> {
    pipeline_handle: vk::Pipeline,
    pipeline_layout_handle: vk::PipelineLayout,
    set_layouts: PhantomData<SLs>,
    set_layout: PhantomData<SL>,
    push_constants: PhantomData<PCs>,
    push_constant: PhantomData<PC>,
    dynamic_states: PhantomData<DSs>,
    dynamic_state: PhantomData<DS>,
}


pub struct Initial;
pub struct Recording;
pub struct InRenderPass<SpC, Sp, PSp> {
    subpass_contents: PhantomData<SpC>,
    subpass: PhantomData<Sp>,
    proceeding_subpass: PhantomData<PSp>,
}
pub struct ReadyToSubmit;
pub struct Submitted;


pub trait SubpassContents {
    fn subpass_contents() -> vk::SubpassContents;
}
pub struct Inline;
pub struct SecondaryCommandBuffers;
impl SubpassContents for Inline {
    fn subpass_contents() -> vk::SubpassContents { vk::SubpassContents::INLINE }
}
impl SubpassContents for SecondaryCommandBuffers{
    fn subpass_contents() -> vk::SubpassContents { vk::SubpassContents::SECONDARY_COMMAND_BUFFERS }
}



impl CommandPool {
    pub unsafe fn new<T>(
        device: Arc<Device>,
        flags: vk::CommandPoolCreateFlags,
        queue: Queue<T>,
    ) -> Self {
        let info = vk::CommandPoolCreateInfo {
            s_type: StructureType::COMMAND_POOL_CREATE_INFO,
            p_next: ptr::null(),
            flags,
            queue_family_index: queue.family_index,
        };

        let handle = device.handle.create_command_pool(&info, None).unwrap();

        Self { device, handle, _not_send: PhantomData }
    }

    pub fn allocate<L>(
        &self, level: L, count: u32
    ) -> Vec<CommandBuffer<L, Initial, ()>> where L: CommandBufferLevel {
        let info = vk::CommandBufferAllocateInfo {
            s_type: StructureType::COMMAND_POOL_CREATE_INFO,
            p_next: ptr::null(),
            level: L::level(),
            command_pool: self.handle,
            command_buffer_count: count,
        };

        let handles = unsafe { self.device.handle.allocate_command_buffers(&info).unwrap() };

        handles.into_iter()
            .map(|handle| CommandBuffer {
                handle,
                pool: unimplemented!(),
                _level: PhantomData,
                _state: PhantomData,
                pipeline_stater: (),
            })
            .collect()
    }

    pub unsafe fn reset(&self) {
        self.device.handle
            .reset_command_pool(self.handle, vk::CommandPoolResetFlags::empty())
            .unwrap()
    }
}

impl<L, S> CommandBuffer<L, S, ()> {
    fn into<L1, S1>(self) -> CommandBuffer<L1, S1, ()> {
        CommandBuffer {
            handle: self.handle,
            pool: self.pool,
            _level: PhantomData,
            _state: PhantomData,
            pipeline_stater: self.pipeline_stater,
        }
    }
}

impl<L, S, SLs, SL, PCs, PC, DSs, DS> CommandBuffer<L, S, PipelineStater<SLs, SL, PCs, PC, DSs, DS>> {
    fn into<L1, S1, SL1, PC1, DS1>(self) -> CommandBuffer<L1, S1, PipelineStater<SLs, SL1, PCs, PC1, DSs, DS1>> {
        CommandBuffer {
            handle: self.handle,
            pool: self.pool,
            _level: PhantomData,
            _state: PhantomData,
            pipeline_stater: self.pipeline_stater.into(),
        }
    }
}

impl CommandBuffer<Primary, Initial, ()> {
    pub fn begin(self, usage: vk::CommandBufferUsageFlags) -> CommandBuffer<Primary, Recording, ()> {
        let info = vk::CommandBufferBeginInfo {
            s_type: StructureType::COMMAND_BUFFER_BEGIN_INFO,
            p_next: ptr::null(),
            flags: usage,
            p_inheritance_info: ptr::null(),
        };

        unsafe { self.pool.device.handle.begin_command_buffer(self.handle, &info).unwrap() }

        self.into()
    }
}

impl CommandBuffer<Secondary, Initial, ()> {
    pub unsafe fn begin(
        self,
        usage: vk::CommandBufferUsageFlags,
        inheritance: vk::CommandBufferInheritanceInfo,
    ) -> CommandBuffer<Secondary, Recording, ()> {
        let info = vk::CommandBufferBeginInfo {
            s_type: StructureType::COMMAND_BUFFER_BEGIN_INFO,
            p_next: ptr::null(),
            flags: usage,
            p_inheritance_info: &inheritance as *const _,
        };

        self.pool.device.handle.begin_command_buffer(self.handle, &info).unwrap();

        self.into()
    }
}


impl<L, PS> CommandBuffer<L, Recording, PS> {
    pub fn copy<D1, D2>(src: D1, dst: D2) -> Self {
        unimplemented!()
    }
}

impl<PS> CommandBuffer<Primary, Recording, PS> {
    pub fn begin_render_pass<A, S, P, SpC>(
        self,
        render_pass: Arc<render_pass::RenderPass<A, S>>,
        framebuffer: (),
        render_area: vk::Rect2D,
        clear_values: &[vk::ClearValue],
        pipeline_bind_point: P,
        subpass_contents: SpC,
    ) -> CommandBuffer<Primary, InRenderPass<SpC, S, ((), P),>, PS>
        where SpC: SubpassContents,
              P: pipeline::bind_point::PipelineBindPoint
    {
        let info = vk::RenderPassBeginInfo {
            s_type: StructureType::RENDER_PASS_BEGIN_INFO,
            p_next: ptr::null(),
            render_pass: render_pass.handle(),
            framebuffer: unimplemented!(),
            render_area,
            clear_value_count: clear_values.len() as u32,
            p_clear_values: clear_values.as_ptr(),
        };

        let contents = SpC::subpass_contents();
        unsafe { self.pool.device.handle.cmd_begin_render_pass(self.handle, &info, contents) }

        CommandBuffer {
            pool: self.pool,
            handle: self.handle,
            _level: PhantomData,
            _state: PhantomData,
            pipeline_stater: self.pipeline_stater,
        }
    }
}

impl<L, Sp, PSp, PS> CommandBuffer<
    L,
    InRenderPass<Inline, Sp, (PSp, pipeline::bind_point::Graphics)>,
    PS,
> {
    pub fn bind_graphics_pipeline<A, S, P, Ls, V, D>(
        self, pipeline: Arc<pipeline::GraphicsPipeline<A, S, P, Ls, V, D>>
    ) -> CommandBuffer<
        L,
        InRenderPass<Inline, Sp, (PSp, pipeline::bind_point::Graphics)>,
        PipelineStater<Ls, Ls, P, P, D, D>,
    > {
        use pipeline::PipelineBindPoint;

        unsafe {
            self.pool.device.handle.cmd_bind_pipeline(
                self.handle,
                pipeline::bind_point::Graphics::bind_point(),
                pipeline.handle(),
            );
        }

        CommandBuffer {
            pool: self.pool,
            handle: self.handle,
            _level: PhantomData,
            _state: PhantomData,
            pipeline_stater: PipelineStater {
                pipeline_handle: pipeline.handle(),
                pipeline_layout_handle: pipeline.layout(),
                set_layouts: PhantomData,
                set_layout: PhantomData,
                push_constants: PhantomData,
                push_constant: PhantomData,
                dynamic_states: PhantomData,
                dynamic_state: PhantomData,
            }
        }
    }
}

impl<L, State, SLs, SL, PCs, PC, DSs0, DSs1, DS> CommandBuffer<
    L,
    State,
    PipelineStater<SLs, SL, PCs, PC, DSs0, (DSs1, DS)>
>
    where DS: pipeline::DynamicState,
{
    pub fn set_dynamic_state(
        self, state: &DS::Type,
    ) -> CommandBuffer<
        L,
        State,
        PipelineStater<SLs, SL, PCs, PC, DSs0, DSs1>
    > {
        unsafe { DS::set_state(&self.pool.device.handle, self.handle, state); }
        self.into()
    }
}

impl<L, State, SLs, SL, PCs0, PCs1, DSs, DS, Ss, T> CommandBuffer<
    L,
    State,
    PipelineStater<SLs, SL, PCs0, (PCs1, pipeline::PushConstant<Ss, T>), DSs, DS>
> where Ss: shader::stage::ShaderStages {
    pub fn bind_push_constant(
        self, push_constant: &T
    ) -> CommandBuffer<
        L,
        State,
        PipelineStater<SLs, SL, PCs0, PCs1, DSs, DS>
    > {
        let ptr = push_constant as *const T as *const u8;
        let size = std::mem::size_of::<T>();


        unsafe {
            let bytes = std::slice::from_raw_parts(ptr, size);
            // NOTE: PipelineLayout, ShaderStageFlags, offset, [u8]
            self.pool.device.handle.cmd_push_constants(
                self.handle,
                self.pipeline_stater.pipeline_layout_handle,
                Ss::shader_stages(),
                0,
                bytes,
            );

            std::mem::forget(bytes);
        }

        self.into()
    }
}

impl<L, State, SLs0, SLs1, SL, PCs, PC, DSs, DS> CommandBuffer<
    L,
    State,
    PipelineStater<SLs0, (SLs1, SL), PCs, PC, DSs, DS>
> {
    pub fn bind_descriptor_sets<R>(self, descriptor_set: Arc<descriptor::DescriptorSet<SL, R>>) {
        unsafe {
//			self.pool.device.handle.cmd_bind_descriptor_sets()
        }
    }

}

impl<Sp, PSp, PS> CommandBuffer<
    Primary,
    InRenderPass<SecondaryCommandBuffers, Sp, PSp>,
    PS,
> {
    pub unsafe fn execute_commands(self, secondary_command_buffers: ()) -> Self {
        self.pool.device.handle.cmd_execute_commands(
            self.handle,
            unimplemented!()
        );

        self.into()
    }
}

impl<SLs, SL, PCs, PC, DSs, DS> PipelineStater<SLs, SL,PCs, PC, DSs, DS> {
    fn into<SL1, PC1, DS1>(self) -> PipelineStater<SLs, SL1, PCs, PC1, DSs, DS1> {
        PipelineStater {
            pipeline_handle: self.pipeline_handle,
            pipeline_layout_handle: self.pipeline_layout_handle,
            set_layouts: PhantomData,
            set_layout: PhantomData,
            push_constants: PhantomData,
            push_constant: PhantomData,
            dynamic_states: PhantomData,
            dynamic_state: PhantomData,
        }
    }
}


