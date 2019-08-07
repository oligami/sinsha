//! # About This Module
//! This module is wrapper of Vulkan API.
//!
//! # Wrapping Policy
//! This wrapper is not for safety. It's make only easy to use.
//! Lifetime of all Vulkan API objects must be

pub mod utility;
pub mod queue_flag;
pub mod device_memory;
pub mod swapchain;
pub mod render_pass;
pub mod framebuffer;
pub mod descriptor;
//pub mod shader;
//pub mod command;

//pub use shader::pipeline;
//pub use shader::descriptor;

use queue_flag::QueueFlags;

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

pub trait Destroy: Sized {
    type Ok;
    type Error: std::error::Error;

    /// Destroy Vulkan API objects without checking.
    ///
    /// # Safety
    /// This method can cause memory violence very easily,
    /// but destroying will occur after all child objects have been destroyed.
    /// This is achieved by child objects having parent objects in std::sync::Arc.
    ///
    /// So, you should be careful for objects used by GPU such as Buffer, Image, DescriptorSet, etc.
    ///
    /// # Future
    /// Manually drop by this destructor should be replaced by Auto-Drop.
    /// A vk::CommandBuffer with a Fence and objects such as Buffer can be achieve this.
    ///
    /// # Multi Thread Synchronization
    /// Vulkan API says: "Host access to objects must be externally synchronized."
    /// This is satisfied by taking ownership. It's also prevent from double-freeing.
    unsafe fn destroy(self) -> Result<Self::Ok, Self::Error>;


    /// Almost all objects are used in std::sync::Arc.
    /// If strong count of Arc is not 1, this method will fail to destroying and return error.
    /// If strong count of Arc is 1, then self will be destroyed.
    /// This method is just for convenience and is unsafe because it's doesn't check command
    /// buffer usage.
    unsafe fn try_destroy(self: std::sync::Arc<Self>) -> Result<Self::Ok, DestroyError<Self::Error>> {
        let obj = std::sync::Arc::try_unwrap(self).map_err(|_| DestroyError::NonZeroStrongCount)?;
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
    queue_families: Vec<vk::QueueFamilyProperties>,
}

pub struct DebugEXT<I> where I: Borrow<Instance> + Deref<Target = Instance> {
    instance: I,
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

pub trait DeviceAbs {
    type Instance: Borrow<Instance> + Deref<Target = Instance>;

    fn instance(&self) -> &Instance;
    fn handle(&self) -> &VkDevice;
}

pub struct Queue<I, D, F>
    where I: Borrow<Instance> + Deref<Target = Instance>,
          D: Borrow<Device<I>> + Deref<Target = Device<I>>,
          F: QueueFlags
{
    _instance: PhantomData<I>,
    device: D,
    handle: vk::Queue,
    family_index: u32,
    _flag: PhantomData<F>,
}

pub trait QueueAbs {
    type Instance: Borrow<Instance> + Deref<Target = Instance>;
    type Device: Borrow<Device<Self::Instance>>;
    type Flag;

    fn instance(&self) -> &Self::Instance;
    fn device(&self) -> &Self::Device;
    fn handle(&self) -> vk::Queue;
    fn family_index(&self) -> u32;
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

    pub fn physical_device_num(&self) -> usize { self.physical_devices.len() }

    pub fn physical_device_handle(&self, index: usize) -> vk::PhysicalDevice {
        self.physical_devices[index].handle
    }

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

impl Destroy for Instance {
    type Ok = ();
    type Error = Infallible;
    unsafe fn destroy(self) -> Result<Self::Ok, Self::Error>{
        self.handle.destroy_instance(None);
        Ok(())
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

        Self { instance, report_loader, report, utils_loader, utils }
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

    #[inline]
    pub fn window(&self) -> &Window { &self.window }

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

impl<I> Destroy for SurfaceKHR<I> where I: Borrow<Instance> + Deref<Target = Instance> {
    type Ok = ();
    type Error = Infallible;
    /// You must destroy this before destroying Instance that create this.
    /// Also you must not destroy this before Swapchain created by this.
    unsafe fn destroy(self) -> Result<Self::Ok, Self::Error> {
        self.loader.destroy_surface(self.handle, None);
        Ok(())
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

    fn extensions() -> Vec<*const i8> {
        vec![khr::Swapchain::name().as_ptr() as _]
    }
}

impl<I> DeviceAbs for Device<I> where I: Borrow<Instance> + Deref<Target = Instance> {
    type Instance = I;

    fn instance(&self) -> &Instance { &self.instance }
    fn handle(&self) -> &VkDevice { &self.handle }
}

impl<I> Destroy for Device<I> where I: Borrow<Instance> + Deref<Target = Instance> {
    type Ok = ();
    type Error = Infallible;

    unsafe fn destroy(self) -> Result<Self::Ok, Self::Error> {
        self.handle.destroy_device(None);
        Ok(())
    }
}

impl<I> Drop for Device<I> where I: Borrow<Instance> + Deref<Target = Instance> {
    fn drop(&mut self) {
        unsafe { self.handle.destroy_device(None); }
    }
}

impl<I, D> Queue<I, D, queue_flag::Empty>
    where I: Borrow<Instance> + Deref<Target = Instance>,
          D: Borrow<Device<I>> + Deref<Target = Device<I>>
{
    pub unsafe fn from_device(device: D, info: &QueueInfo, index: usize) -> Self {
        assert!((index as u32) < info.vk_info.queue_count, "Index is out of Queue count.");
        let handle = device.handle
            .get_device_queue(info.family_index as u32, index as u32);

        Queue {
            _instance: PhantomData,
            device,
            handle,
            family_index: info.family_index as u32,
            _flag: PhantomData,
        }
    }

    pub unsafe fn convert_flag<F>(self, queue_flag: F) -> Queue<I, D, F> where F: QueueFlags {
        Queue {
            _instance: PhantomData,
            device: self.device,
            handle: self.handle,
            family_index: self.family_index,
            _flag: PhantomData,
        }
    }
}

impl<I, D, F> Queue<I, D, F>
    where I: Borrow<Instance> + Deref<Target = Instance>,
          D: Borrow<Device<I>> + Deref<Target = Device<I>>,
          F: QueueFlags
{
    pub fn family_index(&self) -> u32 { self.family_index }
}

impl<'i, 'd> Queue<&'i Instance, &'d Device<&'i Instance>, queue_flag::Empty> {
    pub fn get_queue_info_with_surface_support<I, F>(
        instance: &Instance,
        physical_device_index: usize,
        surface: &SurfaceKHR<I>,
        flags: F,
    ) -> Option<QueueInfo> where I: Borrow<Instance> + Deref<Target = Instance>, F: QueueFlags {
        let physical_device = &instance.physical_devices[physical_device_index];
        let properties = unsafe {
            instance.handle.get_physical_device_queue_family_properties(physical_device.handle)
        };

        properties
            .iter()
            .enumerate()
            .position(|(queue_family_index, property)| {
                let surface_support = unsafe {
                    surface.loader
                        .get_physical_device_surface_support(
                            physical_device.handle,
                            queue_family_index as u32,
                            surface.handle,
                        )
                };

                let queue_flags_support = property.queue_flags
                    .contains(F::queue_flags());

                surface_support && queue_flags_support
            })
            .map(|valid_queue_family_index| {
                let vk_info = properties[valid_queue_family_index];
                QueueInfo { vk_info, family_index: valid_queue_family_index }
            })
    }
}
