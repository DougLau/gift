// lib.rs      gift crate.
//
// Copyright (c) 2019-2020  Douglas Lau
//
//! # GIF*t*
//!
//! A library for decoding and encoding GIF images and animations.
//!
//! ## Decode Example
//! ```
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # let gif = &[
//! #   0x47, 0x49, 0x46, 0x38, 0x39, 0x61, 0x02, 0x00,
//! #   0x02, 0x00, 0x80, 0x01, 0x00, 0x00, 0x00, 0x00,
//! #   0xff, 0xff, 0xff, 0x2c, 0x00, 0x00, 0x00, 0x00,
//! #   0x02, 0x00, 0x02, 0x00, 0x00, 0x02, 0x03, 0x0c,
//! #   0x10, 0x05, 0x00, 0x3b,
//! # ][..];
//! // ... open a `File` as "gif"
//! for raster in gift::Decoder::new(gif) {
//!     // was there a decoding error?
//!     let raster = raster?;
//!     // ... work with raster
//! }
//! # Ok(())
//! # }
//! ```
//!
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/DougLau/gift/master/res/gift_logo.gif"
)]
#![forbid(unsafe_code)]

#[macro_use]
extern crate log;

pub mod block;
pub mod decode;
pub mod encode;
mod error;
mod private;

pub use crate::error::Error;
pub use crate::private::{Decoder, Encoder};
