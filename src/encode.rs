// encode.rs
//
// Copyright (c) 2019-2023  Douglas Lau
//
//! GIF file encoding
use crate::block::*;
use crate::lzw::Compressor;
use crate::private::StepRaster;
use crate::{Error, Result, Step};
use pix::{gray::Gray8, rgb::Rgb, Palette, Raster};
use std::convert::TryInto;
use std::io::{self, Write};

/// Encoder for writing [Block]s into a GIF file.
///
/// Build with Encoder.[into_block_enc].
///
/// [Block]: ../block/enum.Block.html
/// [into_block_enc]: ../struct.Encoder.html#method.into_block_enc
pub struct BlockEnc<W: Write> {
    /// Writer for output data
    writer: W,
}

impl<W: Write> BlockEnc<W> {
    /// Create a new GIF encoder.
    pub(crate) fn new(writer: W) -> Self {
        BlockEnc { writer }
    }

    /// Encode one [Block](block/enum.Block.html).
    pub fn encode<B>(&mut self, block: B) -> Result<()>
    where
        B: Into<Block>,
    {
        use crate::block::Block::*;
        let mut w = &mut self.writer;
        match block.into() {
            Header(b) => b.format(&mut w),
            LogicalScreenDesc(b) => b.format(&mut w),
            GlobalColorTable(b) => b.format(&mut w),
            PlainText(b) => b.format(&mut w),
            GraphicControl(b) => b.format(&mut w),
            Comment(b) => b.format(&mut w),
            Application(b) => b.format(&mut w),
            Unknown(b) => b.format(&mut w),
            ImageDesc(b) => b.format(&mut w),
            LocalColorTable(b) => b.format(&mut w),
            ImageData(b) => b.format(&mut w),
            Trailer(b) => b.format(&mut w),
        }?;
        Ok(())
    }
}

/// Encoder for writing [Frame]s into a GIF file.
///
/// Build with Encoder.[into_frame_enc].
///
/// [Frame]: ../block/struct.Frame.html
/// [into_frame_enc]: ../struct.Encoder.html#method.into_frame_enc
pub struct FrameEnc<W: Write> {
    /// Block encoder
    block_enc: BlockEnc<W>,
    /// Has preamble been encoded?
    has_preamble: bool,
    /// Has trailer been encoded?
    has_trailer: bool,
}

impl Header {
    /// Format a header block
    fn format<W: Write>(self, w: &mut W) -> io::Result<()> {
        w.write_all(b"GIF")?;
        w.write_all(&self.version())
    }
}

impl LogicalScreenDesc {
    /// Format a logical screen desc block
    fn format<W: Write>(self, w: &mut W) -> io::Result<()> {
        let width = self.screen_width();
        let height = self.screen_height();
        w.write_all(&[
            width as u8,
            (width >> 8) as u8,
            height as u8,
            (height >> 8) as u8,
            self.flags(),
            self.background_color_idx(),
            self.pixel_aspect_ratio(),
        ])
    }
}

impl GlobalColorTable {
    /// Format a global color table block
    fn format<W: Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_all(self.colors())
    }
}

impl PlainText {
    /// Format a plain text extension block
    fn format<W: Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_all(BlockCode::Extension_.signature())?;
        w.write_all(&[ExtensionCode::PlainText_.into()])?;
        for b in self.sub_blocks() {
            debug_assert!(!b.is_empty() && b.len() < 256);
            let len = b.len() as u8;
            w.write_all(&[len])?; // sub-block size
            w.write_all(b)?;
        }
        w.write_all(&[0]) // final sub-block size
    }
}

impl GraphicControl {
    /// Format a graphic control extension block
    fn format<W: Write>(self, w: &mut W) -> io::Result<()> {
        w.write_all(BlockCode::Extension_.signature())?;
        let delay = self.delay_time_cs();
        w.write_all(&[
            ExtensionCode::GraphicControl_.into(),
            4, // block size
            self.flags(),
            delay as u8,
            (delay >> 8) as u8,
            self.transparent_color_idx(),
            0, // block size
        ])
    }
}

impl Comment {
    /// Format a comment extension block
    fn format<W: Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_all(BlockCode::Extension_.signature())?;
        w.write_all(&[ExtensionCode::Comment_.into()])?;
        for c in self.comments() {
            debug_assert!(!c.is_empty() && c.len() < 256);
            let len = c.len() as u8;
            w.write_all(&[len])?; // sub-block size
            w.write_all(c)?;
        }
        w.write_all(&[0]) // final sub-block size
    }
}

impl Application {
    /// Format an application extension block
    fn format<W: Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_all(BlockCode::Extension_.signature())?;
        w.write_all(&[ExtensionCode::Application_.into()])?;
        for c in self.app_data() {
            debug_assert!(!c.is_empty() && c.len() < 256);
            let len = c.len() as u8;
            w.write_all(&[len])?; // sub-block size
            w.write_all(c)?;
        }
        w.write_all(&[0]) // final sub-block size
    }
}

impl Unknown {
    /// Format an unknown extension block
    fn format<W: Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_all(BlockCode::Extension_.signature())?;
        w.write_all(self.ext_id())?;
        for c in self.sub_blocks() {
            debug_assert!(!c.is_empty() && c.len() < 256);
            let len = c.len() as u8;
            w.write_all(&[len])?; // sub-block size
            w.write_all(c)?;
        }
        w.write_all(&[0]) // final sub-block size
    }
}

impl ImageDesc {
    /// Format an image desc block
    fn format<W: Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_all(BlockCode::ImageDesc_.signature())?;
        let left = self.left();
        let top = self.top();
        let width = self.width();
        let height = self.height();
        w.write_all(&[
            left as u8,
            (left >> 8) as u8,
            top as u8,
            (top >> 8) as u8,
            width as u8,
            (width >> 8) as u8,
            height as u8,
            (height >> 8) as u8,
            self.flags(),
        ])
    }
}

impl LocalColorTable {
    /// Format a local color table block
    fn format<W: Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_all(self.colors())
    }
}

impl ImageData {
    /// Format an image data block
    fn format<W: Write>(&self, w: &mut W) -> io::Result<()> {
        let min_code_bits =
            next_high_bit(self.data().iter().copied().max().unwrap_or(0));
        // minimum code bits must be between 2 and 8
        let min_code_bits = 2.max(min_code_bits).min(8);
        w.write_all(&[min_code_bits])?;
        let mut buffer = Vec::with_capacity(self.data().len());
        let mut compressor = Compressor::new(min_code_bits);
        compressor.compress(self.data(), &mut buffer);
        // split buffer into sub-blocks
        for chunk in buffer.chunks(255) {
            let len = chunk.len() as u8;
            w.write_all(&[len])?; // sub-block size
            w.write_all(chunk)?;
        }
        w.write_all(&[0]) // final sub-block size
    }
}

/// Get the high bit of a value
fn next_high_bit(value: u8) -> u8 {
    u32::from(value).next_power_of_two().trailing_zeros() as u8
}

impl Trailer {
    /// Format a trailer block
    fn format<W: Write>(self, w: &mut W) -> io::Result<()> {
        w.write_all(BlockCode::Trailer_.signature())
    }
}

impl<W: Write> FrameEnc<W> {
    /// Create a new GIF frame encoder.
    pub(crate) fn new(block_enc: BlockEnc<W>) -> Self {
        FrameEnc {
            block_enc,
            has_preamble: false,
            has_trailer: false,
        }
    }

    /// Encode the GIF preamble blocks.
    ///
    /// Must be called only once, before [encode_frame].
    ///
    /// [encode_frame]: struct.FrameEnc.html#method.encode_frame
    pub fn encode_preamble(&mut self, preamble: &Preamble) -> Result<()> {
        if self.has_preamble {
            return Err(Error::InvalidBlockSequence);
        }
        self.block_enc.encode(preamble.header)?;
        self.block_enc.encode(preamble.logical_screen_desc)?;
        if let Some(tbl) = &preamble.global_color_table {
            self.block_enc.encode(tbl.clone())?;
        }
        if let Some(cnt) = &preamble.loop_count_ext {
            self.block_enc.encode(cnt.clone())?;
        }
        for comment in &preamble.comments {
            self.block_enc.encode(comment.clone())?;
        }
        self.has_preamble = true;
        Ok(())
    }

    /// Encode one `Frame` of a GIF file.
    ///
    /// Must be called after [encode_preamble].
    ///
    /// [encode_preamble]: struct.FrameEnc.html#method.encode_preamble
    pub fn encode_frame(&mut self, frame: &Frame) -> Result<()> {
        if self.has_trailer || !self.has_preamble {
            return Err(Error::InvalidBlockSequence);
        }
        if let Some(ctrl) = &frame.graphic_control_ext {
            self.block_enc.encode(*ctrl)?;
        }
        self.block_enc.encode(frame.image_desc)?;
        if let Some(tbl) = &frame.local_color_table {
            self.block_enc.encode(tbl.clone())?;
        }
        self.block_enc.encode(frame.image_data.clone())?;
        Ok(())
    }

    /// Encode the [Trailer] of a GIF file.
    ///
    /// Must be called last, after all `Frame`s have been encoded with
    /// [encode_frame].
    ///
    /// [encode_frame]: struct.FrameEnc.html#method.encode_frame
    /// [Trailer]: block/struct.Trailer.html
    pub fn encode_trailer(&mut self) -> Result<()> {
        if self.has_trailer || !self.has_preamble {
            return Err(Error::InvalidBlockSequence);
        }
        self.block_enc.encode(Trailer::default())?;
        self.has_trailer = true;
        Ok(())
    }
}

/// Encoder for writing [Step]s into a GIF file.
///
/// All `Raster`s must have the same dimensions.
///
/// [Step]: ../struct.Step.html
pub struct StepEnc<W: Write> {
    /// Frame encoder
    frame_enc: FrameEnc<W>,
    /// Global color table
    global_color_table: (ColorTableConfig, Option<GlobalColorTable>),
    /// Animation loop count
    loop_count: Option<Application>,
    /// Preamble blocks
    preamble: Option<Preamble>,
}

impl<W: Write> Drop for StepEnc<W> {
    fn drop(&mut self) {
        let _ = self.frame_enc.encode_trailer();
    }
}

impl<W: Write> StepEnc<W> {
    /// Create a new GIF raster encoder.
    pub(crate) fn new(frame_enc: FrameEnc<W>) -> Self {
        StepEnc {
            frame_enc,
            global_color_table: (ColorTableConfig::default(), None),
            loop_count: None,
            preamble: None,
        }
    }

    /// Set loop count for the animation.
    ///
    /// * `loop_count`: Number of times to loop animation; zero means forever)
    pub fn with_loop_count(mut self, loop_count: u16) -> Self {
        self.loop_count = Some(Application::with_loop_count(loop_count));
        self
    }

    /// Set the global color table for an animation.
    pub fn with_global_color_table(mut self, palette: &Palette) -> Self {
        let (tbl_cfg, pal) = make_color_table(palette);
        self.global_color_table =
            (tbl_cfg, Some(GlobalColorTable::with_colors(&pal[..])));
        self
    }

    /// Encode an indexed `Raster` to a GIF file.
    fn encode_indexed_raster(
        &mut self,
        raster: &Raster<Gray8>,
        palette: &Palette,
        control: Option<GraphicControl>,
    ) -> Result<()> {
        let image_desc = make_image_desc(raster)?;
        let image_data = raster.into();
        let (tbl_cfg, pal) = make_color_table(palette);
        let logical_screen_desc = LogicalScreenDesc::default()
            .with_screen_width(image_desc.width())
            .with_screen_height(image_desc.height())
            .with_color_table_config(tbl_cfg);
        let global_color_table = Some(GlobalColorTable::with_colors(&pal[..]));
        let loop_count_ext = self.loop_count.clone();
        let preamble = Preamble {
            logical_screen_desc,
            global_color_table,
            loop_count_ext,
            ..Preamble::default()
        };
        match &self.preamble {
            Some(pre) => {
                if !pre
                    .logical_screen_desc
                    .equal_size(preamble.logical_screen_desc)
                {
                    return Err(Error::InvalidRasterDimensions);
                }
                if pre.global_color_table != preamble.global_color_table {
                    let frame = Frame::new(
                        control,
                        image_desc.with_color_table_config(tbl_cfg),
                        Some(LocalColorTable::with_colors(&pal[..])),
                        image_data,
                    );
                    return self.frame_enc.encode_frame(&frame);
                }
            }
            None => {
                self.frame_enc.encode_preamble(&preamble)?;
                self.preamble = Some(preamble);
            }
        }
        let frame = Frame::new(control, image_desc, None, image_data);
        self.frame_enc.encode_frame(&frame)
    }

    /// Encode one [Step] to a GIF file.
    ///
    /// [Step]: ../struct.Step.html
    pub fn encode_step(&mut self, step: &Step) -> Result<()> {
        match &step.raster {
            StepRaster::TrueColor(_) => {
                todo!("convert raster to indexed raster");
            }
            StepRaster::Indexed(raster, palette) => {
                self.encode_indexed_raster(
                    raster,
                    palette,
                    step.graphic_control_ext,
                )?;
            }
        }
        Ok(())
    }
}

/// Make an image description block
fn make_image_desc(raster: &Raster<Gray8>) -> Result<ImageDesc> {
    let width = raster.width().try_into()?;
    let height = raster.height().try_into()?;
    Ok(ImageDesc::default().with_width(width).with_height(height))
}

/// Make a color table from a palette
fn make_color_table(palette: &Palette) -> (ColorTableConfig, Vec<u8>) {
    let tbl_cfg = ColorTableConfig::new(
        ColorTableExistence::Present,
        ColorTableOrdering::NotSorted,
        palette.len() as u16,
    );
    let mut pal = Vec::with_capacity(palette.len() * 3);
    for clr in palette.colors() {
        pal.push(u8::from(Rgb::red(*clr)));
        pal.push(u8::from(Rgb::green(*clr)));
        pal.push(u8::from(Rgb::blue(*clr)));
    }
    while pal.len() < tbl_cfg.size_bytes() {
        pal.push(0);
    }
    (tbl_cfg, pal)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::Encoder;
    use pix::{gray::Gray8, rgb::SRgb8, Palette, Raster};

    #[test]
    fn high_bits() {
        assert_eq!(next_high_bit(0), 0);
        assert_eq!(next_high_bit(2), 1);
        assert_eq!(next_high_bit(3), 2);
        assert_eq!(next_high_bit(4), 2);
        assert_eq!(next_high_bit(5), 3);
        assert_eq!(next_high_bit(7), 3);
        assert_eq!(next_high_bit(8), 3);
        assert_eq!(next_high_bit(9), 4);
        assert_eq!(next_high_bit(16), 4);
    }

    /// Check a raster encode
    fn check_encode(palette: Palette, raster: Raster<Gray8>, data: &[u8]) {
        let mut bytes = vec![];
        let mut enc = Encoder::new(&mut bytes).into_step_enc();
        let step = Step::with_indexed(raster, palette);
        enc.encode_step(&step).unwrap();
        drop(enc);
        assert_eq!(&bytes[..], data);
    }

    /// Encoded 2x2 gif data
    const GIF_2X2: &[u8] = &[
        71, 73, 70, 56, 57, 97, 2, 0, 2, 0, 128, 0, 0, 0, 255, 0, 0, 255, 255,
        44, 0, 0, 0, 0, 2, 0, 2, 0, 0, 2, 2, 12, 16, 0, 59,
    ];

    #[test]
    fn enc_2x2() {
        let mut raster = Raster::with_clear(2, 2);
        *raster.pixel_mut(0, 0) = Gray8::new(1);
        *raster.pixel_mut(1, 1) = Gray8::new(1);
        let mut palette = Palette::new(2);
        palette.set_entry(SRgb8::new(0, 0xFF, 0));
        palette.set_entry(SRgb8::new(0, 0xFF, 0xFF));
        check_encode(palette, raster, GIF_2X2);
    }

    /// Encoded 3x3 gif data
    const GIF_3X3: &[u8] = &[
        71, 73, 70, 56, 57, 97, 3, 0, 3, 0, 162, 0, 0, 255, 0, 0, 0, 255, 0, 0,
        0, 255, 255, 255, 0, 255, 0, 255, 0, 0, 0, 0, 0, 0, 0, 0, 0, 44, 0, 0,
        0, 0, 3, 0, 3, 0, 0, 3, 5, 24, 176, 2, 4, 35, 0, 59,
    ];

    #[test]
    fn enc_3x3() {
        let mut raster = Raster::with_clear(3, 3);
        *raster.pixel_mut(0, 0) = Gray8::new(1);
        *raster.pixel_mut(1, 1) = Gray8::new(2);
        *raster.pixel_mut(2, 2) = Gray8::new(3);
        *raster.pixel_mut(0, 2) = Gray8::new(4);
        let mut palette = Palette::new(5);
        palette.set_entry(SRgb8::new(0xFF, 0, 0));
        palette.set_entry(SRgb8::new(0, 0xFF, 0));
        palette.set_entry(SRgb8::new(0, 0, 0xFF));
        palette.set_entry(SRgb8::new(0xFF, 0xFF, 0));
        palette.set_entry(SRgb8::new(0xFF, 0, 0xFF));
        check_encode(palette, raster, GIF_3X3);
    }

    /// Encoded 4x4 gif data
    const GIF_4X4: &[u8] = &[
        71, 73, 70, 56, 57, 97, 4, 0, 4, 0, 128, 0, 0, 255, 0, 0, 255, 255, 0,
        44, 0, 0, 0, 0, 4, 0, 4, 0, 0, 2, 5, 12, 14, 134, 122, 81, 0, 59,
    ];

    #[test]
    fn enc_4x4() {
        let mut raster = Raster::with_clear(4, 4);
        *raster.pixel_mut(0, 0) = Gray8::new(1);
        *raster.pixel_mut(1, 1) = Gray8::new(1);
        *raster.pixel_mut(2, 2) = Gray8::new(1);
        *raster.pixel_mut(3, 3) = Gray8::new(1);
        let mut palette = Palette::new(2);
        palette.set_entry(SRgb8::new(0xFF, 0, 0));
        palette.set_entry(SRgb8::new(0xFF, 0xFF, 0));
        check_encode(palette, raster, GIF_4X4);
    }
}
