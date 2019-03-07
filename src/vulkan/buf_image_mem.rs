use ash::vk;
use ash::vk::StructureType;
use ash::version::DeviceV1_0;

use crate::vulkan::*;

use std::io;
use std::ptr;
use std::fmt;
use std::mem;
use std::slice;
use std::error::Error;
use std::ops::{Range, Bound, RangeBounds};
use std::path::Path;

pub struct LogicalBuffer<'vk_core> {
	vk_core: &'vk_core VkCore,
	raw_handle: vk::Buffer,
	memory_requirements: vk::MemoryRequirements,
}

pub struct Buffer<'vk_core> {
	logical: LogicalBuffer<'vk_core>,
	range: Range<vk::DeviceSize>,
}

pub struct LogicalImage<'vk_core> {
	vk_core: &'vk_core VkCore,
	raw_handle: vk::Image,
	extent: vk::Extent3D,
	format: vk::Format,
	layout: Vec<vk::ImageLayout>,
	aspect_mask: vk::ImageAspectFlags,
	mip_levels: u32,
	array_layers: u32,
	memory_requirements: vk::MemoryRequirements,
}

pub struct Image<'vk_core> {
	logical: LogicalImage<'vk_core>,
	view: vk::ImageView,
	range: Range<vk::DeviceSize>,
}

pub struct MemoryBlock<'vk_core> {
	vk_core: &'vk_core VkCore,
	raw_handle: vk::DeviceMemory,
	size: vk::DeviceSize,
	properties: vk::MemoryPropertyFlags,
	buffers: Vec<Buffer<'vk_core>>,
	images: Vec<Image<'vk_core>>,
}

pub struct MemoryAccess<'vk_core, 'memory> {
	memory_block: &'memory mut MemoryBlock<'vk_core>,
	mapped_memory: &'memory mut [u8],
	map_range: [vk::MappedMemoryRange; 1],
}

#[derive(Debug)]
pub enum MemoryError {
	VkResult(vk::Result),
	MemoryTypeIndexNotFound,
}

impl<'vk_core> LogicalBuffer<'vk_core> {
	pub fn new(
		vk_core: &'vk_core VkCore,
		size: vk::DeviceSize,
		usage: vk::BufferUsageFlags,
		sharing_mode: vk::SharingMode,
	) -> Result<Self, vk::Result> {
		unsafe {
			let raw_handle = vk_core.device.create_buffer(
				&vk::BufferCreateInfo {
					s_type: StructureType::BUFFER_CREATE_INFO,
					p_next: ptr::null(),
					flags: vk::BufferCreateFlags::empty(),
					size,
					usage,
					sharing_mode,
					queue_family_index_count: vk_core.physical_device.queue_family_index_count(),
					p_queue_family_indices: vk_core.physical_device.queue_family_index_ptr(),
				},
				None,
			)?;

			let memory_requirements = vk_core.device.get_buffer_memory_requirements(raw_handle);

			Ok(
				Self {
					vk_core,
					raw_handle,
					memory_requirements,
				}
			)
		}
	}

	#[inline]
	pub fn raw_handle(&self) -> vk::Buffer {
		self.raw_handle
	}
}

impl Drop for LogicalBuffer<'_> {
	fn drop(&mut self) { unsafe { self.vk_core.device.destroy_buffer(self.raw_handle, None); } }
}

impl<'vk_core> Buffer<'vk_core> {
	#[inline]
	pub fn raw_handle(&self) -> vk::Buffer { self.logical.raw_handle }

	#[inline]
	pub fn size(&self) -> vk::DeviceSize { self.range.end - self.range.start }

	pub fn barrier<R>(
		&self,
		access: (vk::AccessFlags, vk::AccessFlags),
		range: R,
	) -> vk::BufferMemoryBarrier
		where R: RangeBounds<vk::DeviceSize>
	{
		let offset = match range.start_bound() {
			Bound::Included(&n) => n,
			Bound::Excluded(&n) => n + 1,
			Bound::Unbounded => self.range.start,
		};
		let end = match range.end_bound() {
			Bound::Included(&n) => n,
			Bound::Excluded(&n) => n - 1,
			Bound::Unbounded => self.range.end,
		};

		debug_assert!(self.range.start < offset);
		debug_assert!(end < self.range.end);

		vk::BufferMemoryBarrier {
			s_type: StructureType::BUFFER_MEMORY_BARRIER,
			p_next: ptr::null(),
			src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
			dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
			src_access_mask: access.0,
			dst_access_mask: access.1,
			buffer: self.raw_handle(),
			offset,
			size: end - offset,
		}
	}
}

impl<'vk_core> LogicalImage<'vk_core> {
	pub fn new(
		vk_core: &'vk_core VkCore,
		image_type: vk::ImageType,
		extent: vk::Extent3D,
		format: vk::Format,
		usage: vk::ImageUsageFlags,
		sharing_mode: vk::SharingMode,
		initial_layout: vk::ImageLayout,
		sample_count: vk::SampleCountFlags,
		aspect_mask: vk::ImageAspectFlags,
		mip_levels: u32,
		array_layers: u32,
	) -> Result<Self, vk::Result> {
		unsafe {
			let image = vk_core.device.create_image(
				&vk::ImageCreateInfo {
					s_type: StructureType::IMAGE_CREATE_INFO,
					p_next: ptr::null(),
					flags: vk::ImageCreateFlags::empty(),
					extent,
					format,
					usage,
					sharing_mode,
					initial_layout,
					samples: sample_count,
					tiling: vk::ImageTiling::OPTIMAL,
					image_type,
					mip_levels,
					array_layers,
					queue_family_index_count: vk_core.physical_device.queue_family_index_count(),
					p_queue_family_indices: vk_core.physical_device.queue_family_index_ptr(),
				},
				None,
			)?;

			let memory_requirements = vk_core.device.get_image_memory_requirements(image);

			Ok(
				Self {
					vk_core,
					raw_handle: image,
					extent,
					format,
					layout: vec![initial_layout; mip_levels as usize],
					aspect_mask,
					mip_levels,
					array_layers,
					memory_requirements,
				}
			)
		}
	}

	#[inline]
	pub fn raw_handle(&self) -> vk::Image { self.raw_handle }

	#[inline]
	pub fn layout(&self, mip_level: u32) -> vk::ImageLayout {
		debug_assert!(mip_level <= self.mip_levels);
		self.layout[mip_level as usize]
	}

	pub fn extent(&self, mip_level: u32) -> vk::Extent3D {
		debug_assert!(mip_level <= self.mip_levels);

		let (width, height) = (0..mip_level)
			.fold((self.extent.width, self.extent.height), |(width, height), _| {
				(width / 2, height / 2)
			});

		vk::Extent3D {
			width,
			height,
			depth: self.extent.depth,
		}
	}

	#[inline]
	pub fn aspect_mask(&self) -> vk::ImageAspectFlags { self.aspect_mask }

	#[inline]
	pub fn mip_levels(&self) -> u32 { self.mip_levels }

	#[inline]
	pub fn array_layers(&self) -> u32 { self.array_layers }

	/// calculate maximum mip level by width and height.
	pub fn maximum_mip_level(extent: vk::Extent3D) -> u32 {
		let vk::Extent3D { width, height, .. } = extent;
		[width, height]
			.iter()
			.map(|&num| (num as f32).log2() as u32)
			.min()
			.unwrap_or(1)
	}

	pub fn load_image_file<P>(path: P) -> Result<(Vec<u8>, vk::Extent3D), image_crate::ImageError>
		where P: AsRef<Path>
	{
		let image = image_crate::open(path)?.to_rgba();
		let (width, height) = image.dimensions();
		let extent = vk::Extent3D { width, height, depth: 1 };
		let bytes = image.into_raw();
		Ok((bytes, extent))
	}
}

impl Drop for LogicalImage<'_> {
	fn drop(&mut self) { unsafe { self.vk_core.device.destroy_image(self.raw_handle, None); } }
}

impl<'vk_core> Image<'vk_core> {
	#[inline]
	pub fn raw_handle(&self) -> vk::Image { self.logical.raw_handle }

	#[inline]
	pub fn view(&self) -> vk::ImageView { self.view }

	#[inline]
	pub fn layout(&self, mip_level: u32) -> vk::ImageLayout { self.logical.layout(mip_level) }

	#[inline]
	pub fn extent(&self, mip_level: u32) -> vk::Extent3D { self.logical.extent(mip_level) }

	#[inline]
	pub fn extent_tuple(&self, mip_level: u32) -> (i32, i32, i32) {
		let extent = self.logical.extent(mip_level);
		(extent.width as i32, extent.height as i32, extent.depth as i32)
	}

	#[inline]
	pub fn aspect_mask(&self) -> vk::ImageAspectFlags { self.logical.aspect_mask }

	#[inline]
	pub fn mip_levels(&self) -> u32 { self.logical.mip_levels }

	#[inline]
	pub fn array_layers(&self) -> u32 { self.logical.array_layers }

	#[inline]
	pub fn transit_layout(
		&mut self,
		mip_level: u32,
		new_layout: vk::ImageLayout,
	) -> vk::ImageLayout {
		mem::replace(&mut self.logical.layout[mip_level as usize], new_layout)
	}

	pub fn barrier<Rm, Ra>(
		&mut self,
		mip_level_range: &Rm,
		array_layer_range: &Ra,
		new_layout: vk::ImageLayout,
		access: (vk::AccessFlags, vk::AccessFlags),
	) -> (&mut Self, vk::ImageMemoryBarrier)
		where Rm: RangeBounds<u32>,
			  Ra: RangeBounds<u32>,
	{
		let base_mip_level = match mip_level_range.start_bound() {
			Bound::Included(&n) => n,
			Bound::Excluded(&n) => n + 1,
			Bound::Unbounded => 0,
		};
		let end_mip_level = match mip_level_range.end_bound() {
			Bound::Included(&n) => n,
			Bound::Excluded(&n) => n - 1,
			Bound::Unbounded => self.logical.mip_levels,
		};

		let base_array_layer = match array_layer_range.start_bound() {
			Bound::Included(&n) => n,
			Bound::Excluded(&n) => n + 1,
			Bound::Unbounded => 0,
		};

		let end_array_layer = match array_layer_range.end_bound() {
			Bound::Included(&n) => n,
			Bound::Excluded(&n) => n - 1,
			Bound::Unbounded => self.logical.array_layers,
		};

		debug_assert!(end_mip_level < self.logical.mip_levels);
		debug_assert!(end_array_layer < self.logical.array_layers);

		let barrier_info = vk::ImageMemoryBarrier {
			s_type: StructureType::BUFFER_MEMORY_BARRIER,
			p_next: ptr::null(),
			src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
			dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
			src_access_mask: access.0,
			dst_access_mask: access.1,
			image: self.raw_handle(),
			old_layout: self.layout(base_mip_level),
			new_layout,
			subresource_range: vk::ImageSubresourceRange {
				aspect_mask: self.aspect_mask(),
				base_mip_level,
				level_count: end_mip_level - base_mip_level,
				base_array_layer,
				layer_count: end_array_layer - base_array_layer,
			},
		};

		(self, barrier_info)
	}
}

impl Drop for Image<'_> {
	fn drop(&mut self) {
		unsafe { self.logical.vk_core.device.destroy_image_view(self.view, None); }
	}
}

impl<'vk_core> MemoryBlock<'vk_core> {
	pub fn new(
		vk_core: &'vk_core VkCore,
		logical_buffers: Vec<LogicalBuffer<'vk_core>>,
		logical_images: Vec<(LogicalImage<'vk_core>, vk::ImageViewType, vk::ComponentMapping)>,
		memory_properties: vk::MemoryPropertyFlags,
	) -> Result<Self, MemoryError> {
		unsafe {
			let mut mem_reqs = Vec::with_capacity(logical_buffers.len() + logical_images.len());
			logical_buffers
				.iter()
				.for_each(|logical_buffer| mem_reqs.push(logical_buffer.memory_requirements));
			logical_images
				.iter()
				.for_each(|(logical_image, _, _)| mem_reqs.push(logical_image.memory_requirements));


			let (allocation_size, required_memory_type_bits) = mem_reqs
				.into_iter()
				.fold((0, !0), |(size, mem_ty), mem_req| {
					(size + mem_req.size, mem_ty & mem_req.memory_type_bits)
				});

			debug_assert!(allocation_size > 0);
			debug_assert_ne!(required_memory_type_bits, 0);

			let raw_handle = vk_core.device
				.allocate_memory(
					&vk::MemoryAllocateInfo {
						s_type: StructureType::MEMORY_ALLOCATE_INFO,
						p_next: ptr::null(),
						allocation_size,
						memory_type_index: find_memory_type_index(
							&vk_core.physical_device,
							required_memory_type_bits,
							memory_properties,
						)?,
					},
					None,
				)
				.map_err(|vk_err| MemoryError::VkResult(vk_err))?;

			let mut memory_block = Self {
				vk_core,
				raw_handle,
				size: allocation_size,
				properties: memory_properties,
				buffers: Vec::with_capacity(logical_buffers.len()),
				images: Vec::with_capacity(logical_images.len()),
			};

			let mut offset = 0;
			for logical_buffer in logical_buffers.into_iter() {
				let buffer = memory_block
					.bind_buffer(logical_buffer, &mut offset)
					.map_err(|vk_err| MemoryError::VkResult(vk_err))?;
				memory_block.buffers.push(buffer);
			}
			for (logical_image, view_type, components) in logical_images.into_iter() {
				let image = memory_block
					.bind_image(
						logical_image,
						view_type,
						components,
						&mut offset,
					)
					.map_err(|vk_err| MemoryError::VkResult(vk_err))?;
				memory_block.images.push(image);
			}

			Ok(memory_block)
		}
	}

	unsafe fn bind_buffer(
		&self,
		logical_buffer: LogicalBuffer<'vk_core>,
		offset: &mut vk::DeviceSize,
	) -> Result<Buffer<'vk_core>, vk::Result> {
		let range = *offset..*offset + logical_buffer.memory_requirements.size;
		debug_assert!(range.end <= self.size);
		self.vk_core.device
			.bind_buffer_memory(logical_buffer.raw_handle, self.raw_handle, *offset)?;

		*offset = range.end;
		Ok(
			Buffer {
				logical: logical_buffer,
				range,
			}
		)
	}

	unsafe fn bind_image(
		&self,
		logical_image: LogicalImage<'vk_core>,
		view_type: vk::ImageViewType,
		components: vk::ComponentMapping,
		offset: &mut vk::DeviceSize,
	) -> Result<Image<'vk_core>, vk::Result> {
		let range = *offset..*offset + logical_image.memory_requirements.size;
		debug_assert!(range.end <= self.size);
		self.vk_core.device
			.bind_image_memory(logical_image.raw_handle, self.raw_handle, *offset)?;
		let view = self.vk_core.device
			.create_image_view(
				&vk::ImageViewCreateInfo {
					s_type: StructureType::IMAGE_VIEW_CREATE_INFO,
					p_next: ptr::null(),
					flags: vk::ImageViewCreateFlags::empty(),
					image: logical_image.raw_handle,
					format: logical_image.format,
					subresource_range: vk::ImageSubresourceRange {
						aspect_mask: logical_image.aspect_mask,
						base_mip_level: 0,
						level_count: logical_image.mip_levels,
						base_array_layer: 0,
						layer_count: logical_image.array_layers,
					},
					view_type,
					components,
				},
				None,
			)?;

		*offset = range.end;
		Ok(
			Image {
				logical: logical_image,
				view,
				range,
			}
		)
	}

	pub fn buffer_access<'memory, R>(
		&'memory mut self,
		idx: usize,
		map_range: R,
	) -> Result<MemoryAccess<'vk_core, 'memory>, vk::Result>
		where R: RangeBounds<vk::DeviceSize>
	{
		unsafe {
			let map_range_start = match map_range.start_bound() {
				Bound::Included(&n) => n,
				Bound::Excluded(&n) => n + 1,
				Bound::Unbounded => 0,
			};
			let map_range_end = match map_range.end_bound() {
				Bound::Included(&n) => n,
				Bound::Excluded(&n) => n - 1,
				Bound::Unbounded => 0,
			};
			debug_assert!(map_range_end <= self.ref_buffer(idx).size());

			let size = map_range_end - map_range_start;
			let ptr = self.vk_core.device
				.map_memory(self.raw_handle, map_range_start, size, vk::MemoryMapFlags::empty(), )?
				as *mut u8;

			let map_range = [vk::MappedMemoryRange {
				s_type: StructureType::MAPPED_MEMORY_RANGE,
				p_next: ptr::null(),
				memory: self.raw_handle,
				offset: map_range_start,
				size,
			}];

			Ok(
				MemoryAccess {
					memory_block: self,
					mapped_memory: slice::from_raw_parts_mut(ptr, size as usize),
					map_range,
				}
			)
		}
	}

	#[inline]
	pub fn ref_buffer(&self, idx: usize) -> &Buffer { &self.buffers[idx] }
	#[inline]
	pub fn ref_image(&self, idx: usize) -> &Image { &self.images[idx] }
	#[inline]
	pub fn buffer_iter(&self) -> slice::Iter<Buffer> { self.buffers.iter() }
	#[inline]
	pub fn image_iter(&self) -> slice::Iter<Image> { self.images.iter() }
	#[inline]
	pub fn image_iter_mut(&mut self) -> slice::IterMut<Image<'vk_core>> { self.images.iter_mut() }
}

impl Drop for MemoryBlock<'_> {
	fn drop(&mut self) {
		unsafe {
			self.vk_core.device.free_memory(self.raw_handle, None);
		}

		eprintln!("Dropping Memory.");
	}
}

impl io::Read for MemoryAccess<'_, '_> {
	fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
		Ok(io::Read::read(&mut &self.mapped_memory[..], buf)?)
	}
}

impl io::Write for MemoryAccess<'_, '_> {
	fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
		Ok(io::Write::write(&mut self.mapped_memory, buf)?)
	}

	fn flush(&mut self) -> io::Result<()> {
		unsafe {
			self.memory_block.vk_core.device
				.flush_mapped_memory_ranges(&self.map_range)
				.map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
			self.memory_block.vk_core.device
				.invalidate_mapped_memory_ranges(&self.map_range)
				.map_err(|err| io::Error::new(io::ErrorKind::Other, err))
		}
	}
}

impl Drop for MemoryAccess<'_, '_> {
	fn drop(&mut self) {
		unsafe { self.memory_block.vk_core.device.unmap_memory(self.memory_block.raw_handle); }
	}
}

impl fmt::Display for MemoryError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			MemoryError::MemoryTypeIndexNotFound
				=> write!(f, "Not found suitable memory type index in physical device."),
			MemoryError::VkResult(ref err) => write!(f, "vk::Result error: {}", err),
		}
	}
}

impl Error for MemoryError {}

fn find_memory_type_index(
	physical_device: &PhysicalDevice,
	required_memory_type_bits: u32,
	required_memory_property_flags: vk::MemoryPropertyFlags,
) -> Result<u32, MemoryError> {
	for i in 0..physical_device.memory_properties.memory_type_count {
		if required_memory_type_bits & 1 << i != 0
			&& physical_device.memory_properties.memory_types[i as usize].property_flags
			& required_memory_property_flags
			== required_memory_property_flags
		{
			return Ok(i);
		}
	}

	Err(MemoryError::MemoryTypeIndexNotFound)
}
