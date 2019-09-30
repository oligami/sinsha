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

mod memory;
pub mod render;

use ash::vk;
use ash::vk_make_version;
use ash::vk::StructureType;
use ash::extensions::{ khr, ext };
use ash::{ Entry, Instance, Device };
use ash::version::{ EntryV1_0, InstanceV1_0, DeviceV1_0 };

use winit::window::Window;

use std::ptr;
use std::sync::Once;
use std::ffi::CString;
use std::mem::ManuallyDrop;

pub struct Vulkan {
    entry: Entry,
    instance: Instance,
    surface: ManuallyDrop<SurfaceKHR>,
    physical_device: PhysicalDevice,
    device: Device,
    debug: ManuallyDrop<DebugEXT>,
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

pub struct Queue(vk::Queue);

impl Vulkan {
    pub fn new(window: Window) -> Self {
        let entry = Entry::new().unwrap();
        let instance = Self::create_instance(&entry);
        let debug = DebugEXT::new_in_manually_drop(&entry, &instance);
        let surface = SurfaceKHR::new_in_manually_drop(&entry, &instance, window);
        let (physical_device, device) = Self::create_device(&instance, &surface);

        Self { entry, instance, surface, physical_device, device, debug }
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

    fn create_device(instance: &Instance, surface: &SurfaceKHR) -> (PhysicalDevice, Device) {
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
                            .contains(vk::QueueFlags::GRAPHICS);

                        surface_support && queue_flags_support
                    })
                    .map(|(queue_family_index, _)| (vk_physical_device, queue_family_index as u32))
            })
            .unwrap();

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

        let extensions = [];
        let layers = if cfg!(debug_assertions) {
            vec![khr::Swapchain::name().as_ptr()]
        } else {
            vec![]
        };
        let device_info = vk::DeviceCreateInfo::builder()
            .queue_create_infos(std::slice::from_ref(&queue_info))
            .enabled_extension_names(&extensions[..])
            .enabled_layer_names(&layers[..])
            .build();

        let device = unsafe {
            instance
                .create_device(vk_physical_device, &device_info, None)
                .unwrap()
        };

        (physical_device, device)
    }
}

impl Drop for Vulkan {
    fn drop(&mut self) {
        unsafe {
            unsafe { ManuallyDrop::drop(&mut self.surface); }
            // debug is only enabled in debug mode, not in release mode.
            if cfg!(debug_assertions) {
                unsafe { ManuallyDrop::drop(&mut self.debug); }
            }
            self.device.destroy_device(None);
            self.instance.destroy_instance(None);
        }
    }
}

impl DebugEXT {
    pub fn new_in_manually_drop(entry: &Entry, instance: &Instance) -> ManuallyDrop<Self> {
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

        ManuallyDrop::new(Self { report_loader, report, utils_loader, utils })
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
    fn new_in_manually_drop(
        entry: &Entry,
        instance: &Instance,
        window: Window
    ) -> ManuallyDrop<Self> {
        let surface_khr = SurfaceKHR {
            loader: khr::Surface::new(entry, instance),
            handle: unsafe { Self::handle(entry, instance, &window) },
            window,
        };

        ManuallyDrop::new(surface_khr)
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

impl Drop for SurfaceKHR {
    fn drop(&mut self) {
        unsafe { self.loader.destroy_surface(self.handle, None); }
    }
}


