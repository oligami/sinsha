//! # Objects that need mutable reference
//! ## vk::Queue
//! Submitting commands.
//!
//! ## vk::SwapchainKHR
//! Recreating. Require vk::SurfaceKHR to get new extent and also recreate framebuffers.
//!
//! ## vk::DeviceMemory
//! Allocating and Deallocating.
//!
//!
//! ## vk::Buffer
//! Allocating and Deallocating.

pub mod device_memory;
pub mod render;
pub mod device;

pub use device_memory::buffer;
pub use device_memory::image;
//pub use shader::pipeline;
//pub use shader::descriptor;

use ash::vk;
use ash::vk::StructureType;
use ash::extensions::khr;
use ash::extensions::ext;
use ash::vk_make_version;
use ash::Entry;
use ash::Instance;
use ash::Device;
use ash::version::EntryV1_0;
use ash::version::InstanceV1_0;
use ash::version::DeviceV1_0;

use winit::window::Window;

use std::ptr;
use std::ffi::CString;
use std::marker::PhantomData;
use std::borrow::Borrow;

#[cfg(debug_assertions)]
pub struct Vulkan {
    entry: Entry,
    handle: Instance,
    surface: SurfaceKHR,
    physical_device: PhysicalDevice,
    device: Device,
    debug: DebugEXT,
}

#[cfg(not(debug_assertions))]
pub struct Vulkan {
    entry: Entry,
    handle: Instance,
    surface: SurfaceKHR,
    physical_device: PhysicalDevice,
    device: Device,
}

struct SurfaceKHR {
    window: Window,
    loader: khr::Surface,
    handle: vk::SurfaceKHR,
}

struct PhysicalDevice {
    handle: vk::PhysicalDevice,
    memory_types: Vec<vk::MemoryType>,
}

struct DebugEXT {
    report_loader: ext::DebugReport,
    report: vk::DebugReportCallbackEXT,
    utils_loader: ext::DebugUtils,
    utils: vk::DebugUtilsMessengerEXT,
}

pub struct Queue<V> {
    vulkan: V,
    handle: vk::Queue,
}

impl Vulkan {
    pub fn new() -> Self {
        let entry = Entry::new().unwrap();
        let instance = Self::create_instance(&entry);




        unimplemented!()
    }

    fn create_instance(entry: &Entry) -> Instance {
        let app_info = vk::ApplicationInfo {
            s_type: StructureType::APPLICATION_INFO,
            p_next: ptr::null(),
            p_application_name: ptr::null(),
            application_version: 0,
            p_engine_name: ptr::null(),
            engine_version: 0,
            api_version: vk_make_version!(1, 1, 117),
        };

        let instance_extensions = if cfg!(debug_assertions) {
            vec![
                khr::Surface::name().as_ptr() as _,
                khr::Win32Surface::name().as_ptr() as _,
                ext::DebugReport::name().as_ptr() as _,
                ext::DebugUtils::name().as_ptr() as _,
            ]
        } else {
            vec![
                khr::Surface::name().as_ptr() as _,
                khr::Win32Surface::name().as_ptr() as _,
            ]
        };

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
    }

    fn select_physical_device(instance: &Instance, surface: &SurfaceKHR) {
        let vk_physical_devices = unsafe { instance.enumerate_physical_devices().unwrap() };
        let (vk_physical_device, queue_family_index) = vk_physical_devices
            .into_iter()
            .find_map(|vk_physical_device| {
                let property = unsafe {
                    instance.get_physical_device_properties(vk_physical_device)
                };

                let queue_families = unsafe {
                    instance.get_physical_device_queue_family_properties(vk_physical_device)
                };

                queue_families
                    .iter()
                    .enumerate()
                    .find(|(queue_family_index, property)| {
                        let surface_support = unsafe {
                            surface.loader
                                .get_physical_device_surface_support(
                                    vk_physical_device,
                                    *queue_family_index as u32,
                                    surface.handle,
                                )
                        };

                        let queue_flags_support = property.queue_flags
                            .contains(flags);

                        surface_support && queue_flags_support
                    })
                    .map(|(queue_family_index, _)| (vk_physical_device, queue_family_index as u32))
            });

        let memory_properties = unsafe {
            instance.get_physical_device_memory_properties(vk_physical_device)
        };

        let memory_types = memory_properties
            .memory_types[..memory_properties.memory_type_count as usize]
            .to_vec();

        let physical_device = PhysicalDevice {
            handle: vk_physical_device,
            memory_types,
        };

        let queue_priorities = [1.0_f32];
        let queue_info = vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(queue_family_index)
            .queue_priorities(&queue_priorities[..])
            .build();

        let device_info = vk::DeviceCreateInfo::builder()
            .queue_create_infos(std::slice::from_ref(&queue_info))
            .enabled_extension_names(unimplemented!())
            .enabled_layer_names(unimplemented!())
            .build();



        unimplemented!()
    }
}

impl Drop for Vulkan {
    fn drop(&mut self) {
        unsafe { self.instance.destroy_instance(None); }
        unimplemented!()
    }
}

impl DebugEXT {
    pub fn new(entry: &Entry, instance: &Instance) -> Self {
        let info = vk::DebugReportCallbackCreateInfoEXT::builder()
            .flags(
                vk::DebugReportFlagsEXT::ERROR
                    | vk::DebugReportFlagsEXT::WARNING
                    | vk::DebugReportFlagsEXT::PERFORMANCE_WARNING
            )
            .pfn_callback(Some(Self::report_callback));

        let report_loader = ext::DebugReport::new(entry, instance);
        let report = unsafe { report_loader.create_debug_report_callback(&info, None).unwrap() };

        let info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
            .message_severity(
                vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                    | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                    | vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
            )
            .message_type(
                vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                    | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
                    | vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
            )
            .pfn_user_callback(Some(Self::utils_callback));

        let utils_loader = ext::DebugUtils::new(entry, instance);
        let utils = unsafe { utils_loader.create_debug_utils_messenger(&info, None).unwrap() };

        Self { report_loader, report, utils_loader, utils }
    }

    unsafe extern "system" fn report_callback(
        flags: vk::DebugReportFlagsEXT,
        _object_type: vk::DebugReportObjectTypeEXT,
        _object: u64,
        _location: usize,
        _message_code: i32,
        _p_layer_prefix: *const i8,
        p_message: *const i8,
        _p_user_data: *mut std::ffi::c_void,
    ) -> vk::Bool32 {
        let message = std::ffi::CStr::from_ptr(p_message as _).to_str().unwrap();
        let header = "\u{001b}[37;1mReport\u{001b}[0m";
        let flags = match flags {
            vk::DebugReportFlagsEXT::ERROR => "\u{001b}[31mError\u{001b}[0m",
            vk::DebugReportFlagsEXT::WARNING => "\u{001b}[33mWarning\u{001b}[0m",
            vk::DebugReportFlagsEXT::PERFORMANCE_WARNING => "\u{001b}[35mPerformance Warning\u{001b}[0m",
            vk::DebugReportFlagsEXT::DEBUG => "\u{001b}[32mDebug\u{001b}[0m",
            vk::DebugReportFlagsEXT::INFORMATION => "\u{001b}[34mInformation\u{001b}[0m",
            _ => "Unknown",
        };

        // eprintln!("{}: {}\n {}", header, flags, message);

        vk::FALSE
    }

    unsafe extern "system" fn utils_callback(
        message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
        message_types: vk::DebugUtilsMessageTypeFlagsEXT,
        p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
        p_user_data: *mut std::ffi::c_void,
    ) -> vk::Bool32 {
        let message = std::ffi::CStr::from_ptr((*p_callback_data).p_message).to_str().unwrap();

        let header = "\u{001b}[37;1m[Utils]\u{001b}[0m";

        let severity = match message_severity {
            vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => "\u{001b}[31mError\u{001b}[0m",
            vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => "\u{001b}[33mWarning\u{001b}[0m",
            vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE => "\u{001b}[37mVerbose\u{001b}[0m",
            vk::DebugUtilsMessageSeverityFlagsEXT::INFO => "\u{001b}[34mInfo\u{001b}[0m",
            _ => "\u{001b}[90mUnknown\u{001b}[0m"
        };

        let ty = match message_types {
            vk::DebugUtilsMessageTypeFlagsEXT::GENERAL => "General",
            vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE => "Performance",
            vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION => "Validation",
            _ => "\u{001b}[90mUnknown\u{001b}[0m",
        };

        eprintln!("{}\n\tSeverity: {}; Type: {}\n\t{}\n", header, severity, ty, message);

        vk::FALSE
    }
}

impl Drop for DebugEXT {
    fn drop(&mut self) {
        unsafe {
            self.report_loader.destroy_debug_report_callback(self.report, None);
            self.utils_loader.destroy_debug_utils_messenger(self.utils, None);
        }
    }
}

impl SurfaceKHR {
    fn new(entry: &Entry, instance: &Instance, window: Window) -> Self {
        SurfaceKHR {
            window,
            loader: khr::Surface::new(entry, instance),
            handle: unsafe { Self::handle(entry, instance, &window) },
        }
    }


    #[cfg(target_os = "windows")]
    unsafe fn handle(
        entry: &Entry,
        instance: &Instance,
        window: &Window
    ) -> vk::SurfaceKHR {
        use winapi::um::libloaderapi::GetModuleHandleW;
        use winit::platform::windows::WindowExtWindows;

        let info = vk::Win32SurfaceCreateInfoKHR::builder()
            .hwnd(window.hwnd())
            .hinstance(GetModuleHandleW(ptr::null()) as _);

        khr::Win32Surface::new(entry, instance)
            .create_win32_surface(&*info, None)
            .unwrap()
    }
}

impl<I> Device<I> where I: Borrow<Vulkan> {
    pub fn new(
        instance: I,
        physical_device_index: usize,
        queue_info_and_priority: &[(&QueueInfo, &[f32])],
    ) -> Self {
        let queue_infos = queue_info_and_priority.iter()
            .map(|(info, priorities)| {
                vk::DeviceQueueCreateInfo::builder()
                    .queue_family_index(info.family_index as u32)
                    .queue_priorities(priorities)
                    .build()
            })
            .collect::<Vec<_>>();

        let extensions = Self::extensions();
        let features = vk::PhysicalDeviceFeatures::default();
        let device_info = vk::DeviceCreateInfo::builder()
            .enabled_layer_names(&[])
            .enabled_extension_names(&extensions[..])
            .enabled_features(&features)
            .queue_create_infos(&queue_infos);

        let handle = unsafe {
            instance.borrow().handle
                .create_device(
                    instance.borrow().physical_devices[physical_device_index].handle,
                    &*device_info,
                    None,
                )
                .unwrap()
        };

        Self { instance, physical_device_index, handle }
    }

    #[inline]
    pub fn fp(&self) -> &Device { &self.handle }

    fn extensions() -> Vec<*const i8> {
        vec![khr::Swapchain::name().as_ptr() as _]
    }
}

impl<I> Drop for Device<I> {
    #[inline]
    fn drop(&mut self) { unsafe { self.handle.destroy_device(None); } }
}

impl<I, D> Queue<I, D> where D: Borrow<Device<I>> {
    pub fn from_device(device: D, info: &QueueInfo, index: usize) -> Self {
        assert!((index as u32) < info.vk_info.queue_count, "Index is out of Queue count.");
        let handle = unsafe {
            device.borrow().handle.get_device_queue(info.family_index as u32, index as u32)
        };

        Queue {
            _instance: PhantomData,
            device,
            handle,
            family_index: info.family_index as u32,
        }
    }

    pub unsafe fn submit(&mut self, infos: &[vk::SubmitInfo], fence: vk::Fence) {
        self.device.borrow().handle.queue_submit(self.handle, infos, fence).unwrap();
    }

    pub unsafe fn wait_idle(&self) {
        self.device.borrow().handle.device_wait_idle().unwrap()
    }

    #[inline]
    pub fn handle(&self) -> vk::Queue { self.handle }
    #[inline]
    pub fn family_index(&self) -> u32 { self.family_index }
}

