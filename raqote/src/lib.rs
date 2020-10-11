//! A [`raqote`] renderer for [`iced_native`].
//!
//! [`raqote`]: https://github.com/jrmuizel/raqote
//! [`iced_native`]: https://github.com/hecrj/iced/tree/master/native
#![deny(missing_docs)]
#![deny(missing_debug_implementations)]
#![deny(unused_results)]
#![forbid(rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg))]

mod backend;

pub mod settings;
pub mod widget;
#[cfg(feature = "blit")]
pub mod window;

pub use backend::Backend;
pub use settings::Settings;

#[doc(no_inline)]
pub use widget::*;

pub use iced_graphics::Viewport;
pub use iced_native::*;

pub use raqote;

/// A [`raqote`] graphics renderer for [`iced`].
///
/// [`raqote`]: https://github.com/jrmuizel/raqote
/// [`iced`]: https://github.com/hecrj/iced
pub type Renderer = iced_graphics::Renderer<Backend>;
