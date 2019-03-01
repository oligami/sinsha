mod buf_image_mem;
mod cmd_buf;
mod shaders;

pub use crate::vulkan::buf_image_mem::{Buffer, Image, MemoryBlock};

use crate::linear_algebra::*;
use crate::vulkan::shaders::*;
use crate::vulkan::buf_image_mem::*;

use ash::version::EntryV1_0;
use ash::version::InstanceV1_0;
use ash::version::DeviceV1_0;
use ash::extensions::khr;
use ash::vk;
use ash::vk::StructureType;
use ash::vk_make_version;
use ash::Entry;
use ash::Instance;
use ash::Device;

use winit::Window;

use std::ptr;
use std::ffi::CString;
use std::default::Default;


pub struct PhysicalDevice {
	pub raw_handle: vk::PhysicalDevice,
	pub memory_properties: vk::PhysicalDeviceMemoryProperties,
	pub queue_family_index: u32,
}

struct Surface {
	loader: khr::Surface,
	raw_handle: vk::SurfaceKHR,
}

pub struct VkCore {
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
	extent: vk::Extent2D,
	format: vk::Format,
	usage: vk::ImageUsageFlags,
	min_image_count: u32,
	color_space: vk::ColorSpaceKHR,
	sharing_mode: vk::SharingMode,
	pre_transform: vk::SurfaceTransformFlagsKHR,
	composite_alpha: vk::CompositeAlphaFlagsKHR,
	present_mode: vk::PresentModeKHR,
	clipped: vk::Bool32,
}

pub struct VkGraphic<'vk_core> {
	vk_core: &'vk_core VkCore,
	swapchain: Swapchain,
	render_pass: vk::RenderPass,
	shaders: Shaders,
	framebuffers: Vec<vk::Framebuffer>,
}

impl PhysicalDevice {
	pub fn queue_family_index_count(&self) -> u32 { 1 }
	pub fn queue_family_index_ptr(&self) -> *const u32 { &self.queue_family_index as *const _ }
}

impl VkCore {
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

impl Drop for VkCore {
	fn drop(&mut self) {
		unsafe {
			self.surface.loader.destroy_surface(self.surface.raw_handle, None);
			self.device.destroy_device(None);
			self.instance.destroy_instance(None);
		}
	}
}

impl Swapchain {
	pub fn render_extent(&self) -> XY {
		XY::new(self.data.extent.width as f32, self.data.extent.height as f32)
	}
}

impl<'vk_core> VkGraphic<'vk_core> {
	pub fn new(vk_core: &'vk_core VkCore) -> Self {
		unsafe {
			// create swapchain
			let (image_format, image_color_space) = vk_core.surface
				.loader
				.get_physical_device_surface_formats(
					vk_core.physical_device.raw_handle,
					vk_core.surface.raw_handle
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
				let available_present_modes = vk_core.surface
					.loader
					.get_physical_device_surface_present_modes(
						vk_core.physical_device.raw_handle,
						vk_core.surface.raw_handle,
					)
					.unwrap();

				available_present_modes
					.iter()
					.find(|&&mode| mode == vk::PresentModeKHR::MAILBOX)
					.map(|mode| *mode)
					.unwrap_or(available_present_modes[0])
			};

			let surface_capabilities = vk_core.surface
				.loader
				.get_physical_device_surface_capabilities(
					vk_core.physical_device.raw_handle,
					vk_core.surface.raw_handle,
				)
				.unwrap();

			let image_extent = surface_capabilities.current_extent;

			let requested_image_count = surface_capabilities.min_image_count + 1;
			let image_count = if requested_image_count > surface_capabilities.max_image_count
				&& surface_capabilities.max_image_count > 0
			{
				surface_capabilities.max_image_count
			} else {
				requested_image_count
			};

			let swapchain_info = vk::SwapchainCreateInfoKHR {
				s_type: StructureType::SWAPCHAIN_CREATE_INFO_KHR,
				p_next: ptr::null(),
				flags: vk::SwapchainCreateFlagsKHR::empty(),
				surface: vk_core.surface.raw_handle,
				min_image_count: image_count,
				image_format,
				image_color_space,
				image_extent,
				image_array_layers: 1,
				image_usage: vk::ImageUsageFlags::COLOR_ATTACHMENT,
				image_sharing_mode: vk::SharingMode::EXCLUSIVE,
				queue_family_index_count: vk_core.physical_device.queue_family_index_count(),
				p_queue_family_indices: vk_core.physical_device.queue_family_index_ptr(),
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
					usage: swapchain_info.image_usage,
					sharing_mode: swapchain_info.image_sharing_mode,
					pre_transform: swapchain_info.pre_transform,
					present_mode: swapchain_info.present_mode,
					composite_alpha: swapchain_info.composite_alpha,
					clipped: swapchain_info.clipped,
				};

				let loader = khr::Swapchain::new(&vk_core.instance, &vk_core.device);

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

						let image_view = vk_core.device
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

			let render_pass = vk_core.device
				.create_render_pass(&render_pass_info, None)
				.expect("Failed to create render pass.");

			// create shaders
			let shaders = Shaders::load(&vk_core.device, render_pass);

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
					vk_core.device
						.create_framebuffer(&framebuffer_info, None)
						.expect("Failed to create a framebuffer."),
				);
			}

			Self {
				vk_core,
				swapchain,
				render_pass,
				shaders,
				framebuffers,
			}
		}
	}
}

impl Drop for VkGraphic<'_> {
	fn drop(&mut self) {
		unsafe {
			eprintln!("Dropping VkGraphic.");
			eprintln!("Dropping Framebuffers.");
			self.framebuffers
				.iter()
				.for_each(|&framebuffer| {
					self.vk_core.device.destroy_framebuffer(framebuffer, None);
				});
			eprintln!("Dropping Render pass.");
			self.vk_core.device.destroy_render_pass(self.render_pass, None);
			eprintln!("Dropping Swapchain images and views.");
			self.swapchain.images
				.iter()
				.for_each(|&(_image, view)| {
					// image must not be destroyed by device.destroy_image().
					// image will destroyed by destroying swapchain.
					self.vk_core.device.destroy_image_view(view, None);
				});
			eprintln!("Dropping SwapchainKHR.");
			self.swapchain.loader.destroy_swapchain(self.swapchain.raw_handle, None);
//			self.vk_core.device.destroy_pipeline(self.shaders.gui.pipeline, None);
//			self.vk_core.device.destroy_pipeline_layout(self.shaders.gui.pipeline_layout, None);
//			self.vk_core.device
//				.destroy_descriptor_set_layout(self.shaders.gui.descriptor_set_layout, None);
		}
		eprintln!("Dropped VkGraphic.");
	}
}

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