#[doc(hidden)]
macro_rules! __byte_array_newtype {
    ($name:ident, $length:expr, $type:ty) => {
        #[derive(Clone)]
        struct $name($type);

        __byte_array_newtype_impl!($name, $length, $type);
    };
    (pub $name:ident, $length:expr, $type:ty) => {
        #[derive(Clone)]
        pub struct $name($type);

        __byte_array_newtype_impl!($name, $length, $type);
    };
    (pub(crate) $name:ident, $length:expr, $type:ty) => {
        #[derive(Clone)]
        pub(crate) struct $name($type);

        __byte_array_newtype_impl!($name, $length, $type);
    };
}


#[doc(hidden)]
macro_rules! __byte_array_newtype_impl {
    ($name:ident, $length:expr, $type:ty) => {
        impl $name {

            #[allow(unused)]
            pub fn to_bytes(self) -> $type {
                self.into()
            }

            #[allow(unused)]
            pub fn as_bytes(&self) -> &$type {
                self.as_ref()
            }

            #[allow(unused)]
            pub fn as_slice(&self) -> &[u8] {
                self.as_ref()
            }

            #[allow(unused)]
            fn from_bytes(bytes: &[u8]) -> Result<$name, crate::Error> {
                if bytes.len() != $length {
                    let err = crate::Error::BadLengthError {
                        name: stringify!($name),
                        actual: bytes.len(),
                        expected: $length,
                    };
                    return Err(err);
                }
                let mut result = Self::default();
                result.0.copy_from_slice(bytes);
                Ok(result)
            }
        }

        impl Default for $name {
            #[allow(unused)]
            fn default() -> $name {
                $name([0; $length])
            }
        }

        impl From<$type> for $name {
            #[allow(unused)]
            fn from(inner: $type) -> $name {
                $name(inner)
            }
        }

        impl Into<$type> for $name {
            #[allow(unused)]
            fn into(self) -> $type {
                self.0
            }
        }

        impl AsRef<$type> for $name {
            #[allow(unused)]
            fn as_ref(&self) -> &$type {
                &self.0
            }
        }

        impl AsRef<[u8]> for $name {
            #[allow(unused)]
            fn as_ref(&self) -> &[u8] {
                &self.0
            }
        }

    };
}


