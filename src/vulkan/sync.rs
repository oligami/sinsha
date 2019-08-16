use super::*;

use std::time::Duration;

pub struct Semaphore<I, D> where
    I: Borrow<Instance> + Deref<Target = Instance>,
    D: Borrow<Device<I>> + Deref<Target = Device<I>>,
{
    _marker: PhantomData<I>,
    device: D,
    handle: vk::Semaphore,
}

pub struct Fence<I, D> where
    I: Borrow<Instance> + Deref<Target = Instance>,
    D: Borrow<Device<I>> + Deref<Target = Device<I>>,
{
    _marker: PhantomData<I>,
    device: D,
    handle: vk::Fence,
}

pub enum FenceState {
    Signaled,
    UnSignaled,
}

impl<I, D> Semaphore<I, D> where
    I: Borrow<Instance> + Deref<Target = Instance>,
    D: Borrow<Device<I>> + Deref<Target = Device<I>>,
{
    pub fn new(device: D) -> Self {
        let info = vk::SemaphoreCreateInfo::default();
        let handle = unsafe { device.handle.create_semaphore(&info, None).unwrap() };

        Semaphore { _marker: PhantomData, device, handle }
    }

    #[inline]
    pub fn handle(&self) -> vk::Semaphore { self.handle }
}

impl<I, D> Drop for Semaphore<I, D> where
    I: Borrow<Instance> + Deref<Target = Instance>,
    D: Borrow<Device<I>> + Deref<Target = Device<I>>,
{
    fn drop(&mut self) {
        unsafe { self.device.handle.destroy_semaphore(self.handle, None); }
    }
}

impl<I, D> Fence<I, D> where
    I: Borrow<Instance> + Deref<Target = Instance>,
    D: Borrow<Device<I>> + Deref<Target = Device<I>>,
{
    fn new(device: D, flags: vk::FenceCreateFlags) -> Self {
        let info = vk::FenceCreateInfo::builder()
            .flags(flags);
        let handle = unsafe { device.handle.create_fence(&info, None).unwrap() };

        Fence { _marker: PhantomData, device, handle }
    }

    pub fn signaled(device: D) -> Self {
        Self::new(device, vk::FenceCreateFlags::SIGNALED)
    }

    pub fn unsignaled(device: D) -> Self {
        Self::new(device, vk::FenceCreateFlags::empty())
    }

    pub fn poll(&self) -> FenceState {
        unsafe {
            match self.device.handle.get_fence_status(self.handle) {
                Ok(_) => FenceState::Signaled,
                Err(vk::Result::NOT_READY) => FenceState::UnSignaled,
                Err(vk::Result::ERROR_DEVICE_LOST) => {
                    panic!("The device of the fence[{:?}] has lost", self.handle);
                },
                _ => unreachable!(),
            }
        }
    }

    pub fn try_wait(&self, timeout: Duration) {
        unsafe {
            self.device.handle
                .wait_for_fences(&[self.handle], false, timeout.as_nanos() as u64)
                .unwrap();
        }
    }

    pub fn wait(&self) {
        unsafe { self.device.handle.wait_for_fences(&[self.handle], false, !0).unwrap(); }
    }

    pub unsafe fn reset(&self) {
        self.device.handle.reset_fences(&[self.handle]).unwrap();
    }

    #[inline]
    pub fn handle(&self) -> vk::Fence { self.handle }
}

impl<I, D> Drop for Fence<I, D> where
    I: Borrow<Instance> + Deref<Target = Instance>,
    D: Borrow<Device<I>> + Deref<Target = Device<I>>,
{
    fn drop(&mut self) {
        unsafe { self.device.handle.destroy_fence(self.handle, None); }
    }
}