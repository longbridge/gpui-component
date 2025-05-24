/// A physical pixel unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PhysicalUnit<T>(pub T);

macro_rules! impl_to_logical {
    ($($unit:ty),*) => {
        $(
            impl PhysicalUnit<$unit> {
                /// Converts the physical unit to a logical unit using the device scale
                /// factor.
                #[inline]
                pub fn to_logical(&self, device_scale_factor: f32) -> LogicalUnit<$unit> {
                    LogicalUnit((self.0 as f32 / device_scale_factor) as $unit)
                }
            }
        )*
    };
    () => {};
}

impl_to_logical!(i32, u32, i64, u64, f32, f64);

/// A logical pixel unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LogicalUnit<T>(pub T);

macro_rules! impl_to_physical {
    ($($unit:ty),*) => {
        $(
            impl LogicalUnit<$unit> {
                /// Converts the logical unit to a physical unit using the device scale
                /// factor.
                #[inline]
                pub fn to_physical(&self, device_scale_factor: f32) -> PhysicalUnit<$unit> {
                    PhysicalUnit((self.0 as f32 * device_scale_factor) as $unit)
                }
            }
        )*
    };
    () => {};
}

impl_to_physical!(i32, u32, i64, u64, f32, f64);
