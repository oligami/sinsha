//! Implemented traits are below.
//! DeviceLocal
//! HostVisible
//! HostCoherent
//! HostCached
//! LazilyAllocated
//! Protected
//!
//! Each trait represents each flag below.
//! DEVICE_LOCAL
//! HOST_VISIBLE
//! HOST_COHERENT
//! HOST_CACHED
//! LAZILY_ALLOCATED
//! PROTECTED


use ash::vk::MemoryPropertyFlags;

pub trait MemoryProperty {
	fn memory_property() -> MemoryPropertyFlags;
}

pub struct Empty;
impl MemoryProperty for Empty {
	fn memory_property() -> MemoryPropertyFlags { MemoryPropertyFlags::empty() }
}

macro_rules! impl_memory_property {
	($($property_flag:ident, $flag:ident,)*) => {
		$(
			pub struct $property_flag<P>(pub P) where P: MemoryProperty;

			impl<P> MemoryProperty for $property_flag<P> where P: MemoryProperty {
				fn memory_property() -> MemoryPropertyFlags {
					MemoryPropertyFlags::$flag | P::memory_property()
				}
			}
		)*
	};
}

impl_memory_property!(
	DeviceLocalFlag, DEVICE_LOCAL,
	HostVisibleFlag, HOST_VISIBLE,
	HostCoherentFlag, HOST_COHERENT,
	HostCachedFlag, HOST_CACHED,
	LazilyAllocatedFlag, LAZILY_ALLOCATED,
	ProtectedFlag, PROTECTED,
);


macro_rules! impl_property_trait {
	($property_flag:ident, $property:ident, $not_trait:ident, $($other_flag:ident,)*) => {
		pub trait $property: MemoryProperty {}
		pub trait $not_trait: MemoryProperty {}

		impl<P> $property for $property_flag<P> where P: $not_trait {}
		$(impl<P> $property for $other_flag<P> where P: $property {})*

		impl $not_trait for Empty {}
		$(impl<P> $not_trait for $other_flag<P> where P: $not_trait {})*
	};
}


impl_property_trait!(
	DeviceLocalFlag,
	DeviceLocal,
	NotDeviceLocal,
		// DeviceLocalFlag,
		HostVisibleFlag,
		HostCoherentFlag,
		HostCachedFlag,
		LazilyAllocatedFlag,
		ProtectedFlag,
);
impl_property_trait!(
	HostVisibleFlag,
	HostVisible,
	NotHostVisible,
		DeviceLocalFlag,
		// HostVisibleFlag,
		HostCoherentFlag,
		HostCachedFlag,
		LazilyAllocatedFlag,
		ProtectedFlag,
);
impl_property_trait!(
	HostCoherentFlag,
	HostCoherent,
	NotHostCoherent,
		DeviceLocalFlag,
		HostVisibleFlag,
		// HostCoherentFlag,
		HostCachedFlag,
		LazilyAllocatedFlag,
		ProtectedFlag,
);
impl_property_trait!(
	HostCachedFlag,
	HostCached,
	NotHostCached,
		DeviceLocalFlag,
		HostVisibleFlag,
		HostCoherentFlag,
		// HostCachedFlag,
		LazilyAllocatedFlag,
		ProtectedFlag,
);
impl_property_trait!(
	LazilyAllocatedFlag,
	LazilyAllocated,
	NotLazilyAllocated,
		DeviceLocalFlag,
		HostVisibleFlag,
		HostCoherentFlag,
		HostCachedFlag,
		// LazilyAllocatedFlag,
		ProtectedFlag,
);
impl_property_trait!(
	ProtectedFlag,
	Protected,
	NotProtected,
		DeviceLocalFlag,
		HostVisibleFlag,
		HostCoherentFlag,
		HostCachedFlag,
		LazilyAllocatedFlag,
		// ProtectedFlag,
);