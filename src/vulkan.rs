mod mem;
mod shaders;
mod cmd_buf;

pub use self::shaders::*;
pub use self::mem::*;
pub use self::cmd_buf::*;

use crate::linear_algebra::*;

use ash::vk;
use ash::vk::StructureType;
use ash::vk_make_version;
use ash::extensions::khr;
use ash::Entry;
use ash::Instance;
use ash::Device;
use ash::version::EntryV1_0;
use ash::version::InstanceV1_0;
use ash::version::DeviceV1_0;

use winit::Window;

use std::ptr;
use std::ffi::CString;
use std::default::Default;
use std::marker::PhantomData;
use std::ops::Range;

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
	_entry: Entry,
	instance: Instance,
	device: Device,
	queue: vk::Queue,
	physical_device: PhysicalDevice,
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
	min_image_count: u32,
	color_space: vk::ColorSpaceKHR,
	present_mode: vk::PresentModeKHR,
}

pub struct VkGraphic<'vk_core> {
	vk_core: &'vk_core VkCore,
	swapchain: Swapchain,
	render_pass: vk::RenderPass,
	shaders: Shaders,
	framebuffers: Vec<vk::Framebuffer>,
}

pub struct VkSampler<'vk_core> {
	vk_core: &'vk_core VkCore,
	raw_handle: vk::Sampler,
}

pub struct VkFence<'vk_core> {
	vk_core: &'vk_core VkCore,
	raw_handle: vk::Fence,
}

/// vk::Semaphore is often used in type slice.
/// This struct should be destroyed explicitly.
#[derive(Copy, Clone)]
pub struct VkSemaphore<'vk_core> {
	raw_handle: vk::Semaphore,
	_marker: PhantomData<&'vk_core VkCore>,
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
				_entry: entry,
				instance,
				physical_device,
				device,
				queue,
				surface,
			}
		}
	}

	pub fn queue_wait_idle(&self) -> Result<(), vk::Result> {
		unsafe { self.device.queue_wait_idle(self.queue) }
	}

	pub fn memory_properties(&self) -> vk::PhysicalDeviceMemoryProperties {
		self.physical_device.memory_properties
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

	#[inline]
	fn images_num(&self) -> usize { self.images.len() }
}

impl<'vk_core> VkGraphic<'vk_core> {
	pub fn new(vk_core: &'vk_core VkCore) -> Self {
		unsafe {
			// create swapchain
			let (format, color_space) = vk_core.surface
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

			let requested_image_count = surface_capabilities.min_image_count + 1;
			let image_count = if requested_image_count > surface_capabilities.max_image_count
				&& surface_capabilities.max_image_count > 0
			{
				surface_capabilities.max_image_count
			} else {
				requested_image_count
			};

			let data = SwapchainData {
				min_image_count: image_count,
				format,
				color_space,
				extent: surface_capabilities.current_extent,
				present_mode,
			};

			let loader = khr::Swapchain::new(&vk_core.instance, &vk_core.device);
			let (raw_handle, images) = Self::create_swapchain(&vk_core, &loader, &data, None)
				.unwrap();
			let swapchain = Swapchain { loader, raw_handle, images, data };

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
			let width_height_ratio = {
				let extent = &swapchain.data.extent;
				extent.height as f32 / extent.width as f32
			};
			let shaders = Shaders::load(&vk_core.device, render_pass, swapchain.data.extent).unwrap();

			// create framebuffer
			let framebuffers = Self::create_framebuffers(
				&vk_core.device,
				render_pass,
				&swapchain,
			).unwrap();

			Self {
				vk_core,
				swapchain,
				render_pass,
				shaders,
				framebuffers,
			}
		}
	}

	pub fn next_image(
		&self,
		semaphore: Option<&VkSemaphore>,
		fence: Option<&VkFence>,
	) -> Result<usize, vk::Result> {
		unsafe {
			let index = self.swapchain.loader
				.acquire_next_image(
					self.swapchain.raw_handle,
					!0,
					semaphore.map(|s| s.raw_handle).unwrap_or(vk::Semaphore::null()),
					fence.map(|s| s.raw_handle).unwrap_or(vk::Fence::null()),
				)
				.map(|(index, _)| index as usize)?;

			Ok(index)
		}
	}

	pub fn present(
		&self,
		ref image_index: u32,
		semaphores: &[VkSemaphore],
	) -> Result<(), vk::Result> {
		unsafe {
			self.swapchain.loader
				.queue_present(
					self.vk_core.queue,
					&vk::PresentInfoKHR {
						s_type: StructureType::PRESENT_INFO_KHR,
						p_next: ptr::null(),
						wait_semaphore_count: semaphores.len() as _,
						p_wait_semaphores: semaphores.as_ptr() as *const _,
						swapchain_count: 1,
						p_swapchains: &self.swapchain.raw_handle as *const _,
						p_image_indices: image_index as *const _,
						p_results: ptr::null_mut(),
					},
				)?;

			Ok(())
		}
	}

	pub fn recreate(&mut self) -> Result<(), vk::Result> {
		unsafe {
			// recreate swapcahin
			let surface_capabilities = self.vk_core.surface.loader
				.get_physical_device_surface_capabilities(
					self.vk_core.physical_device.raw_handle,
					self.vk_core.surface.raw_handle,
				)?;
			self.swapchain.data.extent = surface_capabilities.current_extent;

			self.swapchain.images
				.iter()
				.for_each(|&(_, view)| self.vk_core.device.destroy_image_view(view, None));

			let (raw_handle, images) = Self::create_swapchain(
				&self.vk_core,
				&self.swapchain.loader,
				&self.swapchain.data,
				Some(self.swapchain.raw_handle),
			)?;
			self.swapchain.raw_handle = raw_handle;
			self.swapchain.images = images;

			// recreate framebuffers
			self.framebuffers
				.iter()
				.for_each(|&fb| self.vk_core.device.destroy_framebuffer(fb, None));
			self.framebuffers = Self::create_framebuffers(
				&self.vk_core.device,
				self.render_pass,
				&self.swapchain,
			)?;

			// recreate graphic pipelines
			self.shaders
				.reload(&self.vk_core.device, self.render_pass, self.swapchain.data.extent)?;

			Ok(())
		}
	}

	#[inline]
	pub fn images_num(&self) -> usize { self.swapchain.images_num() }

	fn create_swapchain(
		vk_core: &VkCore,
		loader: &khr::Swapchain,
		data: &SwapchainData,
		old_swapchain: Option<vk::SwapchainKHR>,
	) -> Result<(vk::SwapchainKHR, Vec<(vk::Image, vk::ImageView)>), vk::Result> {
		unsafe {
			let swapchain_info = vk::SwapchainCreateInfoKHR {
				s_type: StructureType::SWAPCHAIN_CREATE_INFO_KHR,
				p_next: ptr::null(),
				flags: vk::SwapchainCreateFlagsKHR::empty(),
				surface: vk_core.surface.raw_handle,
				min_image_count: data.min_image_count,
				image_format: data.format,
				image_color_space: data.color_space,
				image_extent: data.extent,
				image_array_layers: 1,
				image_usage: vk::ImageUsageFlags::COLOR_ATTACHMENT,
				image_sharing_mode: vk::SharingMode::EXCLUSIVE,
				queue_family_index_count: vk_core.physical_device.queue_family_index_count(),
				p_queue_family_indices: vk_core.physical_device.queue_family_index_ptr(),
				pre_transform: vk::SurfaceTransformFlagsKHR::IDENTITY,
				composite_alpha: vk::CompositeAlphaFlagsKHR::OPAQUE,
				present_mode: data.present_mode,
				clipped: vk::TRUE,
				old_swapchain: old_swapchain.unwrap_or(vk::SwapchainKHR::null()),
			};

			let raw_handle = loader
				.create_swapchain(&swapchain_info, None)
				.expect("Swap chain creation has failed.");

			old_swapchain.map(|raw_handle| loader.destroy_swapchain(raw_handle, None));

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
						format: data.format,
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

			Ok((raw_handle, images))
		}
	}

	fn create_framebuffers(
		device: &Device,
		render_pass: vk::RenderPass,
		swapchain: &Swapchain,
	) -> Result<Vec<vk::Framebuffer>, vk::Result> {
		unsafe {
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

				framebuffers.push(device.create_framebuffer(&framebuffer_info, None)?);
			}

			Ok(framebuffers)
		}
	}
}

impl Drop for VkGraphic<'_> {
	fn drop(&mut self) {
		unsafe {
			let device = &self.vk_core.device;
			self.framebuffers
				.iter()
				.for_each(|&framebuffer| device.destroy_framebuffer(framebuffer, None));
			device.destroy_render_pass(self.render_pass, None);
			self.swapchain.images
				.iter()
				.for_each(|&(_image, view)| {
					// image must not be destroyed by device.destroy_image().
					// image will destroyed by destroying swapchain.
					self.vk_core.device.destroy_image_view(view, None);
				});
			self.swapchain.loader.destroy_swapchain(self.swapchain.raw_handle, None);
			device.destroy_pipeline(self.shaders.gui.pipeline, None);
			device.destroy_pipeline_layout(self.shaders.gui.pipeline_layout, None);
			device.destroy_descriptor_set_layout(self.shaders.gui.descriptor_set_layout, None);
			device.destroy_pipeline(self.shaders.d3.pipeline, None);
			device.destroy_pipeline_layout(self.shaders.d3.pipeline_layout, None);
			device.destroy_descriptor_set_layout(self.shaders.d3.descriptor_set_layout, None);
		}
	}
}

impl<'vk_core> VkSampler<'vk_core> {
	pub fn new(
		vk_core: &'vk_core VkCore,
		(min_filter, mag_filter): (vk::Filter, vk::Filter),
		address_mode_u: vk::SamplerAddressMode,
		address_mode_v: vk::SamplerAddressMode,
		address_mode_w: vk::SamplerAddressMode,
		border_color: vk::BorderColor,
		mipmap_mode: vk::SamplerMipmapMode,
		mip_lod_bias: f32,
		lod_range: Range<f32>,
		anisotropy: Option<f32>,
		compare: Option<vk::CompareOp>,
		unnormalized_coordinates: vk::Bool32,
	) -> Result<Self, vk::Result> {
		unsafe {
			let raw_handle = vk_core.device
				.create_sampler(
					&vk::SamplerCreateInfo {
						s_type: StructureType::SAMPLER_CREATE_INFO,
						p_next: ptr::null(),
						flags: vk::SamplerCreateFlags::empty(),
						min_filter,
						mag_filter,
						address_mode_u,
						address_mode_v,
						address_mode_w,
						border_color,
						mipmap_mode,
						mip_lod_bias,
						min_lod: lod_range.start,
						max_lod: lod_range.end,
						anisotropy_enable: anisotropy.map(|_| vk::TRUE).unwrap_or(vk::FALSE),
						max_anisotropy: anisotropy.unwrap_or(1.0),
						compare_enable: compare.map(|_| vk::TRUE).unwrap_or(vk::FALSE),
						compare_op: compare.unwrap_or(vk::CompareOp::NEVER),
						unnormalized_coordinates,
					},
					None,
				)?;

			Ok(Self { vk_core, raw_handle })
		}
	}
}

impl Drop for VkSampler<'_> {
	fn drop(&mut self) { unsafe { self.vk_core.device.destroy_sampler(self.raw_handle, None); } }
}

impl<'vk_core> VkFence<'vk_core> {
	pub fn new(vk_core: &'vk_core VkCore, signaled: bool) -> Result<Self, vk::Result> {
		unsafe {
			let raw_handle = vk_core.device
				.create_fence(
					&vk::FenceCreateInfo {
						s_type: StructureType::FENCE_CREATE_INFO,
						p_next: ptr::null(),
						flags: if signaled {
							vk::FenceCreateFlags::SIGNALED
						} else {
							vk::FenceCreateFlags::empty()
						},
					},
					None,
				)?;

			Ok(Self { vk_core, raw_handle })
		}
	}

	pub fn reset(&mut self) -> Result<(), vk::Result> {
		unsafe { self.vk_core.device.reset_fences(&[self.raw_handle])? }
		Ok(())
	}

	pub fn wait(&self, timeout: Option<u64>) -> Result<(), vk::Result> {
		unsafe {
			self.vk_core.device.wait_for_fences(&[self.raw_handle], false, timeout.unwrap_or(!0))?;
			self.vk_core.device.reset_fences(&[self.raw_handle])?;
			Ok(())
		}
	}
}

impl Drop for VkFence<'_> {
	fn drop(&mut self) { unsafe { self.vk_core.device.destroy_fence(self.raw_handle, None); } }
}

impl<'vk_core> VkSemaphore<'vk_core> {
	pub fn new(vk_core: &'vk_core VkCore) -> Result<Self, vk::Result> {
		unsafe {
			let raw_handle = vk_core.device
				.create_semaphore(
					&vk::SemaphoreCreateInfo {
						s_type: StructureType::SEMAPHORE_CREATE_INFO,
						p_next: ptr::null(),
						flags: vk::SemaphoreCreateFlags::empty(),
					},
					None,
				)?;

			Ok(Self { raw_handle, _marker: PhantomData, })
		}
	}

	pub fn drop(self, vk_core: &VkCore) {
		unsafe {
			vk_core.device.destroy_semaphore(self.raw_handle, None);
			std::mem::forget(self);
		}
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

