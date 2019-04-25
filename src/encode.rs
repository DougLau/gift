// encode.rs
//
// Copyright (c) 2019  Douglas Lau
//

use std::io::{self, BufWriter, Write};
use crate::block::*;

pub struct Encoder<W: Write> {
    writer: BufWriter<W>,
}

impl<W: Write> Encoder<W> {
    pub fn new(w: W) -> Self {
        Encoder {
            writer: BufWriter::new(w),
        }
    }
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
    fn format<W: Write>(&self, w: &mut BufWriter<W>) -> io::Result<()> {
        w.write_all(b"GIF")?;
        w.write_all(&self.version())
    }
}

impl LogicalScreenDesc {
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
    fn format<W: Write>(&self, w: &mut BufWriter<W>) -> io::Result<()> {
        w.write_all(self.colors())
    }
}

impl PlainText {
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
    fn format<W: Write>(&self, w: &mut BufWriter<W>) -> io::Result<()> {
        w.write_all(self.colors())
    }
}

impl ImageData {
    fn format<W: Write>(&self, w: &mut BufWriter<W>) -> io::Result<()> {
        w.write_all(&[self.min_code_size()])?;
        self.format_block(w)?;
        w.write_all(&[0])
    }
    fn format_block<W: Write>(&self, mut w: &mut BufWriter<W>)
        -> io::Result<()>
    {
        let mut bw = BlockWriter::new(&mut w);
        self.format_data(&mut bw)?;
        bw.flush()
    }
    fn format_data<W: Write>(&self, mut bw: &mut BlockWriter<W>)
        -> io::Result<()>
    {
        let mut enc = lzw::Encoder::new(lzw::LsbWriter::new(&mut bw),
            self.min_code_size())?;
        enc.encode_bytes(self.data())
    }
}

struct BlockWriter<'a, W: Write> {
    writer: &'a mut BufWriter<W>,
    buf: Vec<u8>,
}

impl<'a, W: Write> BlockWriter<'a, W> {
    fn new(writer: &'a mut BufWriter<W>) -> Self {
        let buf = Vec::with_capacity(256);
        BlockWriter { writer, buf }
    }
}

impl<'a, W: Write> Write for BlockWriter<'a, W> {
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
    fn format<W: Write>(&self, w: &mut BufWriter<W>) -> io::Result<()> {
        w.write_all(BlockCode::Trailer_.signature())
    }
}
