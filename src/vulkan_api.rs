mod memory;
mod buffer_and_image;
mod shaders;
mod command_recorder;

pub use self::buffer_and_image::*;
pub use self::shaders::*;
pub use self::command_recorder::*;

use ash::extensions::khr;
use ash::version::DeviceV1_0;
use ash::version::EntryV1_0;
use ash::version::InstanceV1_0;
use ash::vk;
use ash::vk::StructureType;
use ash::vk_make_version;
use ash::Device;
use ash::Entry;
use ash::Instance;

use winit::Window;

use crate::linear_algebra::*;

use std::default::Default;
use std::ffi::CString;
use std::ops::Drop;
use std::ops::{Index, IndexMut};
use std::ptr;


pub struct PhysicalDevice {
	pub raw_handle: vk::PhysicalDevice,
	pub memory_properties: vk::PhysicalDeviceMemoryProperties,
	pub queue_family_index: u32,
}

struct Surface {
	loader: khr::Surface,
	raw_handle: vk::SurfaceKHR,
}

pub struct VkDevices {
	entry: Entry,
	instance: Instance,
	pub device: Device,
	pub queue: vk::Queue,
	pub physical_device: PhysicalDevice,
	surface: Surface,
}

struct Swapchain {
	loader: khr::Swapchain,
	raw_handle: vk::SwapchainKHR,
	images: Vec<(vk::Image, vk::ImageView)>,
	data: SwapchainData,
}

struct SwapchainData {
	min_image_count: u32,
	format: vk::Format,
	color_space: vk::ColorSpaceKHR,
	extent: vk::Extent2D,
	array_layers: u32,
	usage: vk::ImageUsageFlags,
	sharing_mode: vk::SharingMode,
	queue_family_index_count: u32,
	p_queue_family_indices: *const u32,
	pre_transform: vk::SurfaceTransformFlagsKHR,
	composite_alpha: vk::CompositeAlphaFlagsKHR,
	present_mode: vk::PresentModeKHR,
	clipped: vk::Bool32,
}

pub struct GraphicRenderer<'vk_devices> {
	vk_devices: &'vk_devices VkDevices,
	swapchain: Swapchain,
	render_pass: vk::RenderPass,
	framebuffers: Vec<vk::Framebuffer>,
	shaders: Shaders,
}

struct Semaphores {
	image_acquired: vk::Semaphore,
	render_finished: Vec<vk::Semaphore>,
}

struct Commandbuffers {
	pool: vk::CommandPool,
	raw_buffers: Vec<vk::CommandBuffer>,
}

pub struct Vulkan {
	entry: Entry,
	instance: Instance,
	physical_device: PhysicalDevice,
	device: Device,
	queue: vk::Queue,
	surface: Surface,
	swapchain: Swapchain,
	shaders: Shaders,
	render_pass: vk::RenderPass,
	framebuffers: Vec<vk::Framebuffer>,
	command: Commandbuffers,
	semaphores: Semaphores,
}

impl PhysicalDevice {
	pub fn queue_family_index_ptr(&self) -> *const u32 {
		&self.queue_family_index as *const _
	}
}

impl VkDevices {
	pub fn new(window: &Window) -> Self {
		unsafe {
			let entry: Entry = Entry::new().expect("Can't entry Vulkan API.");

			// -----------------------------------------------------------------Instance creation start.
			let app_name = CString::new("sinsha").unwrap();
			let engine_name = CString::new("No Engine").unwrap();
			let app_info = vk::ApplicationInfo {
				s_type: StructureType::APPLICATION_INFO,
				p_next: ptr::null(),
				p_application_name: app_name.as_ptr(),
				application_version: vk_make_version!(0, 1, 0),
				engine_version: vk_make_version!(0, 1, 0),
				p_engine_name: engine_name.as_ptr(),
				api_version: vk_make_version!(1, 1, 85),
			};

			let instance_extensions = instance_extensions();
			let debug_layer = CString::new("VK_LAYER_LUNARG_standard_validation").unwrap();
			let instance_layers = if cfg!(debug_assertions) {
				vec![debug_layer.as_ptr()]
			} else {
				vec![]
			};
			let instance_info = vk::InstanceCreateInfo {
				s_type: StructureType::INSTANCE_CREATE_INFO,
				p_next: ptr::null(),
				flags: vk::InstanceCreateFlags::empty(),
				p_application_info: &app_info,
				enabled_extension_count: instance_extensions.len() as u32,
				pp_enabled_extension_names: instance_extensions.as_ptr(),
				enabled_layer_count: instance_layers.len() as u32,
				pp_enabled_layer_names: instance_layers.as_ptr(),
			};

			let instance: Instance = entry
				.create_instance(&instance_info, None)
				.expect("Instance creation has failed.");

			let surface = Surface {
				raw_handle: surface(&entry, &instance, window),
				loader: khr::Surface::new(&entry, &instance),
			};

			let physical_devices = instance
				.enumerate_physical_devices()
				.expect("Failed to find GPUs supported by Vulkan.");

			let physical_device = {
				let (physical_device, queue_family_index) = physical_devices
					.into_iter()
					.find_map(|physical_device| {
						instance
							.get_physical_device_queue_family_properties(physical_device)
							.iter()
							.enumerate()
							.position(|(i, property)| {
								let surface_support = surface.loader
									.get_physical_device_surface_support(
										physical_device,
										i as u32,
										surface.raw_handle,
									);
								let graphic_support = property.queue_flags
									.contains(vk::QueueFlags::GRAPHICS);

								surface_support && graphic_support
							})
							.map(|index| (physical_device, index as u32))
					})
					.unwrap();

				let memory_properties = instance
					.get_physical_device_memory_properties(physical_device);

				PhysicalDevice {
					raw_handle: physical_device,
					memory_properties,
					queue_family_index,
				}
			};

			let queue_priority = 1.0_f32;
			let queue_info = vk::DeviceQueueCreateInfo {
				s_type: StructureType::DEVICE_QUEUE_CREATE_INFO,
				p_next: ptr::null(),
				flags: vk::DeviceQueueCreateFlags::empty(),
				queue_family_index: physical_device.queue_family_index,
				queue_count: 1,
				p_queue_priorities: &queue_priority as *const f32,
			};

			let queue_infos = [queue_info];

			let device_extensions = [khr::Swapchain::name().as_ptr()];
			let features = vk::PhysicalDeviceFeatures::default();
			let device_info = vk::DeviceCreateInfo {
				s_type: StructureType::DEVICE_CREATE_INFO,
				p_next: ptr::null(),
				flags: vk::DeviceCreateFlags::empty(),
				queue_create_info_count: queue_infos.len() as u32,
				p_queue_create_infos: queue_infos.as_ptr(),
				enabled_layer_count: 0,
				pp_enabled_layer_names: ptr::null(),
				enabled_extension_count: device_extensions.len() as u32,
				pp_enabled_extension_names: device_extensions.as_ptr(),
				p_enabled_features: &features as *const _,
			};

			let device = instance
				.create_device(physical_device.raw_handle, &device_info, None)
				.expect("Creating device has failed.");
			let queue = device.get_device_queue(physical_device.queue_family_index, 0);

			Self {
				entry,
				instance,
				physical_device,
				device,
				queue,
				surface,
			}
		}
	}
}

impl Drop for VkDevices {
	fn drop(&mut self) {
		unsafe {
			self.surface.loader.destroy_surface(self.surface.raw_handle, None);
			self.device.destroy_device(None);
			self.instance.destroy_instance(None);
		}
	}
}

impl Swapchain {
	pub fn render_xy(&self) -> XY {
		XY::new(self.data.extent.width as f32, self.data.extent.height as f32)
	}
}

impl<'vk_devices> GraphicRenderer<'vk_devices> {
	pub fn new(vk_devices: &'vk_devices VkDevices) -> Self {
		unsafe {
			// create swapchain
			let (image_format, image_color_space) = vk_devices.surface
				.loader
				.get_physical_device_surface_formats(
					vk_devices.physical_device.raw_handle,
					vk_devices.surface.raw_handle
				)
				.unwrap()
				.iter()
				.map(|format| {
					match format.format {
						vk::Format::UNDEFINED => (
							vk::Format::R8G8B8A8_UNORM,
							vk::ColorSpaceKHR::EXTENDED_SRGB_NONLINEAR_EXT
						),
						_ => (format.format, format.color_space),
					}
				})
				.next()
				.unwrap();

			let present_mode = {
				let available_present_modes = vk_devices.surface
					.loader
					.get_physical_device_surface_present_modes(
						vk_devices.physical_device.raw_handle,
						vk_devices.surface.raw_handle,
					)
					.unwrap();

				available_present_modes
					.iter()
					.find(|&&mode| mode == vk::PresentModeKHR::MAILBOX)
					.map(|mode| *mode)
					.unwrap_or(available_present_modes[0])
			};

			let surface_capabilities = vk_devices.surface
				.loader
				.get_physical_device_surface_capabilities(
					vk_devices.physical_device.raw_handle,
					vk_devices.surface.raw_handle,
				)
				.unwrap();

			let image_extent = surface_capabilities.current_extent;

			let image_count = match surface_capabilities.max_image_count {
				0 => surface_capabilities.min_image_count + 1,
				max @ _ => if surface_capabilities.min_image_count <= max {
					surface_capabilities.min_image_count + 1
				} else {
					surface_capabilities.min_image_count
				},
			};

			let swapchain_info = vk::SwapchainCreateInfoKHR {
				s_type: StructureType::SWAPCHAIN_CREATE_INFO_KHR,
				p_next: ptr::null(),
				flags: vk::SwapchainCreateFlagsKHR::empty(),
				surface: vk_devices.surface.raw_handle,
				min_image_count: image_count,
				image_format,
				image_color_space,
				image_extent,
				image_array_layers: 1,
				image_usage: vk::ImageUsageFlags::COLOR_ATTACHMENT,
				image_sharing_mode: vk::SharingMode::EXCLUSIVE,
				queue_family_index_count: 1,
				p_queue_family_indices: vk_devices.physical_device.queue_family_index_ptr(),
				pre_transform: surface_capabilities.current_transform,
				composite_alpha: vk::CompositeAlphaFlagsKHR::OPAQUE,
				present_mode,
				clipped: vk::TRUE,
				old_swapchain: vk::SwapchainKHR::null(),
			};

			let swapchain = {
				let data = SwapchainData {
					min_image_count: swapchain_info.min_image_count,
					format: swapchain_info.image_format,
					color_space: swapchain_info.image_color_space,
					extent: swapchain_info.image_extent,
					array_layers: swapchain_info.image_array_layers,
					usage: swapchain_info.image_usage,
					sharing_mode: swapchain_info.image_sharing_mode,
					queue_family_index_count: swapchain_info.queue_family_index_count,
					p_queue_family_indices: swapchain_info.p_queue_family_indices,
					pre_transform: swapchain_info.pre_transform,
					present_mode: swapchain_info.present_mode,
					composite_alpha: swapchain_info.composite_alpha,
					clipped: swapchain_info.clipped,
				};

				let loader = khr::Swapchain::new(&vk_devices.instance, &vk_devices.device);

				let raw_handle = loader
					.create_swapchain(&swapchain_info, None)
					.expect("Swap chain creation has failed.");

				let images = loader
					.get_swapchain_images(raw_handle)
					.expect("Failed to get swap chain images.");

				let images: Vec<_> = images
					.into_iter()
					.map(|image| {
						let image_view_info = vk::ImageViewCreateInfo {
							s_type: StructureType::IMAGE_VIEW_CREATE_INFO,
							p_next: ptr::null(),
							flags: vk::ImageViewCreateFlags::empty(),
							image,
							view_type: vk::ImageViewType::TYPE_2D,
							format: image_format,
							components: vk::ComponentMapping {
								r: vk::ComponentSwizzle::IDENTITY,
								g: vk::ComponentSwizzle::IDENTITY,
								b: vk::ComponentSwizzle::IDENTITY,
								a: vk::ComponentSwizzle::IDENTITY,
							},
							subresource_range: vk::ImageSubresourceRange {
								aspect_mask: vk::ImageAspectFlags::COLOR,
								base_mip_level: 0,
								level_count: 1,
								base_array_layer: 0,
								layer_count: 1,
							},
						};

						let image_view = vk_devices.device
							.create_image_view(&image_view_info, None)
							.unwrap();

						(image, image_view)
					})
					.collect();

				Swapchain {
					loader,
					raw_handle,
					images,
					data,
				}
			};

			// create render pass
			let attachments = [
				vk::AttachmentDescription {
					flags: vk::AttachmentDescriptionFlags::empty(),
					format: swapchain.data.format,
					samples: vk::SampleCountFlags::TYPE_1,
					load_op: vk::AttachmentLoadOp::CLEAR,
					store_op: vk::AttachmentStoreOp::STORE,
					stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
					stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
					initial_layout: vk::ImageLayout::UNDEFINED,
					final_layout: vk::ImageLayout::PRESENT_SRC_KHR,
				},
			];

			let color_attachment_ref = [
				vk::AttachmentReference {
					attachment: 0,
					layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
				}
			];

			let subpasses = [
				vk::SubpassDescription {
					flags: vk::SubpassDescriptionFlags::empty(),
					pipeline_bind_point: vk::PipelineBindPoint::GRAPHICS,
					color_attachment_count: color_attachment_ref.len() as u32,
					p_color_attachments: color_attachment_ref.as_ptr(),
					input_attachment_count: 0,
					p_input_attachments: ptr::null(),
					p_resolve_attachments: ptr::null(),
					p_depth_stencil_attachment: ptr::null(),
					preserve_attachment_count: 0,
					p_preserve_attachments: ptr::null(),
				}
			];

			let subpass_dependencies = [
				vk::SubpassDependency {
					src_subpass: vk::SUBPASS_EXTERNAL,
					dst_subpass: 0,
					src_access_mask: vk::AccessFlags::empty(),
					dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_WRITE
						| vk::AccessFlags::COLOR_ATTACHMENT_READ,
					src_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
					dst_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
					dependency_flags: vk::DependencyFlags::BY_REGION,
				},
			];

			let render_pass_info = vk::RenderPassCreateInfo {
				s_type: StructureType::RENDER_PASS_CREATE_INFO,
				p_next: ptr::null(),
				flags: vk::RenderPassCreateFlags::empty(),
				attachment_count: attachments.len() as u32,
				p_attachments: attachments.as_ptr(),
				subpass_count: subpasses.len() as u32,
				p_subpasses: subpasses.as_ptr(),
				dependency_count: subpass_dependencies.len() as u32,
				p_dependencies: subpass_dependencies.as_ptr(),
			};

			let render_pass = vk_devices.device
				.create_render_pass(&render_pass_info, None)
				.expect("Failed to create render pass.");

			// create shaders
			let shaders = Shaders::load(&vk_devices.device, render_pass);

			// create framebuffer
			let mut framebuffers = Vec::with_capacity(swapchain.images.len());
			for (_, image_view) in swapchain.images.iter() {
				let framebuffer_info = vk::FramebufferCreateInfo {
					s_type: StructureType::FRAMEBUFFER_CREATE_INFO,
					p_next: ptr::null(),
					flags: vk::FramebufferCreateFlags::empty(),
					render_pass,
					attachment_count: 1,
					p_attachments: image_view as *const _,
					width: swapchain.data.extent.width,
					height: swapchain.data.extent.height,
					layers: 1,
				};

				framebuffers.push(
					vk_devices.device
						.create_framebuffer(&framebuffer_info, None)
						.expect("Failed to create a framebuffer."),
				);
			}

			Self {
				vk_devices,
				swapchain,
				render_pass,
				framebuffers,
				shaders,
			}
		}
	}
}

impl Drop for GraphicRenderer<'_> {
	fn drop(&mut self) {
		unsafe {
			self.swapchain.images
				.iter()
				.for_each(|&(image, view)| {
					self.vk_devices.device.destroy_image(image, None);
					self.vk_devices.device.destroy_image_view(view, None);
				});
			self.vk_devices.device.destroy_render_pass(self.render_pass, None);
			self.framebuffers
				.iter()
				.for_each(|&framebuffer| {
					self.vk_devices.device.destroy_framebuffer(framebuffer, None);
				});
			self.vk_devices.device.destroy_pipeline(self.shaders.gui.pipeline, None);
			self.vk_devices.device.destroy_pipeline_layout(self.shaders.gui.pipeline_layout, None);
			self.vk_devices.device
				.destroy_descriptor_set_layout(self.shaders.gui.descriptor_set_layout, None);
		}
	}
}

impl Index<usize> for Commandbuffers {
	type Output = vk::CommandBuffer;
	fn index(&self, index: usize) -> &vk::CommandBuffer {
		&self.raw_buffers[index]
	}
}

impl IndexMut<usize> for Commandbuffers {
	fn index_mut(&mut self, index: usize) -> &mut vk::CommandBuffer {
		&mut self.raw_buffers[index]
	}
}

/// public fn
impl Vulkan {
	pub fn new(window: &Window) -> Self {
		unsafe {
			Self::unsafe_new(window)
		}
	}

	pub fn gui_descriptor_sets(&self, textures: &[vk::DescriptorImageInfo]) -> DescriptorSets {
		unsafe {
			DescriptorSets::gui(
				&self.device,
				self.shaders.gui.descriptor_set_layout,
				self.swapchain.images.len(),
				textures,
			)
		}
	}

	pub fn command_recorder(&self) -> CommandRecorder<Uninitialized> {
		CommandRecorder::new(
			&self.physical_device,
			&self.device,
			&self.queue,
		)
	}

	pub fn submit_command_recorder(
		&self,
		command_recorder: &CommandRecorder<End>,
		wait_dst_stage_mask: vk::PipelineStageFlags,
		wait_semaphores: &[vk::Semaphore],
		signal_semaphores: &[vk::Semaphore],
		fence: &vk::Fence,
	) {
		command_recorder.submit(
			wait_dst_stage_mask,
			wait_semaphores,
			signal_semaphores,
			fence,
		)
	}

	pub fn begin_frame(&self) -> Result<GraphicCommandRecorder<Uninitialized>, vk::Result> {
		let idx = self
			.acquire_available_swapchain_image_idx(
				!0_u64,
				vk::Fence::null(),
			)?;

		Ok(
			GraphicCommandRecorder::new(
				&self.device,
				&self.queue,
				idx,
				&self.command.raw_buffers,
				&self.swapchain,
				&self.shaders,
				&self.render_pass,
				&self.framebuffers
			)
		)
	}

	pub fn end_frame(
		&self,
		command_recorder: GraphicCommandRecorder<End>
	) -> Result<(), vk::Result> {
		let idx = command_recorder.index();

		command_recorder.submit(
			&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT],
			&[self.semaphores.image_acquired],
			&self.semaphores.render_finished[idx..idx+1],
			vk::Fence::null(),
		);

		self.present(idx)?;
		self.wait_for_present();
		Ok(())
	}

	pub fn default_sampler(&self) -> vk::Sampler {
		unsafe {
			self.device
				.create_sampler(
					&vk::SamplerCreateInfo {
						s_type: StructureType::SAMPLER_CREATE_INFO,
						p_next: ptr::null(),
						flags: vk::SamplerCreateFlags::empty(),
						mag_filter: vk::Filter::LINEAR,
						min_filter: vk::Filter::LINEAR,
						address_mode_u: vk::SamplerAddressMode::REPEAT,
						address_mode_v: vk::SamplerAddressMode::REPEAT,
						address_mode_w: vk::SamplerAddressMode::REPEAT,
						mipmap_mode: vk::SamplerMipmapMode::LINEAR,
						mip_lod_bias: 0_f32,
						min_lod: 0_f32,
						max_lod: 0_f32,
						anisotropy_enable: vk::FALSE,
						max_anisotropy: 1_f32,
						compare_enable: vk::FALSE,
						compare_op: vk::CompareOp::ALWAYS,
						border_color: vk::BorderColor::FLOAT_OPAQUE_BLACK,
						unnormalized_coordinates: vk::FALSE,
					},
					None,
				)
				.unwrap()
		}
	}

	pub fn default_clear_value(&self) -> [vk::ClearValue; 1] {
		[
			vk::ClearValue {
				color: vk::ClearColorValue { float32: [0.0, 0.0, 0.0, 1.0] }
			}
		]
	}

	pub fn queue_wait_idle(&self) {
		unsafe {
			self.device.queue_wait_idle(self.queue).unwrap();
		}
	}

	pub fn deal_with_window_resize(&mut self) {
		unsafe {
			self.deal_with_window_resize_unsafe();
		}
	}
}

/// private fn
impl Vulkan {
	fn acquire_available_swapchain_image_idx(
		&self,
		timeout: u64,
		fence: vk::Fence
	) -> Result<usize, vk::Result> {
		unsafe {
			let idx = self.swapchain.loader
				.acquire_next_image(
					self.swapchain.raw_handle,
					timeout,
					self.semaphores.image_acquired,
					fence
				)?
				.0 as usize;

			Ok(idx)
		}
	}

	fn present(&self, idx: usize) -> Result<(), vk::Result> {
		unsafe {
			self.swapchain.loader
				.queue_present(
					self.queue,
					&vk::PresentInfoKHR {
						s_type: StructureType::PRESENT_INFO_KHR,
						p_next: ptr::null(),
						wait_semaphore_count: 1,
						p_wait_semaphores: self.semaphores.render_finished[idx..idx+1].as_ptr(),
						swapchain_count: 1,
						p_swapchains: &self.swapchain.raw_handle as *const _,
						p_image_indices: &(idx as u32) as *const _,
						p_results: ptr::null_mut(),
					},
				)?;

			Ok(())
		}
	}

	fn wait_for_present(&self) {
		unsafe {
			self.device.queue_wait_idle(self.queue).unwrap();
		}
	}

	unsafe fn unsafe_new(window: &Window) -> Self {
		let entry: Entry = Entry::new().expect("Can't entry Vulkan API.");

		// -----------------------------------------------------------------Instance creation start.
		let app_name = CString::new("Sinsha").unwrap();
		let engine_name = CString::new("No Engine").unwrap();
		let app_info = vk::ApplicationInfo {
			s_type: StructureType::APPLICATION_INFO,
			p_next: ptr::null(),
			p_application_name: app_name.as_ptr(),
			application_version: vk_make_version!(0, 1, 0),
			engine_version: vk_make_version!(0, 1, 0),
			p_engine_name: engine_name.as_ptr(),
			api_version: vk_make_version!(1, 1, 85),
		};

		let instance_extensions = instance_extensions();
		let debug_layer = CString::new("VK_LAYER_LUNARG_standard_validation").unwrap();
		let instance_layers = if cfg!(debug_assertions) {
			vec![debug_layer.as_ptr()]
		} else {
			vec![]
		};
		let instance_info = vk::InstanceCreateInfo {
			s_type: StructureType::INSTANCE_CREATE_INFO,
			p_next: ptr::null(),
			flags: vk::InstanceCreateFlags::empty(),
			p_application_info: &app_info,
			enabled_extension_count: instance_extensions.len() as u32,
			pp_enabled_extension_names: instance_extensions.as_ptr(),
			enabled_layer_count: instance_layers.len() as u32,
			pp_enabled_layer_names: instance_layers.as_ptr(),
		};

		let instance: Instance = entry
			.create_instance(&instance_info, None)
			.expect("Instance creation has failed.");
		// ------------------------------------------------------------------ Instance creation end.

		// ----------------------------------------------------------------- Surface creation start.
		let surface = Surface {
			loader: khr::Surface::new(&entry, &instance),
			raw_handle: surface(&entry, &instance, window),
		};
		// ------------------------------------------------------------------- Surface creation end.

		// ------------------------------------------ PhysicalDevice and QueueFamily creation start.
		let physical_devices = instance
			.enumerate_physical_devices()
			.expect("Failed to find GPUs supported by Vulkan.");

		let physical_device = {
			let (physical_device, queue_family_index) = physical_devices
				.into_iter()
				.find_map(|physical_device| {
					let _device_properties =
						instance.get_physical_device_properties(physical_device);

					let _device_features = instance.get_physical_device_features(physical_device);

					let queue_family_properties =
						instance.get_physical_device_queue_family_properties(physical_device);

					// Judge if device has valid features.
					queue_family_properties
						.iter()
						.enumerate()
						.position(|(i, property)| {
							property.queue_flags.contains(vk::QueueFlags::GRAPHICS)
								&& surface.loader.get_physical_device_surface_support(
									physical_device,
									i as u32,
									surface.raw_handle,
								)
						})
						.map(|index| (physical_device, index as u32))
				})
				.expect("There is no valid physical device or queue family.");

			let memory_properties = instance.get_physical_device_memory_properties(physical_device);

			PhysicalDevice {
				raw_handle: physical_device,
				memory_properties,
				queue_family_index,
			}
		};
		// -------------------------------------------- PhysicalDevice and QueueFamily creation end.

		// -------------------------------------------------------- Device and Queue creation start.
		let queue_priority = 1.0_f32;
		let queue_info = vk::DeviceQueueCreateInfo {
			s_type: StructureType::DEVICE_QUEUE_CREATE_INFO,
			p_next: ptr::null(),
			flags: vk::DeviceQueueCreateFlags::empty(),
			queue_family_index: physical_device.queue_family_index,
			queue_count: 1,
			p_queue_priorities: &queue_priority as *const f32,
		};

		let queue_infos = vec![queue_info];

		let device_extensions = [khr::Swapchain::name().as_ptr()];
		let features = vk::PhysicalDeviceFeatures::default();
		let device_info = vk::DeviceCreateInfo {
			s_type: StructureType::DEVICE_CREATE_INFO,
			p_next: ptr::null(),
			flags: vk::DeviceCreateFlags::empty(),
			queue_create_info_count: queue_infos.len() as u32,
			p_queue_create_infos: queue_infos.as_ptr(),
			enabled_layer_count: 0,
			pp_enabled_layer_names: ptr::null(),
			enabled_extension_count: device_extensions.len() as u32,
			pp_enabled_extension_names: device_extensions.as_ptr(),
			p_enabled_features: &features as *const _,
		};

		let device = instance
			.create_device(physical_device.raw_handle, &device_info, None)
			.expect("Creating device has failed.");
		let queue = device.get_device_queue(physical_device.queue_family_index, 0);
		// ---------------------------------------------------------- Device and Queue creation end.

		// --------------------------------------------------------------- Swapchain creation start.
		let (image_format, image_color_space) = surface
			.loader
			.get_physical_device_surface_formats(physical_device.raw_handle, surface.raw_handle)
			.unwrap()
			.iter()
			.map(|format| {
				match format.format {
					vk::Format::UNDEFINED => (
						vk::Format::R8G8B8A8_UNORM,
						vk::ColorSpaceKHR::EXTENDED_SRGB_NONLINEAR_EXT
					),
					_ => (format.format, format.color_space),
				}
			})
			.next()
			.unwrap();

		let present_mode = {
			let available_present_modes = surface
				.loader
				.get_physical_device_surface_present_modes(
					physical_device.raw_handle,
					surface.raw_handle,
				)
				.unwrap();

			*available_present_modes
				.iter()
				.find(|&&mode| mode == vk::PresentModeKHR::MAILBOX)
				.unwrap_or(available_present_modes.iter().next().unwrap())
		};

		let surface_capabilities = surface
			.loader
			.get_physical_device_surface_capabilities(
				physical_device.raw_handle,
				surface.raw_handle,
			)
			.unwrap();

		let image_extent = surface_capabilities.current_extent;

		let image_count = match surface_capabilities.max_image_count {
			0 => surface_capabilities.min_image_count + 1,
			max @ _ => if surface_capabilities.min_image_count <= max {
				surface_capabilities.min_image_count + 1
			} else {
				surface_capabilities.min_image_count
			},
		};

		let swapchain_info = vk::SwapchainCreateInfoKHR {
			s_type: StructureType::SWAPCHAIN_CREATE_INFO_KHR,
			p_next: ptr::null(),
			flags: vk::SwapchainCreateFlagsKHR::empty(),
			surface: surface.raw_handle,
			min_image_count: image_count,
			image_format,
			image_color_space,
			image_extent,
			image_array_layers: 1,
			image_usage: vk::ImageUsageFlags::COLOR_ATTACHMENT,
			image_sharing_mode: vk::SharingMode::EXCLUSIVE,
			queue_family_index_count: 0,
			p_queue_family_indices: ptr::null(),
			pre_transform: surface_capabilities.current_transform,
			composite_alpha: vk::CompositeAlphaFlagsKHR::OPAQUE,
			present_mode,
			clipped: vk::TRUE,
			old_swapchain: vk::SwapchainKHR::null(),
		};

		let swapchain = {
			let data = SwapchainData {
				min_image_count: swapchain_info.min_image_count,
				format: swapchain_info.image_format,
				color_space: swapchain_info.image_color_space,
				extent: swapchain_info.image_extent,
				array_layers: swapchain_info.image_array_layers,
				usage: swapchain_info.image_usage,
				sharing_mode: swapchain_info.image_sharing_mode,
				queue_family_index_count: swapchain_info.queue_family_index_count,
				p_queue_family_indices: swapchain_info.p_queue_family_indices,
				pre_transform: swapchain_info.pre_transform,
				present_mode: swapchain_info.present_mode,
				composite_alpha: swapchain_info.composite_alpha,
				clipped: swapchain_info.clipped,
			};

			let loader = khr::Swapchain::new(&instance, &device);

			let raw_handle = loader
				.create_swapchain(&swapchain_info, None)
				.expect("Swap chain creation has failed.");

			let images = loader
				.get_swapchain_images(raw_handle)
				.expect("Failed to get swap chain images.");

			let images = images
				.into_iter()
				.map(|image| {
					let image_view_info = vk::ImageViewCreateInfo {
						s_type: StructureType::IMAGE_VIEW_CREATE_INFO,
						p_next: ptr::null(),
						flags: vk::ImageViewCreateFlags::empty(),
						image,
						view_type: vk::ImageViewType::TYPE_2D,
						format: image_format,
						components: vk::ComponentMapping {
							r: vk::ComponentSwizzle::IDENTITY,
							g: vk::ComponentSwizzle::IDENTITY,
							b: vk::ComponentSwizzle::IDENTITY,
							a: vk::ComponentSwizzle::IDENTITY,
						},
						subresource_range: vk::ImageSubresourceRange {
							aspect_mask: vk::ImageAspectFlags::COLOR,
							base_mip_level: 0,
							level_count: 1,
							base_array_layer: 0,
							layer_count: 1,
						},
					};

					let image_view = device.create_image_view(&image_view_info, None).unwrap();

					(image, image_view)
				})
				.collect::<Vec<_>>();

			Swapchain {
				loader,
				raw_handle,
				images,
				data,
			}
		};
		// ----------------------------------------------------------------- Swapchain creation end.

		// ------------------------------------------------------------- Render pass creation start.
		let attachments = [
			vk::AttachmentDescription {
				flags: vk::AttachmentDescriptionFlags::empty(),
				format: swapchain.data.format,
				samples: vk::SampleCountFlags::TYPE_1,
				load_op: vk::AttachmentLoadOp::CLEAR,
				store_op: vk::AttachmentStoreOp::STORE,
				stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
				stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
				initial_layout: vk::ImageLayout::UNDEFINED,
				final_layout: vk::ImageLayout::PRESENT_SRC_KHR,
			},
		];

		let color_attachment_ref = [
			vk::AttachmentReference {
				attachment: 0,
				layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
			}
		];

		let subpasses = [
			vk::SubpassDescription {
				flags: vk::SubpassDescriptionFlags::empty(),
				pipeline_bind_point: vk::PipelineBindPoint::GRAPHICS,
				color_attachment_count: color_attachment_ref.len() as u32,
				p_color_attachments: color_attachment_ref.as_ptr(),
				input_attachment_count: 0,
				p_input_attachments: ptr::null(),
				p_resolve_attachments: ptr::null(),
				p_depth_stencil_attachment: ptr::null(),
				preserve_attachment_count: 0,
				p_preserve_attachments: ptr::null(),
			}
		];

		let subpass_dependencies = [
			vk::SubpassDependency {
				src_subpass: vk::SUBPASS_EXTERNAL,
				dst_subpass: 0,
				src_access_mask: vk::AccessFlags::empty(),
				dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_WRITE
					| vk::AccessFlags::COLOR_ATTACHMENT_READ,
				src_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
				dst_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
				dependency_flags: vk::DependencyFlags::BY_REGION,
			},
		];

		let render_pass_info = vk::RenderPassCreateInfo {
			s_type: StructureType::RENDER_PASS_CREATE_INFO,
			p_next: ptr::null(),
			flags: vk::RenderPassCreateFlags::empty(),
			attachment_count: attachments.len() as u32,
			p_attachments: attachments.as_ptr(),
			subpass_count: subpasses.len() as u32,
			p_subpasses: subpasses.as_ptr(),
			dependency_count: subpass_dependencies.len() as u32,
			p_dependencies: subpass_dependencies.as_ptr(),
		};

		let render_pass = device
			.create_render_pass(&render_pass_info, None)
			.expect("Failed to create render pass.");
		// --------------------------------------------------------------- Render pass creation end.

		let shaders = Shaders::load(&device, render_pass);

		// ------------------------------------------------------------ Framebuffers creation start.
		let mut framebuffers = Vec::with_capacity(swapchain.images.len());
		for (_, image_view) in swapchain.images.iter() {
			let framebuffer_info = vk::FramebufferCreateInfo {
				s_type: StructureType::FRAMEBUFFER_CREATE_INFO,
				p_next: ptr::null(),
				flags: vk::FramebufferCreateFlags::empty(),
				render_pass,
				attachment_count: 1,
				p_attachments: image_view as *const _,
				width: swapchain.data.extent.width,
				height: swapchain.data.extent.height,
				layers: 1,
			};

			framebuffers.push(
				device
					.create_framebuffer(&framebuffer_info, None)
					.expect("Failed to create a framebuffer."),
			);
		}
		// -------------------------------------------------------------- Framebuffers creation end.

		// ----------------------------------------------------------------- Command creation start.
		let command = {
			let command_pool_info = vk::CommandPoolCreateInfo {
				s_type: StructureType::COMMAND_POOL_CREATE_INFO,
				p_next: ptr::null(),
				flags: vk::CommandPoolCreateFlags::TRANSIENT
					| vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
				queue_family_index: physical_device.queue_family_index,
			};

			let pool = device
				.create_command_pool(&command_pool_info, None)
				.expect("Command pool creation has failed.");

			let command_buffers_allocation_info = vk::CommandBufferAllocateInfo {
				s_type: StructureType::COMMAND_BUFFER_ALLOCATE_INFO,
				p_next: ptr::null(),
				command_pool: pool,
				level: vk::CommandBufferLevel::PRIMARY,
				command_buffer_count: framebuffers.len() as u32,
			};

			let buffers = device
				.allocate_command_buffers(&command_buffers_allocation_info)
				.expect("Allocating command buffers failed.");

			Commandbuffers { pool, raw_buffers: buffers }
		};

		// ----------------------------------------------------- Semaphore and Fence creation start.
		let semaphores = Semaphores {
			image_acquired: create_semaphore(&device),
			render_finished: (0..swapchain.images.len())
				.map(|_| {
					create_semaphore(&device)
				})
				.collect(),
		};
		// ------------------------------------------------------- Semaphore and Fence creation end.

		dbg!("successfully return.");

		Self {
			entry,
			instance,
			surface,
			physical_device,
			device,
			queue,
			swapchain,
			shaders,
			render_pass,
			framebuffers,
			command,
			semaphores,
		}
	}

	unsafe fn deal_with_window_resize_unsafe(&mut self) {
		// Wait for finish commands in flight
		self.queue_wait_idle();

		// Drop unavailable resources.
		self.framebuffers.iter().for_each(|&framebuffer| {
			self.device.destroy_framebuffer(framebuffer, None);
		});

		self.swapchain.images.iter().for_each(|&(_, image_view)| {
			self.device.destroy_image_view(image_view, None);
		});

		// Query new physical surface extent.
		let surface_capabilities = self
			.surface
			.loader
			.get_physical_device_surface_capabilities(
				self.physical_device.raw_handle,
				self.surface.raw_handle,
			)
			.expect("Failed to get new surface capabilities.");

		self.swapchain.data.extent = surface_capabilities.current_extent;

		// Create new resources.
		let swapchain_info = vk::SwapchainCreateInfoKHR {
			// New data.
			image_extent: self.swapchain.data.extent,
			// Inherit old swapcahin info.
			s_type: StructureType::SWAPCHAIN_CREATE_INFO_KHR,
			p_next: ptr::null(),
			flags: vk::SwapchainCreateFlagsKHR::empty(),
			surface: self.surface.raw_handle,
			min_image_count: self.swapchain.data.min_image_count,
			image_format: self.swapchain.data.format,
			image_color_space: self.swapchain.data.color_space,
			image_array_layers: self.swapchain.data.array_layers,
			image_usage: self.swapchain.data.usage,
			image_sharing_mode: self.swapchain.data.sharing_mode,
			queue_family_index_count: self.swapchain.data.queue_family_index_count,
			p_queue_family_indices: self.swapchain.data.p_queue_family_indices,
			present_mode: self.swapchain.data.present_mode,
			pre_transform: self.swapchain.data.pre_transform,
			composite_alpha: self.swapchain.data.composite_alpha,
			clipped: self.swapchain.data.clipped,
			old_swapchain: self.swapchain.raw_handle,
		};

		let new_swapchain_raw_handle = self
			.swapchain
			.loader
			.create_swapchain(&swapchain_info, None)
			.expect("Failed to recreate swapchain.");

		self.swapchain
			.loader
			.destroy_swapchain(self.swapchain.raw_handle, None);
		// Make sure that creating new swapchain has finished, then destory previous swapcahin.
		self.swapchain.raw_handle = new_swapchain_raw_handle;

		self.swapchain.images = self
			.swapchain
			.loader
			.get_swapchain_images(self.swapchain.raw_handle)
			.expect("Failed to get handle of recreated swapchain images.")
			.into_iter()
			.map(|image| {
				let image_view_info = vk::ImageViewCreateInfo {
					s_type: StructureType::IMAGE_VIEW_CREATE_INFO,
					p_next: ptr::null(),
					flags: vk::ImageViewCreateFlags::empty(),
					image,
					view_type: vk::ImageViewType::TYPE_2D,
					format: self.swapchain.data.format,
					components: vk::ComponentMapping {
						r: vk::ComponentSwizzle::IDENTITY,
						g: vk::ComponentSwizzle::IDENTITY,
						b: vk::ComponentSwizzle::IDENTITY,
						a: vk::ComponentSwizzle::IDENTITY,
					},
					subresource_range: vk::ImageSubresourceRange {
						aspect_mask: vk::ImageAspectFlags::COLOR,
						base_mip_level: 0,
						level_count: 1,
						base_array_layer: 0,
						layer_count: 1,
					},
				};

				let image_view = self
					.device
					.create_image_view(&image_view_info, None)
					.expect("Failed to create image view.");

				(image, image_view)
			})
			.collect();

		self.framebuffers = self
			.swapchain
			.images
			.iter()
			.map(|(_, image_view)| {
				let framebuffer_info = vk::FramebufferCreateInfo {
					s_type: StructureType::FRAMEBUFFER_CREATE_INFO,
					p_next: ptr::null(),
					flags: vk::FramebufferCreateFlags::empty(),
					render_pass: self.render_pass,
					attachment_count: 1,
					p_attachments: image_view as *const _,
					width: self.swapchain.data.extent.width,
					height: self.swapchain.data.extent.height,
					layers: 1,
				};

				self.device
					.create_framebuffer(&framebuffer_info, None)
					.expect("Failed to create framebuffer")
			})
			.collect();
	}
}

impl Drop for Vulkan {
	fn drop(&mut self) {
		unsafe {
			self.device.destroy_semaphore(self.semaphores.image_acquired, None);
			self.semaphores.render_finished
				.iter()
				.for_each(|&semaphore| {
					self.device.destroy_semaphore(semaphore, None);
				});

			self.device.destroy_command_pool(self.command.pool, None);

			for framebuffer in self.framebuffers.clone().into_iter() {
				self.device.destroy_framebuffer(framebuffer, None);
			}

			self.device.destroy_render_pass(self.render_pass, None);
			self.device.destroy_pipeline(self.shaders.gui.pipeline, None);
			self.device.destroy_pipeline_layout(self.shaders.gui.pipeline_layout, None);
			self.device.destroy_descriptor_set_layout(self.shaders.gui.descriptor_set_layout, None);

			self.swapchain
				.images
				.iter()
				.for_each(|&(_, image_view)| self.device.destroy_image_view(image_view, None));

			self.swapchain
				.loader
				.destroy_swapchain(self.swapchain.raw_handle, None);

			self.surface
				.loader
				.destroy_surface(self.surface.raw_handle, None);

			self.device.destroy_device(None);
			self.instance.destroy_instance(None);
		}
	}
}

#[cfg(target_os = "windows")]
fn instance_extensions() -> Vec<*const i8> {
	vec![
		khr::Surface::name().as_ptr(),
		khr::Win32Surface::name().as_ptr(),
	]
}

#[cfg(target_os = "windows")]
unsafe fn surface(entry: &Entry, instance: &Instance, window: &winit::Window) -> vk::SurfaceKHR {
	use winapi::um::libloaderapi::GetModuleHandleW;
	use winit::os::windows::WindowExt;

	let info = vk::Win32SurfaceCreateInfoKHR {
		s_type: StructureType::WIN32_SURFACE_CREATE_INFO_KHR,
		p_next: ptr::null(),
		flags: vk::Win32SurfaceCreateFlagsKHR::from_raw(0),
		hwnd: window.get_hwnd() as *const _,
		hinstance: GetModuleHandleW(ptr::null()) as *const _,
	};

	khr::Win32Surface::new(entry, instance)
		.create_win32_surface(&info, None)
		.expect("Failed to create win32 surface.")
}

unsafe fn create_semaphore(device: &Device) -> vk::Semaphore {
	device
		.create_semaphore(
			&vk::SemaphoreCreateInfo {
				s_type: StructureType::SEMAPHORE_CREATE_INFO,
				p_next: ptr::null(),
				flags: vk::SemaphoreCreateFlags::empty(),
			},
			None,
		)
		.unwrap()
}
