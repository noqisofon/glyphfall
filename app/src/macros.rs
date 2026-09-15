//! `glyphfall-core`の型をBevyの`Resource`として載せるためのニュータイプを
//! 生成するマクロ。孤児則によりcoreの型へ直接`Resource`を実装できないため、
//! `XxxRes(pub CoreType)` + `Deref`/`DerefMut`という定型文を各所で手書きして
//! いたが、コアの型が増えるたびに増殖するのでここに畳んでおく。
macro_rules! newtype_resource {
    ($name:ident, $inner:path) => {
        #[derive(bevy::prelude::Resource)]
        pub struct $name(pub $inner);

        impl std::ops::Deref for $name {
            type Target = $inner;
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl std::ops::DerefMut for $name {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }
    };
    ($name:ident, $inner:path, default) => {
        newtype_resource!($name, $inner);

        impl Default for $name {
            fn default() -> Self {
                Self(Default::default())
            }
        }
    };
}

pub(crate) use newtype_resource;
