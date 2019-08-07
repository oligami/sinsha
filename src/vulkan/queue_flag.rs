use ash::vk;

pub trait QueueFlags {
    fn queue_flags() -> vk::QueueFlags;
}

impl<F1, F2> QueueFlags for (F1, F2) where F1: QueueFlags, F2: QueueFlags {
    fn queue_flags() -> vk::QueueFlags { F1::queue_flags() | F2::queue_flags() }
}

#[derive(Copy, Clone)]
pub struct Empty;
impl QueueFlags for Empty {
    fn queue_flags() -> vk::QueueFlags { vk::QueueFlags::empty() }
}

pub struct QueueFlagBuilder<F>(F);
pub fn builder() -> QueueFlagBuilder<Empty> { QueueFlagBuilder(Empty) }
impl<F> QueueFlagBuilder<F> where F: QueueFlags {
    pub fn build_with_surface_support(self) -> (F, SurfaceSupportFlag) {
        (self.0, SurfaceSupportFlag)
    }
    pub fn build_with_no_surface_support(self) -> (F, NoSurfaceSupportFlag) {
        (self.0, NoSurfaceSupportFlag)
    }
}

#[derive(Copy, Clone)]
pub struct SurfaceSupportFlag;
#[derive(Copy, Clone)]
pub struct NoSurfaceSupportFlag;
pub trait SurfaceSupport {}
pub trait NoSurfaceSupport {}
impl<F> SurfaceSupport for (F, SurfaceSupportFlag) where F: QueueFlags {}
impl<F> NoSurfaceSupport for (F, NoSurfaceSupportFlag) where F: QueueFlags {}
impl<F> QueueFlags for (F, SurfaceSupportFlag) where F: QueueFlags {
    fn queue_flags() -> vk::QueueFlags { F::queue_flags() }
}
impl<F> QueueFlags for (F, NoSurfaceSupportFlag) where F: QueueFlags {
    fn queue_flags() -> vk::QueueFlags { F::queue_flags() }

}

macro_rules! impl_queue_flag {
    ($($Flag: ident, $Const: ident, $fn_name: ident, $Trait: ident, $NotTrait: ident, $($OtherFlag: ident,)*,)*) => {$(
        #[derive(Copy, Clone)]
        pub struct $Flag;
        pub trait $Trait: QueueFlags {}

        impl QueueFlags for $Flag {
            fn queue_flags() -> vk::QueueFlags { vk::QueueFlags::$Const }
        }

        pub trait $NotTrait: QueueFlags {}
        $(impl $NotTrait for $OtherFlag {})*

        impl<F> $Trait for (F, $Flag) where F: $NotTrait {}
        impl<F1, F2> $Trait for (F1, F2) where F1: $Trait, F2: $NotTrait {}

        impl $NotTrait for Empty {}
        impl<F1, F2> $NotTrait for (F1, F2) where F1: $NotTrait, F2: $NotTrait {}

        impl<F> QueueFlagBuilder<F> where F: $NotTrait {
            pub fn $fn_name(self) -> QueueFlagBuilder<(F, $Flag)> {
                QueueFlagBuilder((self.0, $Flag))
            }
        }
    )*};
}

impl Transfer for GraphicsFlag {}
impl Transfer for ComputeFlag {}

impl_queue_flag!(
    GraphicsFlag,
    GRAPHICS,
    graphics,
    Graphics,
    NotGraphics,
        // GraphicsFlag,
        ComputeFlag,
        TransferFlag,
        SparseBindingFlag,
        ProtectedFlag,,

    ComputeFlag,
    COMPUTE,
    compute,
    Compute,
    NotCompute,
        GraphicsFlag,
        // ComputeFlag,
        TransferFlag,
        SparseBindingFlag,
        ProtectedFlag,,

    TransferFlag,
    TRANSFER,
    transfer,
    Transfer,
    NotTransfer,
        // GraphicsFlag,
        // ComputeFlag,
        // TransferFlag,
        SparseBindingFlag,
        ProtectedFlag,,

    SparseBindingFlag,
    SPARSE_BINDING,
    sparse_binding,
    SparseBinding,
    NotSparseBinding,
        GraphicsFlag,
        ComputeFlag,
        TransferFlag,
        // SparseBindingFlag,
        ProtectedFlag,,

    ProtectedFlag,
    PROTECTED,
    protected,
    Protected,
    NotProtected,
        GraphicsFlag,
        ComputeFlag,
        TransferFlag,
        SparseBindingFlag,,
        // ProtectedFlag,

);

