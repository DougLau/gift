// encode.rs
//
// Copyright (c) 2019-2020  Douglas Lau
//
//! GIF file encoding
use crate::{block::*, private::StepRaster, Error, Result, Step};
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
            assert!(b.len() < 256);
            let len = b.len() as u8;
            w.write_all(&[len])?; // block size
            w.write_all(b)?;
        }
        w.write_all(&[0]) // block size
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
            assert!(c.len() < 256);
            let len = c.len() as u8;
            w.write_all(&[len])?; // block size
            w.write_all(c)?;
        }
        w.write_all(&[0]) // block size
    }
}

impl Application {
    /// Format an application extension block
    fn format<W: Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_all(BlockCode::Extension_.signature())?;
        w.write_all(&[ExtensionCode::Application_.into()])?;
        for c in self.app_data() {
            assert!(c.len() < 256);
            let len = c.len() as u8;
            w.write_all(&[len])?; // block size
            w.write_all(c)?;
        }
        w.write_all(&[0]) // block size
    }
}

impl Unknown {
    /// Format an unknown extension block
    fn format<W: Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_all(BlockCode::Extension_.signature())?;
        w.write_all(self.ext_id())?;
        for c in self.sub_blocks() {
            assert!(c.len() < 256);
            let len = c.len() as u8;
            w.write_all(&[len])?; // block size
            w.write_all(c)?;
        }
        w.write_all(&[0]) // block size
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
        w.write_all(&[self.min_code_size()])?;
        self.format_block(w)?;
        w.write_all(&[0])
    }

    /// Format the entire "block" (including sub-blocks)
    fn format_block<W: Write>(&self, mut w: &mut W) -> io::Result<()> {
        let mut bw = BlockWriter::new(&mut w);
        self.format_data(&mut bw)?;
        bw.flush()
    }

    /// Format image data (with LZW encoding)
    fn format_data<W: Write>(
        &self,
        mut bw: &mut BlockWriter<W>,
    ) -> io::Result<()> {
        let mut enc = lzw::Encoder::new(
            lzw::LsbWriter::new(&mut bw),
            self.min_code_size(),
        )?;
        enc.encode_bytes(self.data())
    }
}

/// Block / sub-block writer
struct BlockWriter<'a, W: Write> {
    /// Writer for blocks
    writer: &'a mut W,
    /// Block buffer
    buf: Vec<u8>,
}

impl<'a, W: Write> BlockWriter<'a, W> {
    /// Create a new block writer
    fn new(writer: &'a mut W) -> Self {
        let buf = Vec::with_capacity(256);
        BlockWriter { writer, buf }
    }
}

impl<'a, W: Write> Write for BlockWriter<'a, W> {
    /// Write a buffer
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let remaining = 0xFF - self.buf.len();
        let consumed = remaining.min(buf.len());
        self.buf.extend_from_slice(&buf[..consumed]);
        if self.buf.len() == 0xFF {
            // Technically, we're only supposed to make one attempt to write to
            // the wrapped writer.  Since we're adding the 0xFF length
            // at the beginning, we can't allow writes to be split up.
            self.writer.write_all(&[0xFF])?;
            self.writer.write_all(&self.buf)?;
            self.buf.clear();
        }
        Ok(consumed)
    }

    /// Flush data remaining in the buffer
    fn flush(&mut self) -> io::Result<()> {
        let len = self.buf.len();
        if len > 0 {
            self.writer.write_all(&[len as u8])?;
            self.writer.write_all(&self.buf[..len])?;
            self.buf.clear();
        }
        Ok(())
    }
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
        self.block_enc.encode(preamble.header.clone())?;
        self.block_enc
            .encode(preamble.logical_screen_desc.clone())?;
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
            self.block_enc.encode(ctrl.clone())?;
        }
        self.block_enc.encode(frame.image_desc.clone())?;
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
        let image_data = make_image_data(raster);
        let (tbl_cfg, pal) = make_color_table(palette);
        let mut preamble = Preamble::default();
        preamble.logical_screen_desc = LogicalScreenDesc::default()
            .with_screen_width(image_desc.width())
            .with_screen_height(image_desc.height())
            .with_color_table_config(tbl_cfg);
        preamble.global_color_table =
            Some(GlobalColorTable::with_colors(&pal[..]));
        preamble.loop_count_ext = self.loop_count.clone();
        match &self.preamble {
            Some(pre) => {
                if pre.logical_screen_desc != preamble.logical_screen_desc {
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

/// Make an image data block
fn make_image_data(raster: &Raster<Gray8>) -> ImageData {
    let mut image_data = ImageData::new(raster.pixels().len());
    image_data.add_data(raster.as_u8_slice());
    image_data
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
