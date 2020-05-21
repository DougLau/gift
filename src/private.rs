// private.rs
//
// Copyright (c) 2019-2020  Douglas Lau
//
//! Private module for top-level items
use crate::{decode, encode, Error};
use pix::rgb::SRgba8;
use pix::Raster;
use std::io::{BufReader, BufWriter, Read, Write};

/// GIF file decoder
///
/// Can be converted to one of three `Iterator`s:
/// * [into_iter] / [into_rasters] for high-level `Raster`s
/// * [into_frames] for mid-level [Frame]s
/// * [into_blocks] for low-level [Block]s
///
/// ## Example: Get a `Raster` from a GIF
/// ```
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let gif = &[
/// #   0x47, 0x49, 0x46, 0x38, 0x39, 0x61, 0x02, 0x00,
/// #   0x02, 0x00, 0x80, 0x01, 0x00, 0x00, 0x00, 0x00,
/// #   0xff, 0xff, 0xff, 0x2c, 0x00, 0x00, 0x00, 0x00,
/// #   0x02, 0x00, 0x02, 0x00, 0x00, 0x02, 0x03, 0x0c,
/// #   0x10, 0x05, 0x00, 0x3b,
/// # ][..];
/// // ... open a `File` as "gif"
/// if let Some(raster) = gift::Decoder::new(gif).into_iter().next() {
///     // was there a decoding error?
///     let raster = raster?;
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
/// [into_rasters]: struct.Decoder.html#method.into_rasters
///
pub struct Decoder<R: Read> {
    /// Reader for input data
    reader: R,
    /// Maximum image size, in bytes
    max_image_sz: Option<usize>,
}

impl<R: Read> Decoder<BufReader<R>> {
    /// Create a new buffered GIF decoder.
    pub fn new(reader: R) -> Self {
        Self::new_unbuffered(BufReader::new(reader))
    }
}

impl<R: Read> Decoder<R> {
    /// Create a new unbuffered GIF decoder.
    pub fn new_unbuffered(reader: R) -> Self {
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

    /// Convert into a raster `Iterator`.
    pub fn into_rasters(self) -> decode::Rasters<R> {
        decode::Rasters::new(self.into_frames())
    }
}

impl<R: Read> IntoIterator for Decoder<R> {
    type Item = Result<Raster<SRgba8>, Error>;
    type IntoIter = decode::Rasters<R>;

    /// Convert into a raster `Iterator`
    fn into_iter(self) -> Self::IntoIter {
        self.into_rasters()
    }
}

/// GIF file encoder
///
/// Can be converted to one of three encoders:
/// * [into_raster_enc] for high-level `Raster`s
/// * [into_frame_enc] for mid-level [Frame]s
/// * [into_block_enc] for low-level [Block]s
///
/// ## Encoding Example
/// ```
/// use gift::Encoder;
/// use pix::{gray::Gray8, Palette, Raster, rgb::SRgb8};
/// use std::error::Error;
/// use std::io::Write;
///
/// fn encode<W: Write>(mut w: W) -> Result<(), Box<dyn Error>> {
///     let mut enc = Encoder::new(&mut w).into_raster_enc();
///     let mut raster = Raster::with_clear(4, 4);
///     *raster.pixel_mut(0, 0) = Gray8::new(1);
///     *raster.pixel_mut(1, 1) = Gray8::new(1);
///     *raster.pixel_mut(2, 2) = Gray8::new(1);
///     *raster.pixel_mut(3, 3) = Gray8::new(1);
///     let mut palette = Palette::new(2);
///     palette.set_entry(SRgb8::new(0xFF, 0, 0));
///     palette.set_entry(SRgb8::new(0xFF, 0xFF, 0));
///     enc.encode_indexed_raster(&raster, palette)?;
///     Ok(())
/// }
/// ```
///
/// [Block]: block/enum.Block.html
/// [Frame]: block/struct.Frame.html
/// [into_block_enc]: struct.Encoder.html#method.into_block_enc
/// [into_frame_enc]: struct.Encoder.html#method.into_frame_enc
/// [into_raster_enc]: struct.Encoder.html#method.into_raster_enc
pub struct Encoder<W: Write> {
    /// Writer for output data
    writer: W,
}

impl<W: Write> Encoder<BufWriter<W>> {
    /// Create a new GIF encoder.
    pub fn new(writer: W) -> Self {
        Self::new_unbuffered(BufWriter::new(writer))
    }
}

impl<W: Write> Encoder<W> {
    /// Create a new unbuffered GIF encoder.
    pub fn new_unbuffered(writer: W) -> Self {
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

    /// Convert into a raster encoder.
    pub fn into_raster_enc(self) -> encode::RasterEnc<W> {
        encode::RasterEnc::new(self.into_frame_enc())
    }
}
