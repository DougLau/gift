// lib.rs      gift crate.
//
// Copyright (c) 2019  Douglas Lau
//
//! # GIF*t*
//!
//! A decoder and encoder for GIF images.
//!
#![doc(html_logo_url = "https://raw.githubusercontent.com/DougLau/gift/master/res/gift_logo.gif")]
#![forbid(unsafe_code)]

#[macro_use]
extern crate log;

pub mod block;
mod decode;
mod encode;
mod error;

pub use crate::decode::{Decoder, BlockDecoder, FrameDecoder};
pub use crate::encode::Encoder;
pub use crate::error::DecodeError;
