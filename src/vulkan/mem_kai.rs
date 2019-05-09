use ash::vk;
use ash::vk::StructureType;
use ash::version::DeviceV1_0;

use crate::vulkan::VkCore;

use std::io;
use std::fs::File;
use std::ptr;
use std::ops;
use std::fmt;
use std::mem;
use std::slice;
use std::marker::PhantomData;
use std::error::Error;
use std::path::Path;


pub use self::memory_type::*;
mod memory_type {
	use ash::vk::MemoryPropertyFlags as MP;

	pub trait MemoryProperties {
		fn flags() -> MP;
	}

	pub struct DeviceLocalFlag;
	pub struct HostVisibleFlag;
	pub struct HostCoherentFlag;

	macro_rules! impl_memory_properties {
		($marker_struct:ty, $flag_bits:expr) => {
			impl MemoryProperties for $marker_struct {
				fn flags() -> MP { $flag_bits }
			}
		};
	}

	impl_memory_properties!(DeviceLocalFlag, MP::DEVICE_LOCAL);
	impl_memory_properties!(HostVisibleFlag, MP::HOST_VISIBLE);
	impl_memory_properties!(HostCoherentFlag, MP::HOST_COHERENT);

	impl<T, U> MemoryProperties for (T, U)
		where T: MemoryProperties,
			  U: MemoryProperties
	{
		fn flags() -> MP { T::flags() | U::flags() }
	}

	impl<T, U, V> MemoryProperties for (T, U, V)
		where T: MemoryProperties,
			  U: MemoryProperties,
			  V: MemoryProperties
	{
		fn flags() -> MP { T::flags() | U::flags() | V::flags() }
	}

	pub trait DeviceLocal: MemoryProperties {}
	pub trait HostVisible: MemoryProperties {}
	pub trait HostCoherent: MemoryProperties {}

	macro_rules! impl_marker_trait {
		($t:ident, $s:ty) => {
			impl<T> $t for ($s, T) where T: MemoryProperties {}
			impl<T> $t for (T, $s) where T: MemoryProperties {}
			impl<T, U> $t for ($s, T, U) where T: MemoryProperties, U: MemoryProperties {}
			impl<T, U> $t for (T, $s, U) where T: MemoryProperties, U: MemoryProperties {}
			impl<T, U> $t for (T, U, $s) where T: MemoryProperties, U: MemoryProperties {}
		};
	}

	impl_marker_trait!(DeviceLocal, DeviceLocalFlag);
	impl_marker_trait!(HostVisible, HostVisibleFlag);
	impl_marker_trait!(HostCoherent, HostCoherentFlag);
}

pub use self::usage::{BufferUsage, ImageUsage};
mod usage {
	use ash::vk::BufferUsageFlags as BU;
	
	pub trait BufferUsage {
		fn flags() -> BU;
	}

	pub trait ImageUsage {
		fn flags() -> ash::vk::ImageUsageFlags;
	}


	pub struct Source;
	pub struct Destination;
	pub struct Vertex;
	pub struct Index;
	pub struct Uniform;

	pub struct Sampled;
	pub struct ColorAttach;
	pub struct DepthStencilAttach;
	pub struct TransientAttach;
	pub struct InputAttach;

	macro_rules! impl_buffer_usage {
		($flag:ty, $bits:expr) => {
			impl BufferUsage for $flag {
				fn flags() -> BU { $bits }
			}
		};
	}

	impl_buffer_usage!(Source, BU::TRANSFER_SRC);
	impl_buffer_usage!(Destination, BU::TRANSFER_DST);
	impl_buffer_usage!(Vertex, BU::VERTEX_BUFFER);
	impl_buffer_usage!(Index, BU::INDEX_BUFFER);
}

pub struct VkMemory<'vk_core, M> where M: MemoryProperties {
	vk_core: &'vk_core VkCore,
	handle: vk::DeviceMemory,
	size: u64,
	_type: PhantomData<M>,
}

pub struct VkBuffer<'vk_core, M, U> where M: MemoryProperties, U: BufferUsage {
	vk_core: &'vk_core VkCore,
	handle: vk::Buffer,
	vk_memory: vk::DeviceMemory,
	size: u64,
	_memory_type: PhantomData<M>,
	_usage: PhantomData<U>,
}

pub struct VkData<M, U, D> where M: MemoryProperties, U: BufferUsage, D: ?Sized {
	offset: u64,
	len: u64,
	vk_memory: vk::DeviceMemory,
	vk_buffer: vk::Buffer,
	_memory_type: PhantomData<M>,
	_buffer_usage: PhantomData<U>,
	_data_type: PhantomData<D>,
}


pub trait VertexBuffer {
	type VertexType: Vertex;
	fn vk_buffer_id(&self) -> vk::Buffer;
	fn offset(&self) -> u64;
	fn count(&self) -> u64;
}

pub trait Vertex {
	fn input_descriptions() -> VertexInputDescriptions;
}

pub struct VertexInputDescriptions {
	binding: vk::VertexInputBindingDescription,
	attributes_count: usize,
	attributes: [vk::VertexInputAttributeDescription; 8],
}

pub struct VertexInputState<'desc> {
	input_state: vk::PipelineVertexInputStateCreateInfo,
	_marker: PhantomData<&'desc VertexInputDescriptions>,
}

pub trait GraphicPipeline {
	type Vertex: Vertex;
	type DescriptorSet;
	type PushConstant;
}

pub type VkResult<T> = Result<T, vk::Result>;

impl<'vk_core, M> VkMemory<'vk_core, M> where M: MemoryProperties {
	fn new(vk_core: &VkCore, size: u64) -> VkResult<Self> {
		let flags = M::flags();
		unimplemented!()
	}

	fn buffer<U>(range: ops::Range<u64>) -> VkResult<VkBuffer<'vk_core, M, U>>
		where U: BufferUsage
	{
		let flags = U::flags();
		unimplemented!()
	}
}

impl<'vk_core, M> VkMemory<'vk_core, M> where M: HostVisible + HostCoherent {
	fn write(&mut self, buf: &mut [u8]) -> VkResult<u64> {
		unimplemented!()
	}
}

fn memory(vk_core: &VkCore) {
	let mut memory: VkMemory<(DeviceLocalFlag, (HostVisibleFlag, HostCoherentFlag))>
		= VkMemory::new(vk_core, 100).unwrap();

	memory.write(&mut [0, 1, 2]).unwrap();
}