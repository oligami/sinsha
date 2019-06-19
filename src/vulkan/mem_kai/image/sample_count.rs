use ash::vk::SampleCountFlags;
pub trait SampleCount {
	fn sample_count() -> SampleCountFlags;
}

macro_rules! impl_sample_count {
		($($sample_count:ident, $flag:expr),*) => {
			$(
				pub struct $sample_count;
				impl SampleCount for $sample_count {
					fn sample_count() -> SampleCountFlags { $flag }
				}
			)*
		}
	}

impl_sample_count!(
		Type1, SampleCountFlags::TYPE_1,
		Type2, SampleCountFlags::TYPE_2,
		Type4, SampleCountFlags::TYPE_4,
		Type8, SampleCountFlags::TYPE_8,
		Type16, SampleCountFlags::TYPE_16,
		Type32, SampleCountFlags::TYPE_32,
		Type64, SampleCountFlags::TYPE_64
	);