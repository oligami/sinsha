use ash::vk;

use crate::vulkan::*;

use std::path::*;
use std::io::Write;
use std::error::Error;

pub fn load_gui<'vk_core, 'memory>(
	vk_core: &'vk_core VkCore,
	command_recorder: &mut CommandRecorder<'vk_core, '_>,
) -> Result<gui::Rect2Ds<'vk_core, 'memory>, Box<dyn Error>> {
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

	let mut buffer_access = staging_buffer.buffer_access(0, 0..size)?;

	for bytes in bytes_of_images.into_iter() {
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
				..,
				..,
				vk::ImageLayout::TRANSFER_DST_OPTIMAL,
				(vk::AccessFlags::empty(), vk::AccessFlags::TRANSFER_WRITE),
			);
		image_barriers.push(barrier);
	}

	command_recorder
		.barriers(
			(vk::PipelineStageFlags::TOP_OF_PIPE, vk::PipelineStageFlags::TRANSFER),
			&[],
			image_barriers,
		);

	for image in images.image_iter() {
		command_recorder
			.buffer_to_image(
				staging_buffer.ref_buffer(0),
				(image, 0),
				&[
					vk::BufferImageCopy {
						buffer_offset: unimplemented!(),
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
	}


	unimplemented!()
}