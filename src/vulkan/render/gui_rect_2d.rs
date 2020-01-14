//! # About This Module
//! This module provides one shader pipeline intended to use to render the two dimensional
//! rectangle. Also provides some utility structure and function such as vertex structure and
//! shader loading function.
//!
//! # Pipeline Detail
//! The pipeline in this module only provides the ??? to draw two dimensional rectangle with one
//! texture. Don't allow changing color dynamically by uniform or vertex attributes.
//!

use ash::vk;
use ash::version::DeviceV1_0;
use ash::Device;

use crate::linear_algebra::XY;

use super::Vulkan;
use super::Render;
use super::RenderPass;

use std::borrow::Borrow;


/// This is the vertex structure used in the pipeline in this module.
/// The position of the object that is composed of this vertex is determined by shader,
/// so, this vertex has not own position data.
#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq)]
pub struct Vertex {
    /// Texture Coordinates.
    tex_xy: XY<f32>,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq)]
/// The uniform data used in vertex shader.
pub struct Uniform {
    /// Where the anchor of this drawing object is. Same as vulkan coordinates.
    /// This anchor is on object coordinates and is the center of object's scaling;
    /// normalized coordinates to pixel coordinate.
    object_anchor: XY<f32>,

    /// Where the anchor of rendering surface is. Same as vulkan coordinates.
    /// This anchor is on surface coordinates and is a pin to render the object to surface.
    surface_anchor: XY<f32>,

    /// Difference between object anchor and surface anchor in rendering surface pixel size.
    /// This vector is from surface anchor to object anchor.
    delta_of_anchor: XY<f32>,

    /// Scale of drawing object in rendering surface pixel size.
    scale: XY<f32>,
}

/// This struct allows you to allocate vk::DescriptorSet.
pub struct Shader {
    descriptor_set_layout: vk::DescriptorSetLayout,
    pipeline_layout: vk::PipelineLayout,
    pipeline: vk::Pipeline,
}

/// This is the descriptor set used in the pipeline in this module.
pub struct DescriptorSet<U, T> {
    /// Vulkan handle.
    handle: vk::DescriptorSet,

    /// This is uniform data that is used in vertex shader to calculate the position of the object.
    uniform: U,

    /// This is the texture of the object.
    texture: T
}

impl Shader {
    pub fn descriptor_pool_size() -> [vk::DescriptorPoolSize; 2] {
        [
            vk::DescriptorPoolSize::builder()
                .descriptor_count(1)
                .ty(vk::DescriptorType::UNIFORM_BUFFER)
                .build(),
            vk::DescriptorPoolSize::builder()
                .descriptor_count(1)
                .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .build(),
        ]
    }

    unsafe fn load(
        device: &Device,
        render_pass: &RenderPass,
        subpass: u32,
        surface_extent: vk::Extent2D,
        pipeline_cache: vk::PipelineCache,
    ) -> Self {
        // Descriptor Set Layout creation.
        let bindings = [
            vk::DescriptorSetLayoutBinding::builder()
                .binding(0)
                .stage_flags(vk::ShaderStageFlags::VERTEX)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .descriptor_count(1)
                .build(),
            vk::DescriptorSetLayoutBinding::builder()
                .binding(1)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(1)
                .build(),
        ];

        let info = vk::DescriptorSetLayoutCreateInfo::builder()
            .flags(vk::DescriptorSetLayoutCreateFlags::empty())
            .bindings(&bindings[..])
            .build();

        let descriptor_set_layout = device.create_descriptor_set_layout(&info, None).unwrap();


        // Pipeline Layout creation.
        let set_layouts = [descriptor_set_layout];
        let info = vk::PipelineLayoutCreateInfo::builder()
            .flags(vk::PipelineLayoutCreateFlags::empty())
            .set_layouts(&set_layouts[..])
            .build();

        let pipeline_layout = device.create_pipeline_layout(&info, None).unwrap();

        // Graphics Pipeline creation.
        // load vertex shader SPIR-V.
        let vert = include_bytes!("gui_rect_2d/vert.spv")[..];
        let info = vk::ShaderModuleCreateInfo::builder()
            .code(std::slice::from_raw_parts(&vert as *const u8 as *const u32, vert.len() / 4))
            .build();
        let vert = device.create_shader_module(&info, None).unwrap();

        // load fragment shader SPIR-V.
        let frag = include_bytes!("gui_rect_2d/frag.spv")[..];
        let info = vk::ShaderModuleCreateInfo::builder()
            .code(std::slice::from_raw_parts(&vert as *const u8 as *const u32, vert.len() / 4))
            .build();
        let frag = device.create_shader_module(&info, None).unwrap();

        let fn_name = std::ffi::CString::new("main").unwrap();
        let stages = [
            vk::PipelineShaderStageCreateInfo::builder()
                .flags(vk::PipelineShaderStageCreateFlags::empty())
                .module(vert)
                .stage(vk::ShaderStageFlags::VERTEX)
                .name(&fn_name)
                .build(),
            vk::PipelineShaderStageCreateInfo::builder()
                .flags(vk::PipelineShaderStageCreateFlags::empty())
                .module(frag)
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .name(&fn_name)
                .build(),
        ];

        let vertex_bindings = [
            vk::VertexInputBindingDescription::builder()
                .binding(0)
                .input_rate(vk::VertexInputRate::VERTEX)
                .stride(std::mem::size_of::<Vertex>() as u32)
                .build(),
        ];

        let vertex_attributes = [
            vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .offset(0)
                .format(vk::Format::R32G32_SFLOAT)
                .location(0)
                .build(),
        ];

        let vertex_input = vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_binding_descriptions(&vertex_bindings[..])
            .vertex_attribute_descriptions(&vertex_attributes[..])
            .build();

        let assembly = vk::PipelineInputAssemblyStateCreateInfo::builder()
            .flags(vk::PipelineInputAssemblyStateCreateFlags::empty())
            .primitive_restart_enable(false)
            .topology(vk::PrimitiveTopology::TRIANGLE_FAN)
            .build();

        let viewports = [
            vk::Viewport::builder()
                .x(0.0)
                .y(0.0)
                .width(surface_extent.width as f32)
                .height(surface_extent.height as f32)
                .max_depth(1.0)
                .min_depth(0.0)
                .build(),
        ];

        let scissors = [
            vk::Rect2D::builder()
                .offset(vk::Offset2D { x: 0, y: 0 } )
                .extent(vk::Extent2D {
                    width: surface_extent.width,
                    height: surface_extent.height,
                })
                .build(),
        ];

        let viewport = vk::PipelineViewportStateCreateInfo::builder()
            .flags(vk::PipelineViewportStateCreateFlags::empty())
            .viewports(&viewports[..])
            .scissors(&scissors[..])
            .build();

        let rasterization = vk::PipelineRasterizationStateCreateInfo::builder()
            .flags(vk::PipelineRasterizationStateCreateFlags::empty())
            .rasterizer_discard_enable(false)
            .line_width(1.0)
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            .cull_mode(vk::CullModeFlags::BACK)
            .polygon_mode(vk::PolygonMode::FILL)
            .depth_bias_enable(false)
            .depth_clamp_enable(false)
            .build();

        let color_blend_attachments = [
            vk::PipelineColorBlendAttachmentState::builder()
                .color_write_mask(vk::ColorComponentFlags::all())
                .blend_enable(true)
                .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
                .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
                .color_blend_op(vk::BlendOp::ADD)
                .src_alpha_blend_factor(vk::BlendFactor::ONE)
                .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
                .alpha_blend_op(vk::BlendOp::ADD)
                .build(),
        ];

        let color_blend = vk::PipelineColorBlendStateCreateInfo::builder()
            .flags(vk::PipelineColorBlendStateCreateFlags::empty())
            .logic_op_enable(false)
            .attachments(&color_blend_attachments[..])
            .build();

        let info = vk::GraphicsPipelineCreateInfo::builder()
            .flags(vk::PipelineCreateFlags::empty())
            .render_pass(render_pass)
            .subpass(subpass)
            .layout(pipeline_layout)
            .stages(&stages[..])
            .vertex_input_state(&vertex_input)
            .input_assembly_state(&assembly)
            .viewport_state(&viewport)
            .rasterization_state(&rasterization)
            .color_blend_state(&color_blend)
            .build();

        let pipeline = device
            .create_graphics_pipelines(pipeline_cache, &[info], None)
            .map_err(|(_pipelines, err)| err)
            .unwrap()[0];


        todo!()
    }
}

pub unsafe fn load(vulkan: &Vulkan, render: &Render, subpass: u32) -> Shader {

    Shader {
        descriptor_set_layout,
        pipeline_layout,
        pipeline,
    }
}

