pub mod mem_kai;
pub mod render_pass;
pub mod swapchain;
pub mod framebuffer;
pub mod shader;

pub use self::mem_kai::alloc;

use crate::linear_algebra::*;

use ash::vk;
use ash::vk::StructureType;
use ash::vk_make_version;
use ash::extensions::khr;
use ash::Entry;
use ash::Instance as VkInstance;
use ash::Device as VkDevice;
use ash::version::EntryV1_0;
use ash::version::InstanceV1_0;
use ash::version::DeviceV1_0;

use winit::Window;

use std::ptr;
use std::ffi::CString;
use std::default::Default;
use std::marker::PhantomData;
use std::ops::Range;
use std::sync::Arc;

pub trait Destroy: Sized {
	type Error: std::error::Error;

	/// Destroy Vulkan API objects without checking.
	///
	/// # Safety
	/// Be careful for objects used by GPU such as Buffer, Image, DescriptorSet, etc.
	/// This method can cause memory violence very easily.
	///
	/// Vulkan API says: "Host access to objects must be externally synchronized."
	/// This is satisfied by taking ownership. It's also prevent from double-freeing.
	unsafe fn destroy(self) -> Result<(), Self::Error>;


	/// Almost all objects are used in std::sync::Arc.
	/// If strong count of Arc is not 1, this method will fail to destroying and return error.
	/// If strong count of Arc is 1, then self will be destroyed.
	/// This method is just for convenience.
	unsafe fn try_destroy(self: Arc<Self>) -> Result<(), DestroyError<Self::Error>> {
		let obj = Arc::try_unwrap(self).map_err(|_| DestroyError::NonZeroStrongCount)?;
		obj.destroy().map_err(|e| DestroyError::Specific(e))
	}
}

/// Many kinds of objects in Vulkan API are definitely success destroying.
/// Destroying some objects may violate memory safety, but destroying will succeed anyway.
///
/// This enum should become type alias when !(never_type) has stabilized.
#[derive(Copy)]
pub enum Infallible {}

impl std::fmt::Display for Infallible {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match *self {}
	}
}

impl std::fmt::Debug for Infallible {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match *self {}
	}
}

impl Clone for Infallible {
	fn clone(&self) -> Self {
		match *self {}
	}
}

impl std::error::Error for Infallible {}


/// TODO: Should have Arc<Obj> and reconsider variant's names.
pub enum DestroyError<E> {
	NonZeroStrongCount,
	Specific(E),
}

impl<E> std::fmt::Display for DestroyError<E> {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		unimplemented!()
	}
}

impl<E> std::fmt::Debug for DestroyError<E> {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		unimplemented!()
	}
}

impl<E> std::error::Error for DestroyError<E> {}


pub struct Instance {
	entry: Entry,
	handle: VkInstance,
	physical_devices: Vec<PhysicalDevice>,
}

pub struct PhysicalDevice {
	handle: vk::PhysicalDevice,
	memory_types: Vec<vk::MemoryType>,
	memory_heaps: Vec<vk::MemoryHeap>,
}

pub struct SurfaceKHR {
	instance: Arc<Instance>,
	loader: khr::Surface,
	handle: vk::SurfaceKHR,
	window: Window,
}

pub struct Device {
	instance: Arc<Instance>,
	physical_device_index: usize,
	handle: VkDevice,
}

pub struct Queue<C> {
	device: Arc<Device>,
	handle: vk::Queue,
	family_index: u32,
	_type: PhantomData<C>,
}

pub struct Graphics;
pub struct Compute;
pub struct Transfer;

impl Instance {
	pub fn new() -> Arc<Self> {
		let entry = Entry::new().unwrap();

		let handle = {
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

			let instance_extensions = Self::extensions();
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

			unsafe { entry.create_instance(&instance_info, None).unwrap() }
		};

		let physical_devices = {
			unsafe { handle.enumerate_physical_devices().unwrap() }
				.into_iter()
				.map(|vk_physical_device| {
					let memory_properties = unsafe {
						handle.get_physical_device_memory_properties(vk_physical_device)
					};

					let memory_types = memory_properties
						.memory_types[..memory_properties.memory_type_count as usize]
						.to_vec();

					let memory_heaps = memory_properties
						.memory_heaps[..memory_properties.memory_heap_count as usize]
						.to_vec();

					PhysicalDevice { handle: vk_physical_device, memory_types, memory_heaps }
				})
				.collect()
		};


		Arc::new(Self { entry, handle, physical_devices })
	}

	fn extensions() -> Vec<*const i8> {
		vec![
			khr::Surface::name().as_ptr(),
			khr::Win32Surface::name().as_ptr(),
		]
	}
}

impl Destroy for Instance {
	type Error = Infallible;
	unsafe fn destroy(self) -> Result<(), Self::Error>{
		self.handle.destroy_instance(None);
		Ok(())
	}
}

impl SurfaceKHR {
	pub fn new(instance: Arc<Instance>, window: Window) -> Arc<Self> {
		let loader = khr::Surface::new(&instance.entry, &instance.handle);
		let handle = unsafe { Self::handle(&instance.entry, &instance.handle, &window) };

		Arc::new(Self { instance, loader, handle, window })
	}

	#[cfg(target_os = "windows")]
	unsafe fn handle(
		entry: &Entry,
		instance: &VkInstance,
		window: &winit::Window
	) -> vk::SurfaceKHR {
		use winapi::um::libloaderapi::GetModuleHandleW;
		use winit::os::windows::WindowExt;

		let info = vk::Win32SurfaceCreateInfoKHR {
			s_type: StructureType::WIN32_SURFACE_CREATE_INFO_KHR,
			p_next: ptr::null(),
			flags: vk::Win32SurfaceCreateFlagsKHR::empty(),
			hwnd: window.get_hwnd() as *const _,
			hinstance: GetModuleHandleW(ptr::null()) as *const _,
		};

		khr::Win32Surface::new(entry, instance)
			.create_win32_surface(&info, None)
			.unwrap()
	}
}

impl Destroy for SurfaceKHR {
	type Error = Infallible;
	/// You must destroy this before destroying Instance that create this.
	/// Also you must not destroy this before Swapchain created by this.
	unsafe fn destroy(self) -> Result<(), Self::Error> {
		self.loader.destroy_surface(self.handle, None);
		Ok(())
	}
}

impl Device {
	pub fn new_with_a_graphics_queue(
		instance: Arc<Instance>,
		surface: Arc<SurfaceKHR>,
		queue_priority: f32,
	) -> (Arc<Self>, Arc<Queue<Graphics>>) {
		let (physical_device_index, queue_family_index) = instance.physical_devices
			.iter()
			.enumerate()
			.try_fold((), |_, (i, physical_device)| {
				let properties = unsafe {
					instance.handle
						.get_physical_device_queue_family_properties(physical_device.handle)
				};

				let queue_family_index = properties
					.iter()
					.position(|properties| {
						let surface_support = unsafe {
							surface.loader
								.get_physical_device_surface_support(
									physical_device.handle,
									i as u32,
									surface.handle,
								)
						};

						let graphics_support = properties.queue_flags
							.contains(vk::QueueFlags::GRAPHICS);

						surface_support && graphics_support
					});

				match queue_family_index {
					Some(index) => Err((index, i)),
					None => Ok(()),
				}
			})
			.err()
			.unwrap();

		let queue_info = vk::DeviceQueueCreateInfo {
			s_type: StructureType::DEVICE_QUEUE_CREATE_INFO,
			p_next: ptr::null(),
			flags: vk::DeviceQueueCreateFlags::empty(),
			queue_family_index: queue_family_index as u32,
			queue_count: 1,
			p_queue_priorities: &queue_priority as *const _,
		};

		let extensions = Self::extensions();
		let features = vk::PhysicalDeviceFeatures::default();
		let device_info = vk::DeviceCreateInfo {
			s_type: StructureType::DEVICE_CREATE_INFO,
			p_next: ptr::null(),
			flags: vk::DeviceCreateFlags::empty(),
			queue_create_info_count: 1,
			p_queue_create_infos: &queue_info as *const _,
			enabled_layer_count: 0,
			pp_enabled_layer_names: ptr::null(),
			enabled_extension_count: extensions.len() as u32,
			pp_enabled_extension_names: extensions.as_ptr(),
			p_enabled_features: &features as *const _,
		};

		let device = {
			let handle = unsafe {
				instance.handle
					.create_device(
						instance.physical_devices[physical_device_index].handle,
						&device_info,
						None,
					)
					.unwrap()
			};

			Arc::new(Self { instance, physical_device_index, handle })
		};

		let queue = {
			let handle = unsafe { device.handle.get_device_queue(queue_family_index as u32, 0) };

			Arc::new(
				Queue {
					device: device.clone(),
					family_index: queue_family_index as u32,
					handle,
					_type: PhantomData,
				}
			)
		};


		(device, queue)
	}

	fn extensions() -> Vec<*const i8> {
		vec![khr::Swapchain::name().as_ptr()]
	}
}

impl Destroy for Device {
	type Error = Infallible;

	unsafe fn destroy(self) -> Result<(), Self::Error> {
		self.handle.destroy_device(None);
		Ok(())
	}
}