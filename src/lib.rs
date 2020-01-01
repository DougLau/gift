// lib.rs      gift crate.
//
// Copyright (c) 2019  Douglas Lau
//
//! # GIF*t*
//!
//! A library for encoding and decoding GIF images and animations.
//!
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/DougLau/gift/master/res/gift_logo.gif"
)]
#![forbid(unsafe_code)]

#[macro_use]
extern crate log;

pub mod block;
mod decode;
mod encode;
mod error;

pub use crate::decode::{BlockDecoder, Decoder, FrameDecoder, RasterDecoder};
pub use crate::encode::{Encoder, FrameEncoder};
pub use crate::error::{DecodeError, EncodeError};
