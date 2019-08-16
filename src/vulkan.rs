//! # About This Module
//! This module is wrapper of Vulkan API.
//!
//! # Wrapping Policy
//! This wrapper is not for safety. It's make only easy to use.
//! Lifetime of all Vulkan API objects must be

pub mod utility;
pub mod device_memory;
pub mod swapchain;
pub mod render_pass;
pub mod framebuffer;
pub mod descriptor;
pub mod pipeline;
pub mod vertex;
pub mod command;
pub mod sync;

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
use ash::Instance as VkInstance;
use ash::Device as VkDevice;
use ash::version::EntryV1_0;
use ash::version::InstanceV1_0;
use ash::version::DeviceV1_0;

use winit::window::Window;

use std::ptr;
use std::ffi::CString;
use std::marker::PhantomData;
use std::sync::Arc;

use std::ops::Deref;
use std::borrow::Borrow;

pub struct Instance {
    entry: Entry,
    handle: VkInstance,
    physical_devices: Vec<PhysicalDevice>,
}

pub struct PhysicalDevice {
    handle: vk::PhysicalDevice,
    memory_types: Vec<vk::MemoryType>,
    memory_heaps: Vec<vk::MemoryHeap>,
    queue_families: Vec<vk::QueueFamilyProperties>,
}

pub struct DebugEXT<I> where I: Borrow<Instance> + Deref<Target = Instance> {
    instance: PhantomData<I>,
    report_loader: ext::DebugReport,
    report: vk::DebugReportCallbackEXT,
    utils_loader: ext::DebugUtils,
    utils: vk::DebugUtilsMessengerEXT,
}

pub struct SurfaceKHR<I> where I: Borrow<Instance> + Deref<Target = Instance> {
    instance: I,
    loader: khr::Surface,
    handle: vk::SurfaceKHR,
    window: Window,
}

pub struct Device<I> where I: Borrow<Instance> + Deref<Target = Instance> {
    instance: I,
    physical_device_index: usize,
    handle: VkDevice,
}

pub struct Queue<I, D> where
    I: Borrow<Instance> + Deref<Target = Instance>,
    D: Borrow<Device<I>> + Deref<Target = Device<I>>,
{
    _marker: PhantomData<I>,
    device: D,
    handle: vk::Queue,
    family_index: u32,
}

pub struct QueueInfo {
    family_index: usize,
    vk_info: vk::QueueFamilyProperties,
}

impl Instance {
    pub fn new() -> Self {
        let entry = Entry::new().unwrap();

        let handle = {
            let app_info = vk::ApplicationInfo {
                s_type: StructureType::APPLICATION_INFO,
                p_next: ptr::null(),
                p_application_name: ptr::null(),
                application_version: 0,
                p_engine_name: ptr::null(),
                engine_version: 0,
                api_version: vk_make_version!(1, 1, 117),
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

                    let property = unsafe {
                        handle.get_physical_device_properties(vk_physical_device)
                    };

                    let memory_types = memory_properties
                        .memory_types[..memory_properties.memory_type_count as usize]
                        .to_vec();

                    let memory_heaps = memory_properties
                        .memory_heaps[..memory_properties.memory_heap_count as usize]
                        .to_vec();

                    let queue_families = unsafe {
                        handle.get_physical_device_queue_family_properties(vk_physical_device)
                    };

                    PhysicalDevice {
                        handle: vk_physical_device,
                        memory_types,
                        memory_heaps,
                        queue_families,
                    }
                })
                .collect()
        };


        Self { entry, handle, physical_devices }
    }

    #[inline]
    pub fn physical_device_num(&self) -> usize { self.physical_devices.len() }

    fn extensions() -> Vec<*const i8> {
        if cfg!(debug_assertions) {
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
        }
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        unsafe { self.handle.destroy_instance(None); }
    }
}

impl<I> DebugEXT<I> where I: Borrow<Instance> + Deref<Target = Instance> {
    pub fn new(instance: I) -> Self {
        let info = vk::DebugReportCallbackCreateInfoEXT::builder()
            .flags(
                vk::DebugReportFlagsEXT::ERROR
                    | vk::DebugReportFlagsEXT::WARNING
                    | vk::DebugReportFlagsEXT::PERFORMANCE_WARNING
            )
            .pfn_callback(Some(Self::report_callback));

        let report_loader = ext::DebugReport::new(&instance.entry, &instance.handle);
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

        let utils_loader = ext::DebugUtils::new(&instance.entry, &instance.handle);
        let utils = unsafe { utils_loader.create_debug_utils_messenger(&info, None).unwrap() };

        Self { instance: PhantomData, report_loader, report, utils_loader, utils }
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

//        eprintln!("{}: {}\n {}", header, flags, message);

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

impl<I> Drop for DebugEXT<I> where I: Borrow<Instance> + Deref<Target = Instance> {
    fn drop(&mut self) {
        unsafe {
            self.report_loader.destroy_debug_report_callback(self.report, None);
            self.utils_loader.destroy_debug_utils_messenger(self.utils, None);
        }
    }
}

impl<I> SurfaceKHR<I> where I: Borrow<Instance> + Deref<Target = Instance> {
    pub fn new(instance: I, window: Window) -> Self {
        let loader = khr::Surface::new(&instance.entry, &instance.handle);
        let handle = unsafe { Self::handle(&instance.entry, &instance.handle, &window) };

        Self { instance, loader, handle, window }
    }

    #[cfg(target_os = "windows")]
    unsafe fn handle(
        entry: &Entry,
        instance: &VkInstance,
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

impl<I> Drop for SurfaceKHR<I> where I: Borrow<Instance> + Deref<Target = Instance> {
    fn drop(&mut self) {
        unsafe { self.loader.destroy_surface(self.handle, None); }
    }
}

impl<I> Device<I> where I: Borrow<Instance> + Deref<Target = Instance> {
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
            instance.handle
                .create_device(
                    instance.physical_devices[physical_device_index].handle,
                    &*device_info,
                    None,
                )
                .unwrap()
        };

        Self { instance, physical_device_index, handle }
    }

    #[inline]
    pub fn fp(&self) -> &VkDevice { &self.handle }

    fn extensions() -> Vec<*const i8> {
        vec![khr::Swapchain::name().as_ptr() as _]
    }
}

impl<I> Drop for Device<I> where I: Borrow<Instance> + Deref<Target = Instance> {
    fn drop(&mut self) {
        unsafe { self.handle.destroy_device(None); }
    }
}

impl<I, D> Queue<I, D>
    where I: Borrow<Instance> + Deref<Target = Instance>,
          D: Borrow<Device<I>> + Deref<Target = Device<I>>
{
    pub fn from_device(device: D, info: &QueueInfo, index: usize) -> Self {
        assert!((index as u32) < info.vk_info.queue_count, "Index is out of Queue count.");
        let handle = unsafe {
            device.handle.get_device_queue(info.family_index as u32, index as u32)
        };

        Queue {
            _marker: PhantomData,
            device,
            handle,
            family_index: info.family_index as u32,
        }
    }

    pub unsafe fn submit(&mut self, infos: &[vk::SubmitInfo], fence: vk::Fence) {
        self.device.handle.queue_submit(self.handle, infos, fence).unwrap();
    }

    pub unsafe fn wait_idle(&self) {
        self.device.handle.device_wait_idle().unwrap()
    }

    #[inline]
    pub fn handle(&self) -> vk::Queue { self.handle }
    #[inline]
    pub fn family_index(&self) -> u32 { self.family_index }
}

impl<'i, 'd> Queue<&'i Instance, &'d Device<&'i Instance>> {
    pub fn get_queue_info_with_surface_support<I>(
        instance: &Instance,
        physical_device_index: usize,
        surface: &SurfaceKHR<I>,
        flags: vk::QueueFlags,
    ) -> Option<QueueInfo> where I: Borrow<Instance> + Deref<Target = Instance> {
        let physical_device = &instance.physical_devices[physical_device_index];

        physical_device.queue_families
            .iter()
            .enumerate()
            .find(|(queue_family_index, property)| {
                let surface_support = unsafe {
                    surface.loader
                        .get_physical_device_surface_support(
                            physical_device.handle,
                            *queue_family_index as u32,
                            surface.handle,
                        )
                };

                let queue_flags_support = property.queue_flags
                    .contains(flags);

                surface_support && queue_flags_support
            })
            .map(|(valid_queue_family_index, info)| {
                QueueInfo { vk_info: *info, family_index: valid_queue_family_index }
            })
    }
}
