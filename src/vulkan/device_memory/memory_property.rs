//! Implemented traits are below.
//! - DeviceLocal
//! - HostVisible
//! - HostCoherent
//! - HostCached
//! - LazilyAllocated
//! - Protected
//!
//! Each trait represents each flag below.
//! - DEVICE_LOCAL
//! - HOST_VISIBLE
//! - HOST_COHERENT
//! - HOST_CACHED
//! - LAZILY_ALLOCATED
//! - PROTECTED

pub use inner::{
    MemoryProperties,
    MemoryProperty,
    DeviceLocal,
    HostVisible,
    HostCoherent,
    HostCached,
    LazilyAllocated,
    Protected,
};

mod inner {
    #![allow(unused)]
    use ash::vk::MemoryPropertyFlags;
    use std::marker::PhantomData;

    pub struct MemoryProperties<P>(PhantomData<P>);

    pub trait MemoryProperty {
        #[inline]
        fn memory_property() -> MemoryPropertyFlags;
    }

    pub trait MemoryPropertyFlag {
        #[inline]
        fn flag() -> MemoryPropertyFlags;
    }

    pub struct Not;
    pub struct DeviceLocalFlag;
    pub struct HostVisibleFlag;
    pub struct HostCoherentFlag;
    pub struct HostCachedFlag;
    pub struct LazilyAllocatedFlag;
    pub struct ProtectedFlag;
    pub trait DeviceLocal: MemoryProperty {}
    pub trait HostVisible: MemoryProperty {}
    pub trait HostCoherent: MemoryProperty {}
    pub trait HostCached: MemoryProperty {}
    pub trait LazilyAllocated: MemoryProperty {}
    pub trait Protected: MemoryProperty {}


    impl MemoryProperties<(Not, Not, Not, Not, Not, Not)> {
        #[inline]
        pub fn empty() -> Self { MemoryProperties(PhantomData) }
    }
    impl<P1, P2, P3, P4, P5, P6> MemoryProperty for MemoryProperties<(P1, P2, P3, P4, P5, P6)>
        where P1: MemoryPropertyFlag,
              P2: MemoryPropertyFlag,
              P3: MemoryPropertyFlag,
              P4: MemoryPropertyFlag,
              P5: MemoryPropertyFlag,
              P6: MemoryPropertyFlag,
    {
        fn memory_property() -> MemoryPropertyFlags {
            P1::flag() | P2::flag() | P3::flag() | P4::flag() | P5::flag() | P6::flag()
        }
    }

    impl MemoryPropertyFlag for Not {
        fn flag() -> MemoryPropertyFlags { MemoryPropertyFlags::empty() }
    }
    impl MemoryPropertyFlag for DeviceLocalFlag {
        fn flag() -> MemoryPropertyFlags { MemoryPropertyFlags::DEVICE_LOCAL }
    }
    impl MemoryPropertyFlag for HostVisibleFlag {
        fn flag() -> MemoryPropertyFlags { MemoryPropertyFlags::HOST_VISIBLE }
    }
    impl MemoryPropertyFlag for HostCoherentFlag {
        fn flag() -> MemoryPropertyFlags { MemoryPropertyFlags::HOST_COHERENT }
    }
    impl MemoryPropertyFlag for HostCachedFlag {
        fn flag() -> MemoryPropertyFlags { MemoryPropertyFlags::HOST_CACHED }
    }
    impl MemoryPropertyFlag for LazilyAllocatedFlag {
        fn flag() -> MemoryPropertyFlags { MemoryPropertyFlags::LAZILY_ALLOCATED }
    }
    impl MemoryPropertyFlag for ProtectedFlag {
        fn flag() -> MemoryPropertyFlags { MemoryPropertyFlags::PROTECTED }
    }



    impl<P1, P2, P3, P4, P5> DeviceLocal for MemoryProperties<(DeviceLocalFlag, P1, P2, P3, P4, P5)>
        where P1: MemoryPropertyFlag,
              P2: MemoryPropertyFlag,
              P3: MemoryPropertyFlag,
              P4: MemoryPropertyFlag,
              P5: MemoryPropertyFlag, {}

    impl<P1, P2, P3, P4, P5> MemoryProperties<(Not, P1, P2, P3, P4, P5)>
        where P1: MemoryPropertyFlag,
              P2: MemoryPropertyFlag,
              P3: MemoryPropertyFlag,
              P4: MemoryPropertyFlag,
              P5: MemoryPropertyFlag,
    {
        #[inline]
        pub fn device_local(self) -> MemoryProperties<(DeviceLocalFlag, P1, P2, P3, P4, P5)> {
            MemoryProperties(PhantomData)
        }
    }

    impl<P1, P2, P3, P4, P5> HostVisible for MemoryProperties<(P1, HostVisibleFlag, P2, P3, P4, P5)>
        where P1: MemoryPropertyFlag,
              P2: MemoryPropertyFlag,
              P3: MemoryPropertyFlag,
              P4: MemoryPropertyFlag,
              P5: MemoryPropertyFlag, {}

    impl<P1, P2, P3, P4, P5> MemoryProperties<(P1, Not, P2, P3, P4, P5)>
        where P1: MemoryPropertyFlag,
              P2: MemoryPropertyFlag,
              P3: MemoryPropertyFlag,
              P4: MemoryPropertyFlag,
              P5: MemoryPropertyFlag,
    {
        #[inline]
        pub fn host_visible(self) -> MemoryProperties<(P1, HostVisibleFlag, P2, P3, P4, P5)> {
            MemoryProperties(PhantomData)
        }
    }

    impl<P1, P2, P3, P4, P5> HostCoherent for MemoryProperties<(P1, P2, HostCoherentFlag, P3, P4, P5)>
        where P1: MemoryPropertyFlag,
              P2: MemoryPropertyFlag,
              P3: MemoryPropertyFlag,
              P4: MemoryPropertyFlag,
              P5: MemoryPropertyFlag, {}

    impl<P1, P2, P3, P4, P5> MemoryProperties<(P1, P2, Not, P3, P4, P5)>
        where P1: MemoryPropertyFlag,
              P2: MemoryPropertyFlag,
              P3: MemoryPropertyFlag,
              P4: MemoryPropertyFlag,
              P5: MemoryPropertyFlag,
    {
        #[inline]
        pub fn host_coherent(self) -> MemoryProperties<(P1, P2, HostCoherentFlag, P3, P4, P5)> {
            MemoryProperties(PhantomData)
        }
    }

    impl<P1, P2, P3, P4, P5> HostCached for MemoryProperties<(P1, P2, P3, HostCachedFlag, P4, P5)>
        where P1: MemoryPropertyFlag,
              P2: MemoryPropertyFlag,
              P3: MemoryPropertyFlag,
              P4: MemoryPropertyFlag,
              P5: MemoryPropertyFlag, {}

    impl<P1, P2, P3, P4, P5> MemoryProperties<(P1, P2, P3, Not, P4, P5)>
        where P1: MemoryPropertyFlag,
              P2: MemoryPropertyFlag,
              P3: MemoryPropertyFlag,
              P4: MemoryPropertyFlag,
              P5: MemoryPropertyFlag,
    {
        #[inline]
        pub fn host_cached(self) -> MemoryProperties<(P1, P2, P3, HostCachedFlag, P4, P5)> {
            MemoryProperties(PhantomData)
        }
    }

    impl<P1, P2, P3, P4, P5> LazilyAllocated for MemoryProperties<(P1, P2, P3, P4, LazilyAllocatedFlag, P5)>
        where P1: MemoryPropertyFlag,
              P2: MemoryPropertyFlag,
              P3: MemoryPropertyFlag,
              P4: MemoryPropertyFlag,
              P5: MemoryPropertyFlag, {}

    impl<P1, P2, P3, P4, P5> MemoryProperties<(P1, P2, P3, P4, Not, P5)>
        where P1: MemoryPropertyFlag,
              P2: MemoryPropertyFlag,
              P3: MemoryPropertyFlag,
              P4: MemoryPropertyFlag,
              P5: MemoryPropertyFlag,
    {
        #[inline]
        pub fn lazily_allocated(self) -> MemoryProperties<(P1, P2, P3, P4, LazilyAllocatedFlag, P5)> {
            MemoryProperties(PhantomData)
        }
    }

    impl<P1, P2, P3, P4, P5> Protected for MemoryProperties<(P1, P2, P3, P4, P5, ProtectedFlag)>
        where P1: MemoryPropertyFlag,
              P2: MemoryPropertyFlag,
              P3: MemoryPropertyFlag,
              P4: MemoryPropertyFlag,
              P5: MemoryPropertyFlag, {}

    impl<P1, P2, P3, P4, P5> MemoryProperties<(P1, P2, P3, P4, P5, Not)>
        where P1: MemoryPropertyFlag,
              P2: MemoryPropertyFlag,
              P3: MemoryPropertyFlag,
              P4: MemoryPropertyFlag,
              P5: MemoryPropertyFlag,
    {
        #[inline]
        pub fn protected(self) -> MemoryProperties<(P1, P2, P3, P4, P5, ProtectedFlag)> {
            MemoryProperties(PhantomData)
        }
    }
}