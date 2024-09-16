#[macro_export]
macro_rules! define_ref {
    ($ref_name: ident, $inner: ty) => {
        #[derive(Clone)]
        pub struct $ref_name {
            inner: crate::mrc::Mrc<$inner>,
        }

        impl $ref_name {
            pub fn new(inner: $inner) -> Self {
                Self {
                    inner: crate::mrc::Mrc::new(inner)
                }
            }
        }

        impl core::ops::Deref for $ref_name {
            type Target = $inner;

            fn deref(&self) -> &Self::Target {
                &self.inner
            }
        }

        impl core::ops::DerefMut for $ref_name {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.inner
            }
        }
    };
}