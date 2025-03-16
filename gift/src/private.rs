// private.rs
//
// Copyright (c) 2019-2025  Douglas Lau
//
//! Private module for top-level items
use crate::{
    Result,
    block::{DisposalMethod, GraphicControl},
    decode, encode,
};
use pix::{Palette, Raster, gray::Gray8, rgb::SRgba8};
use std::io::{Read, Write};

/// Raster for an animation step.
pub(crate) enum StepRaster {
    /// True color 24-bit raster
    TrueColor(Raster<SRgba8>),
    /// Indexed color 8-bit raster
    Indexed(Raster<Gray8>, Palette),
}

/// One step of an animation.
#[derive(Clone)]
pub struct Step {
    /// Raster of the animation step
    pub(crate) raster: StepRaster,
    /// Graphic control for the step
    pub(crate) graphic_control_ext: Option<GraphicControl>,
}

/// GIF file decoder
///
/// Can be converted to one of three `Iterator`s:
/// * [into_iter] / [into_steps] for high-level [Step]s
/// * [into_frames] for mid-level [Frame]s
/// * [into_blocks] for low-level [Block]s
///
/// ## Example: Get a `Raster` from a GIF
/// ```
/// use gift::Decoder;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let gif = &[
/// #   0x47, 0x49, 0x46, 0x38, 0x39, 0x61, 0x02, 0x00,
/// #   0x02, 0x00, 0x80, 0x01, 0x00, 0x00, 0x00, 0x00,
/// #   0xff, 0xff, 0xff, 0x2c, 0x00, 0x00, 0x00, 0x00,
/// #   0x02, 0x00, 0x02, 0x00, 0x00, 0x02, 0x03, 0x0c,
/// #   0x10, 0x05, 0x00, 0x3b,
/// # ][..];
/// // ... open a `File` as "gif"
/// if let Some(step) = Decoder::new(gif).into_steps().next() {
///     // was there a decoding error?
///     let step = step?;
///     let raster = step.raster();
///     // ... work with raster
/// }
/// # Ok(())
/// # }
/// ```
///
/// [Block]: block/enum.Block.html
/// [Frame]: block/struct.Frame.html
/// [into_blocks]: struct.Decoder.html#method.into_blocks
/// [into_frames]: struct.Decoder.html#method.into_frames
/// [into_iter]: struct.Decoder.html#method.into_iter
/// [into_steps]: struct.Decoder.html#method.into_steps
/// [Step]: struct.Step.html
///
pub struct Decoder<R: Read> {
    /// Reader for input data
    reader: R,
    /// Maximum image size, in bytes
    max_image_sz: Option<usize>,
}

impl Clone for StepRaster {
    fn clone(&self) -> Self {
        match self {
            StepRaster::TrueColor(r) => {
                StepRaster::TrueColor(Raster::with_raster(r))
            }
            StepRaster::Indexed(r, p) => {
                StepRaster::Indexed(Raster::with_raster(r), p.clone())
            }
        }
    }
}

impl Step {
    /// Create an animation step with a true color raster.
    pub fn with_true_color(raster: Raster<SRgba8>) -> Self {
        let raster = StepRaster::TrueColor(raster);
        Step {
            raster,
            graphic_control_ext: None,
        }
    }

    /// Create an animation step with an indexed raster.
    pub fn with_indexed(raster: Raster<Gray8>, palette: Palette) -> Self {
        let raster = StepRaster::Indexed(raster, palette);
        Step {
            raster,
            graphic_control_ext: None,
        }
    }

    /// Adjust the disposal method.
    pub fn with_disposal_method(mut self, method: DisposalMethod) -> Self {
        let mut control = self.graphic_control_ext.unwrap_or_default();
        control.set_disposal_method(method);
        if control != GraphicControl::default() {
            self.graphic_control_ext = Some(control);
        } else {
            self.graphic_control_ext = None;
        }
        self
    }

    /// Adjust the transparent color.
    pub fn with_transparent_color(mut self, clr: Option<u8>) -> Self {
        let mut control = self.graphic_control_ext.unwrap_or_default();
        control.set_transparent_color(clr);
        if control != GraphicControl::default() {
            self.graphic_control_ext = Some(control);
        } else {
            self.graphic_control_ext = None;
        }
        self
    }

    /// Get the transparent color
    pub fn transparent_color(&self) -> Option<u8> {
        self.graphic_control_ext.and_then(|c| c.transparent_color())
    }

    /// Adjust the delay time.
    pub fn with_delay_time_cs(mut self, delay: Option<u16>) -> Self {
        let mut control = self.graphic_control_ext.unwrap_or_default();
        control.set_delay_time_cs(delay.unwrap_or_default());
        if control != GraphicControl::default() {
            self.graphic_control_ext = Some(control);
        } else {
            self.graphic_control_ext = None;
        }
        self
    }

    /// Get the raster
    pub fn raster(&self) -> &Raster<SRgba8> {
        match &self.raster {
            StepRaster::TrueColor(r) => r,
            StepRaster::Indexed(_, _) => todo!("convert to true color"),
        }
    }

    /// Get the delay time in centiseconds
    pub fn delay_time_cs(&self) -> Option<u16> {
        self.graphic_control_ext.map(|c| c.delay_time_cs())
    }
}

impl<R: Read> Decoder<R> {
    /// Create a new GIF decoder.
    pub fn new(reader: R) -> Self {
        Decoder {
            reader,
            max_image_sz: Some(1 << 25),
        }
    }

    /// Set the maximum image size (in bytes) to allow for decoding.
    pub fn max_image_sz(mut self, max_image_sz: Option<usize>) -> Self {
        self.max_image_sz = max_image_sz;
        self
    }

    /// Convert into a block `Iterator`.
    pub fn into_blocks(self) -> decode::Blocks<R> {
        decode::Blocks::new(self.reader, self.max_image_sz)
    }

    /// Convert into a frame `Iterator`.
    pub fn into_frames(self) -> decode::Frames<R> {
        decode::Frames::new(self.into_blocks())
    }

    /// Convert into a step `Iterator` without looping.
    pub fn into_steps(self) -> decode::Steps<R> {
        decode::Steps::new_once(self.into_frames())
    }
}

impl<R: Read> IntoIterator for Decoder<R> {
    type Item = Result<Step>;
    type IntoIter = decode::Steps<R>;

    /// Convert into a step `Iterator` with looping
    fn into_iter(self) -> Self::IntoIter {
        decode::Steps::new_looping(self.into_frames())
    }
}

/// GIF file encoder
///
/// Can be converted to one of three encoders:
/// * [into_step_enc] for high-level [Step]s
/// * [into_frame_enc] for mid-level [Frame]s
/// * [into_block_enc] for low-level [Block]s
///
/// ## Encoding Example
/// ```
/// use gift::{Encoder, Step};
/// use pix::{gray::Gray8, Palette, Raster, rgb::SRgb8};
/// use std::error::Error;
/// use std::io::Write;
///
/// fn encode<W: Write>(mut w: W) -> Result<(), Box<dyn Error>> {
///     let mut enc = Encoder::new(&mut w).into_step_enc();
///     let mut raster = Raster::with_clear(4, 4);
///     *raster.pixel_mut(0, 0) = Gray8::new(1);
///     *raster.pixel_mut(1, 1) = Gray8::new(1);
///     *raster.pixel_mut(2, 2) = Gray8::new(1);
///     *raster.pixel_mut(3, 3) = Gray8::new(1);
///     let mut palette = Palette::new(2);
///     palette.set_entry(SRgb8::new(0xFF, 0, 0));
///     palette.set_entry(SRgb8::new(0xFF, 0xFF, 0));
///     let step = Step::with_indexed(raster, palette);
///     enc.encode_step(&step)?;
///     Ok(())
/// }
/// ```
///
/// [Block]: block/enum.Block.html
/// [Frame]: block/struct.Frame.html
/// [into_block_enc]: struct.Encoder.html#method.into_block_enc
/// [into_frame_enc]: struct.Encoder.html#method.into_frame_enc
/// [into_step_enc]: struct.Encoder.html#method.into_step_enc
/// [Step]: struct.Step.html
pub struct Encoder<W: Write> {
    /// Writer for output data
    writer: W,
}

impl<W: Write> Encoder<W> {
    /// Create a new GIF encoder.
    pub fn new(writer: W) -> Self {
        Encoder { writer }
    }

    /// Convert into a block encoder.
    pub fn into_block_enc(self) -> encode::BlockEnc<W> {
        encode::BlockEnc::new(self.writer)
    }

    /// Convert into a frame encoder.
    pub fn into_frame_enc(self) -> encode::FrameEnc<W> {
        encode::FrameEnc::new(self.into_block_enc())
    }

    /// Convert into a step encoder.
    pub fn into_step_enc(self) -> encode::StepEnc<W> {
        encode::StepEnc::new(self.into_frame_enc())
    }
}
