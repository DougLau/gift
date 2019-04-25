// lib.rs      gift crate.
//
// Copyright (c) 2019  Douglas Lau
//
//! # GIF*t*
//!
//! A decoder and encoder for GIF images.
//!
#[macro_use]
extern crate log;

pub mod block;
mod decode;
mod encode;
mod error;

pub use crate::decode::{Decoder, BlockDecoder, FrameDecoder};
pub use crate::encode::Encoder;
pub use crate::error::DecodeError;
