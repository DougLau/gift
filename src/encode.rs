// encode.rs
//
// Copyright (c) 2019  Douglas Lau
//
use crate::EncodeError;
use crate::block::*;
use std::io::{self, BufWriter, Write};

/// Encoder for GIF files.
pub struct Encoder<W: Write> {
    // FIXME: this should be a builder for BlockEncoder / FrameEncoder
    //        Also add builder option for global vs. local color tables.
    //        Also builder option for color table creation mode.  Use Lab or Lch
    //        with octree clustering and dithering.  Check out exoquant.
    writer: BufWriter<W>,
}

/// Encoder for writing [Frame](block/struct.Frame.html)s into a GIF file.
pub struct FrameEncoder<W: Write> {
    encoder: Encoder<W>,
    has_preamble: bool,
    has_trailer: bool,
}

impl<W: Write> Encoder<W> {
    /// Create a new GIF encoder.
    pub fn new(w: W) -> Self {
        Encoder {
            writer: BufWriter::new(w),
        }
    }
    /// Encode one [Block](block/enum.Block.html).
    pub fn encode(&mut self, block: &Block) -> Result<(), EncodeError> {
        use crate::block::Block::*;
        let mut w = &mut self.writer;
        match block {
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
    /// Convert into a frame encoder.
    pub fn into_frame_encoder(self) -> FrameEncoder<W> {
        FrameEncoder::new(self)
    }
}

impl Header {
    /// Format a header block
    fn format<W: Write>(&self, w: &mut BufWriter<W>) -> io::Result<()> {
        w.write_all(b"GIF")?;
        w.write_all(&self.version())
    }
}

impl LogicalScreenDesc {
    /// Format a logical screen desc block
    fn format<W: Write>(&self, w: &mut BufWriter<W>) -> io::Result<()> {
        let mut buf = Vec::with_capacity(7);
        let width = self.screen_width();
        buf.push((width >> 0) as u8);
        buf.push((width >> 8) as u8);
        let height = self.screen_height();
        buf.push((height >> 0) as u8);
        buf.push((height >> 8) as u8);
        buf.push(self.flags());
        buf.push(self.background_color_idx());
        buf.push(self.pixel_aspect_ratio());
        w.write_all(&buf)
    }
}

impl GlobalColorTable {
    /// Format a global color table block
    fn format<W: Write>(&self, w: &mut BufWriter<W>) -> io::Result<()> {
        w.write_all(self.colors())
    }
}

impl PlainText {
    /// Format a plain text extension block
    fn format<W: Write>(&self, w: &mut BufWriter<W>) -> io::Result<()> {
        w.write_all(BlockCode::Extension_.signature())?;
        w.write_all(&[ExtensionCode::PlainText_.into()])?;
        for b in self.sub_blocks() {
            assert!(b.len() < 256);
            let len = b.len() as u8;
            w.write_all(&[len])?;   // block size
            w.write_all(b)?;
        }
        w.write_all(&[0])   // block size
    }
}

impl GraphicControl {
    /// Format a graphic control extension block
    fn format<W: Write>(&self, w: &mut BufWriter<W>) -> io::Result<()> {
        w.write_all(BlockCode::Extension_.signature())?;
        let mut buf = Vec::with_capacity(7);
        buf.push(ExtensionCode::GraphicControl_.into());
        buf.push(4);    // block size
        buf.push(self.flags());
        let delay = self.delay_time_cs();
        buf.push((delay >> 0) as u8);
        buf.push((delay >> 8) as u8);
        buf.push(self.transparent_color_idx());
        buf.push(0);    // block size
        w.write_all(&buf)
    }
}

impl Comment {
    /// Format a comment extension block
    fn format<W: Write>(&self, w: &mut BufWriter<W>) -> io::Result<()> {
        w.write_all(BlockCode::Extension_.signature())?;
        w.write_all(&[ExtensionCode::Comment_.into()])?;
        for c in self.comments() {
            assert!(c.len() < 256);
            let len = c.len() as u8;
            w.write_all(&[len])?;   // block size
            w.write_all(c)?;
        }
        w.write_all(&[0])   // block size
    }
}

impl Application {
    /// Format an application extension block
    fn format<W: Write>(&self, w: &mut BufWriter<W>) -> io::Result<()> {
        w.write_all(BlockCode::Extension_.signature())?;
        w.write_all(&[ExtensionCode::Application_.into()])?;
        for c in self.app_data() {
            assert!(c.len() < 256);
            let len = c.len() as u8;
            w.write_all(&[len])?;   // block size
            w.write_all(c)?;
        }
        w.write_all(&[0])   // block size
    }
}

impl Unknown {
    /// Format an unknown extension block
    fn format<W: Write>(&self, w: &mut BufWriter<W>) -> io::Result<()> {
        w.write_all(BlockCode::Extension_.signature())?;
        w.write_all(self.ext_id())?;
        for c in self.sub_blocks() {
            assert!(c.len() < 256);
            let len = c.len() as u8;
            w.write_all(&[len])?;   // block size
            w.write_all(c)?;
        }
        w.write_all(&[0])   // block size
    }
}

impl ImageDesc {
    /// Format an image desc block
    fn format<W: Write>(&self, w: &mut BufWriter<W>) -> io::Result<()> {
        w.write_all(BlockCode::ImageDesc_.signature())?;
        let mut buf = Vec::with_capacity(9);
        let left = self.left();
        buf.push((left >> 0) as u8);
        buf.push((left >> 8) as u8);
        let top = self.top();
        buf.push((top >> 0) as u8);
        buf.push((top >> 8) as u8);
        let width = self.width();
        buf.push((width >> 0) as u8);
        buf.push((width >> 8) as u8);
        let height = self.height();
        buf.push((height >> 0) as u8);
        buf.push((height >> 8) as u8);
        buf.push(self.flags());
        w.write_all(&buf)
    }
}

impl LocalColorTable {
    /// Format a local color table block
    fn format<W: Write>(&self, w: &mut BufWriter<W>) -> io::Result<()> {
        w.write_all(self.colors())
    }
}

impl ImageData {
    /// Format an image data block
    fn format<W: Write>(&self, w: &mut BufWriter<W>) -> io::Result<()> {
        w.write_all(&[self.min_code_size()])?;
        self.format_block(w)?;
        w.write_all(&[0])
    }
    /// Format the entire "block" (including sub-blocks)
    fn format_block<W: Write>(&self, mut w: &mut BufWriter<W>)
        -> io::Result<()>
    {
        let mut bw = BlockWriter::new(&mut w);
        self.format_data(&mut bw)?;
        bw.flush()
    }
    /// Format image data (with LZW encoding)
    fn format_data<W: Write>(&self, mut bw: &mut BlockWriter<W>)
        -> io::Result<()>
    {
        let mut enc = lzw::Encoder::new(lzw::LsbWriter::new(&mut bw),
            self.min_code_size())?;
        enc.encode_bytes(self.data())
    }
}

/// Block / sub-block writer
struct BlockWriter<'a, W: Write> {
    writer: &'a mut BufWriter<W>,
    buf: Vec<u8>,
}

impl<'a, W: Write> BlockWriter<'a, W> {
    /// Create a new block writer
    fn new(writer: &'a mut BufWriter<W>) -> Self {
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
            // the wrapped BufWriter.  Since we're adding the 0xFF length
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
    fn format<W: Write>(&self, w: &mut BufWriter<W>) -> io::Result<()> {
        w.write_all(BlockCode::Trailer_.signature())
    }
}

impl<W: Write> FrameEncoder<W> {
    /// Create a new GIF frame encoder.
    fn new(encoder: Encoder<W>) -> Self {
        let has_preamble = false;
        let has_trailer = false;
        FrameEncoder { encoder, has_preamble, has_trailer }
    }
    /// Encode the GIF preamble blocks.
    ///
    /// Must be called only once, before
    /// [encode_frame](struct.FrameEncoder.html#method.encode_frame).
    pub fn encode_preamble(&mut self, preamble: &Preamble)
        -> Result<(), EncodeError>
    {
        if self.has_preamble {
            return Err(EncodeError::InvalidBlockSequence);
        }
        self.encoder.encode(&preamble.header.clone().into())?;
        self.encoder.encode(&preamble.logical_screen_desc.clone().into())?;
        if let Some(tbl) = &preamble.global_color_table {
            self.encoder.encode(&tbl.clone().into())?;
        }
        if let Some(cnt) = &preamble.loop_count_ext {
            self.encoder.encode(&cnt.clone().into())?;
        }
        for comment in &preamble.comments {
            self.encoder.encode(&comment.clone().into())?;
        }
        self.has_preamble = true;
        Ok(())
    }
    /// Encode one `Frame` of a GIF file.
    ///
    /// Must be called after
    /// [encode_preamble](struct.FrameEncoder.html#method.encode_preamble).
    pub fn encode_frame(&mut self, frame: &Frame) -> Result<(), EncodeError> {
        if self.has_trailer || !self.has_preamble {
            return Err(EncodeError::InvalidBlockSequence);
        }
        if let Some(ctrl) = &frame.graphic_control_ext {
            self.encoder.encode(&ctrl.clone().into())?;
        }
        self.encoder.encode(&frame.image_desc.clone().into())?;
        if let Some(tbl) = &frame.local_color_table {
            self.encoder.encode(&tbl.clone().into())?;
        }
        self.encoder.encode(&frame.image_data.clone().into())?;
        Ok(())
    }
    /// Encode the [Trailer](block/struct.Trailer.html) of a GIF file.
    ///
    /// Must be called last, after all `Frame`s have been encoded with
    /// [encode_frame](struct.FrameEncoder.html#method.encode_frame).
    pub fn encode_trailer(&mut self) -> Result<(), EncodeError> {
        if self.has_trailer || !self.has_preamble {
            return Err(EncodeError::InvalidBlockSequence);
        }
        self.encoder.encode(&Trailer::default().into())?;
        self.has_trailer = true;
        Ok(())
    }
}
