mod data;

use data::*;

use ash::vk;

use crate::vulkan::*;
use crate::linear_algebra::*;

use std::mem;
use std::path::*;
use std::io::Write;
use std::error::Error;

pub fn load_gui<'vk_core, 'memory>(
	vk_core: &'vk_core VkCore,
	command_recorder: &mut CommandRecorder<'vk_core, '_>,
) -> Result<(), Box<dyn Error>> {
	let texture_path_root = "assets/textures";

	let texture_pathes = [
		[texture_path_root, "info_box.png"].iter().collect::<PathBuf>(),
	];
	let _font = "assets/font/friz_quadrata.png";


	let n = texture_pathes.len();
	let (mut bytes_of_images, mut logical_images) = (Vec::with_capacity(n), Vec::with_capacity(n));
	for path in texture_pathes.iter() {
		let (bytes, extent) = LogicalImage::load_image_file(path)?;
		let logical_image = LogicalImage::new(
			vk_core,
			vk::ImageType::TYPE_2D,
			extent,
			vk::Format::R8G8B8A8_UNORM,
			vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST,
			vk::SharingMode::EXCLUSIVE,
			vk::ImageLayout::UNDEFINED,
			vk::SampleCountFlags::TYPE_1,
			vk::ImageAspectFlags::COLOR,
			1, 1,
		)?;

		bytes_of_images.push(bytes);
		logical_images
			.push((logical_image, vk::ImageViewType::TYPE_2D, vk::ComponentMapping::default()));
	}
	let (bytes_of_images, logical_images) = (bytes_of_images, logical_images);

	let size: usize = bytes_of_images.iter().map(|bytes| bytes.len()).sum();
	let size = size as vk::DeviceSize;
	let staging_logical_buffer = LogicalBuffer::new(
		vk_core,
		size,
		vk::BufferUsageFlags::TRANSFER_SRC,
		vk::SharingMode::EXCLUSIVE,
	)?;

	let mut staging_buffer = MemoryBlock::new(
		vk_core,
		vec![staging_logical_buffer],
		vec![],
		vk::MemoryPropertyFlags::HOST_VISIBLE,
	)?;

	let mut buffer_access = staging_buffer.buffer_access(0, ..)?;

	for bytes in bytes_of_images.iter() {
		debug_assert_eq!(bytes.len(), buffer_access.write(&bytes[..])?);
	}
	buffer_access.flush()?;

	drop(buffer_access);

	let mut images = MemoryBlock::new(
		vk_core,
		vec![],
		logical_images,
		vk::MemoryPropertyFlags::DEVICE_LOCAL,
	)?;

	let mut image_barriers = Vec::with_capacity(n);
	for image in images.image_iter_mut() {
		let barrier = image
			.barrier(
				&(..),
				&(..),
				vk::ImageLayout::TRANSFER_DST_OPTIMAL,
				(vk::AccessFlags::empty(), vk::AccessFlags::TRANSFER_WRITE),
			);
		image_barriers.push(barrier);
	}

	eprintln!("still work.");

	command_recorder
		.barriers(
			(vk::PipelineStageFlags::TOP_OF_PIPE, vk::PipelineStageFlags::TRANSFER),
			&[],
			&mut image_barriers[..],
		);

	let mut offset = 0;
	for (image, bytes) in images.image_iter().zip(bytes_of_images.iter()) {
		command_recorder
			.buffer_to_image(
				staging_buffer.buffer_ref(0),
				(image, 0),
				&[
					vk::BufferImageCopy {
						buffer_offset: offset,
						buffer_row_length: 0,
						buffer_image_height: 0,
						image_offset: vk::Offset3D { x: 0, y: 0, z: 0 },
						image_extent: image.extent(0),
						image_subresource: vk::ImageSubresourceLayers {
							aspect_mask: image.aspect_mask(),
							mip_level: 0,
							base_array_layer: 0,
							layer_count: image.array_layers(),
						}
					}
				]
			);

		offset += offset + bytes.len() as u64;
	}

	for image in images.image_iter_mut() {
		for mip_level in 0..image.mip_levels() {
			let (src_mip_level, dst_mip_level) = (mip_level, mip_level + 1);
			let array_layer_range = 0..image.array_layers();
			let barrier = image.barrier(
				&(src_mip_level..dst_mip_level),
				&array_layer_range,
				vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
				(vk::AccessFlags::TRANSFER_WRITE, vk::AccessFlags::TRANSFER_READ)
			);

			command_recorder
				.barriers(
					(vk::PipelineStageFlags::empty(), vk::PipelineStageFlags::empty()),
					&[],
					&mut [barrier],
				)
				.blit_image(
					(image, image),
					(src_mip_level, dst_mip_level),
					(&array_layer_range, &array_layer_range),
					(
						&((0, 0, 0)..image.extent_tuple(src_mip_level)),
						&((0, 0, 0)..image.extent_tuple(dst_mip_level))
					),
					vk::Filter::LINEAR,
				);
		}
	}

	for image in images.image_iter_mut() {
		let array_layer_range = 0..image.array_layers();
		let mip_level_range = 0..image.mip_levels();
		let barrier = image.barrier(
			&mip_level_range,
			&array_layer_range,
			vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
			(
				vk::AccessFlags::TRANSFER_READ | vk::AccessFlags::TRANSFER_WRITE,
				vk::AccessFlags::SHADER_READ
			),
		);

		command_recorder
			.barriers(
				(vk::PipelineStageFlags::TRANSFER, vk::PipelineStageFlags::FRAGMENT_SHADER),
				&[],
				&mut [barrier]
			);
	}

	let start_buttom = START_BUTTON;
	let logical_buffer = LogicalBuffer::new(
		vk_core,
		mem::size_of_val(&start_buttom) as _,
		vk::BufferUsageFlags::VERTEX_BUFFER,
		vk::SharingMode::EXCLUSIVE,
	)?;

	let mut buffer_memory = MemoryBlock::new(
		vk_core,
		vec![logical_buffer],
		vec![],
		vk::MemoryPropertyFlags::HOST_VISIBLE,
	)?;

	let mut access = buffer_memory.buffer_access(0, ..)?;

	drop(access);
	Ok(())
}