// encode.rs
//
// Copyright (c) 2019  Douglas Lau
//

use std::io::{self, BufWriter, Write};
use crate::block::*;

/// Encoder for GIF files.
pub struct Encoder<W: Write> {
    writer: BufWriter<W>,
}

impl<W: Write> Encoder<W> {
    /// Create a new GIF encoder.
    pub fn new(w: W) -> Self {
        Encoder {
            writer: BufWriter::new(w),
        }
    }
    /// Encode one [Block](block/enum.Block.html).
    pub fn encode(&mut self, block: &Block) -> io::Result<()> {
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
        }
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
