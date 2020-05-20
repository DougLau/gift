// decode.rs
//
// Copyright (c) 2019-2020  Douglas Lau
//
//! GIF file decoding
use crate::block::*;
use crate::error::Error;
use pix::rgb::SRgba8;
use pix::{Raster, Region};
use std::io::{ErrorKind, Read};

/// Buffer size (must be at least as large as a color table with 256 entries)
const BUF_SZ: usize = 1024;

/// An Iterator for [Block]s within a GIF file.
///
/// Build with Decoder.[into_blocks].
///
/// ## Example: Read comments in a GIF
/// ```
/// # use crate::gift::block::Block;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let gif = &[
/// #   0x47, 0x49, 0x46, 0x38, 0x39, 0x61, 0x02, 0x00,
/// #   0x02, 0x00, 0x80, 0x01, 0x00, 0x00, 0x00, 0x00,
/// #   0xff, 0xff, 0xff, 0x2c, 0x00, 0x00, 0x00, 0x00,
/// #   0x02, 0x00, 0x02, 0x00, 0x00, 0x02, 0x03, 0x0c,
/// #   0x10, 0x05, 0x00, 0x3b,
/// # ][..];
/// // ... open a File as "gif"
/// for block in gift::Decoder::new(gif).into_blocks() {
///     if let Block::Comment(b) = block? {
///         for c in b.comments() {
///             println!("{}", &String::from_utf8_lossy(&c));
///         }
///     }
/// }
/// # Ok(())
/// # }
/// ```
///
/// [Block]: ../block/enum.Block.html
/// [into_blocks]: ../struct.Decoder.html#method.into_blocks
///
pub struct Blocks<R: Read> {
    /// Buffered reader
    reader: R,
    /// Maximum image size in bytes
    max_image_sz: Option<usize>,
    /// Data buffer
    buffer: Vec<u8>,
    /// Expected next block
    expected_next: Option<(BlockCode, usize)>,
    /// Size of image data
    image_sz: usize,
    /// LZW decoder
    decoder: Option<lzw::Decoder<lzw::LsbReader>>,
    /// Flag when done
    done: bool,
}

impl<R: Read> Iterator for Blocks<R> {
    type Item = Result<Block, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            self.done = false;
            None
        } else {
            let res = self.next_block();
            match res {
                Ok(Block::Trailer(_)) | Err(_) => self.done = true,
                _ => (),
            }
            Some(res)
        }
    }
}

impl<R: Read> Blocks<R> {
    /// Create a new block iterator
    pub(crate) fn new(reader: R, max_image_sz: Option<usize>) -> Self {
        use self::BlockCode::Header_;
        Blocks {
            reader,
            max_image_sz,
            buffer: Vec::with_capacity(BUF_SZ),
            expected_next: Some((Header_, Header_.size())),
            image_sz: 0,
            done: false,
            decoder: None,
        }
    }

    /// Examine buffer for block code and size.
    fn examine_buffer(&mut self) -> Result<(BlockCode, usize), Error> {
        let code = *self
            .buffer
            .iter()
            .next()
            .ok_or(Error::UnexpectedEndOfFile)?;
        let bc_sz = self.expected_next.take().or_else(|| {
            match BlockCode::from_u8(code) {
                Some(b) => Some((b, b.size())),
                None => None,
            }
        });
        match bc_sz {
            Some(b) => {
                self.expected_next = self.expected(b.0);
                Ok(b)
            }
            None => Err(Error::InvalidBlockCode),
        }
    }

    /// Get next expected block code and size
    fn expected(&self, bc: BlockCode) -> Option<(BlockCode, usize)> {
        use crate::block::BlockCode::*;
        let buf = &self.buffer[..];
        match bc {
            Header_ => {
                let sz = LogicalScreenDesc_.size();
                Some((LogicalScreenDesc_, sz))
            }
            LogicalScreenDesc_ => {
                let sz = LogicalScreenDesc_.size();
                if buf.len() >= sz {
                    let buf = &buf[..sz];
                    if let Ok(b) = LogicalScreenDesc::from_buf(buf) {
                        let sz = b.color_table_config().size_bytes();
                        if sz > 0 {
                            return Some((GlobalColorTable_, sz));
                        }
                    }
                }
                None
            }
            ImageDesc_ => {
                let sz = ImageDesc_.size();
                if buf.len() >= sz {
                    let buf = &buf[..sz];
                    if let Ok(b) = ImageDesc::from_buf(buf) {
                        let sz = b.color_table_config().size_bytes();
                        if sz > 0 {
                            return Some((LocalColorTable_, sz));
                        } else {
                            return Some((ImageData_, ImageData_.size()));
                        }
                    }
                }
                None
            }
            LocalColorTable_ => Some((ImageData_, ImageData_.size())),
            Trailer_ => Some((Header_, Header_.size())),
            _ => None,
        }
    }

    /// Decode the next block (including all sub-blocks).
    fn next_block(&mut self) -> Result<Block, Error> {
        self.fill_buffer()?;
        let (bc, sz) = self.examine_buffer()?;
        let mut block = self.decode_block(bc, sz)?;
        if block.has_sub_blocks() {
            while self.decode_sub_block(&mut block)? {}
        }
        self.check_block_end(&block)?;
        Ok(block)
    }

    /// Check end of block (after sub-blocks)
    fn check_block_end(&mut self, block: &Block) -> Result<(), Error> {
        if let Block::ImageData(b) = block {
            self.decoder = None;
            if !b.is_complete() {
                return Err(Error::IncompleteImageData);
            }
        }
        Ok(())
    }

    /// Fill the buffer from reader
    fn fill_buffer(&mut self) -> Result<(), Error> {
        let mut len = self.buffer.len();
        self.buffer.resize(BUF_SZ, 0);
        while len < BUF_SZ {
            match self.reader.read(&mut self.buffer[len..]) {
                Ok(0) => break, // EOF
                Ok(n) => len += n,
                Err(ref e) if e.kind() == ErrorKind::Interrupted => {}
                Err(e) => return Err(e.into()),
            }
        }
        self.buffer.resize(len, 0);
        Ok(())
    }

    /// Decode one block
    fn decode_block(
        &mut self,
        bc: BlockCode,
        sz: usize,
    ) -> Result<Block, Error> {
        let len = self.buffer.len();
        if len >= sz {
            debug!("  block  : {:?} {:?}", bc, sz);
            let block = self.parse_block(bc, sz)?;
            self.buffer.drain(..sz);
            self.check_block_start(&block)?;
            Ok(block)
        } else {
            Err(Error::UnexpectedEndOfFile)
        }
    }

    /// Parse a block in the buffer
    fn parse_block(&self, bc: BlockCode, sz: usize) -> Result<Block, Error> {
        use crate::block::BlockCode::*;
        let buf = &self.buffer[..sz];
        Ok(match bc {
            Header_ => Header::from_buf(buf)?.into(),
            LogicalScreenDesc_ => LogicalScreenDesc::from_buf(buf)?.into(),
            GlobalColorTable_ => GlobalColorTable::from_buf(buf).into(),
            Extension_ => Block::parse_extension(buf),
            ImageDesc_ => ImageDesc::from_buf(buf)?.into(),
            LocalColorTable_ => LocalColorTable::from_buf(buf).into(),
            ImageData_ => ImageData::from_buf(self.image_sz, buf)?.into(),
            Trailer_ => Trailer::default().into(),
        })
    }

    /// Check start of block (before sub-blocks)
    fn check_block_start(&mut self, block: &Block) -> Result<(), Error> {
        match block {
            Block::ImageDesc(b) => {
                self.image_sz = b.image_sz();
                if let Some(sz) = self.max_image_sz {
                    if self.image_sz > sz {
                        return Err(Error::TooLargeImage);
                    }
                }
            }
            Block::ImageData(b) => {
                self.decoder = Some(lzw::Decoder::new(
                    lzw::LsbReader::new(),
                    b.min_code_size(),
                ));
            }
            _ => {}
        }
        Ok(())
    }

    /// Decode one sub-block
    fn decode_sub_block(&mut self, block: &mut Block) -> Result<bool, Error> {
        self.fill_buffer()?;
        let len = self.buffer.len();
        if len > 0 {
            let sz = self.buffer[0] as usize;
            if len > sz {
                let bsz = sz + 1;
                if sz > 0 {
                    debug!("sub-block: {:?} {:?}", block, sz);
                    self.parse_sub_block(block, bsz)?;
                }
                self.buffer.drain(..bsz);
                return Ok(sz > 0);
            }
        }
        Err(Error::UnexpectedEndOfFile)
    }

    /// Parse a sub-block in the buffer
    fn parse_sub_block(
        &mut self,
        block: &mut Block,
        sz: usize,
    ) -> Result<(), Error> {
        assert!(sz <= 256);
        use crate::block::Block::*;
        match block {
            PlainText(b) => b.parse_buf(&self.buffer[1..sz]),
            GraphicControl(b) => b.parse_buf(&self.buffer[1..sz])?,
            Comment(b) => b.parse_buf(&self.buffer[1..sz]),
            Application(b) => b.parse_buf(&self.buffer[1..sz]),
            Unknown(b) => b.parse_buf(&self.buffer[1..sz]),
            ImageData(b) => self.decode_image_data(b, sz)?,
            _ => panic!("Invalid state in parse_sub_block!"),
        }
        Ok(())
    }

    /// Decode image data
    fn decode_image_data(
        &mut self,
        b: &mut ImageData,
        sz: usize,
    ) -> Result<(), Error> {
        if let Some(ref mut dec) = &mut self.decoder {
            let mut s = 1;
            while s < sz {
                let buf = &self.buffer[s..sz];
                let (consumed, data) = dec.decode_bytes(buf)?;
                b.parse_buf(data);
                s += consumed;
            }
            return Ok(());
        }
        panic!("Invalid state in decode_image_data!");
    }
}

impl Header {
    /// Decode a Header block from a buffer
    fn from_buf(buf: &[u8]) -> Result<Self, Error> {
        assert_eq!(buf.len(), BlockCode::Header_.size());
        if &buf[..3] == b"GIF" {
            let version = [buf[3], buf[4], buf[5]];
            match &version {
                b"87a" | b"89a" => Ok(Header::with_version(version)),
                _ => Err(Error::UnsupportedVersion(version)),
            }
        } else {
            Err(Error::MalformedHeader)
        }
    }
}

impl LogicalScreenDesc {
    /// Decode a Logical Screen Descriptor block from a buffer
    fn from_buf(buf: &[u8]) -> Result<Self, Error> {
        assert_eq!(buf.len(), BlockCode::LogicalScreenDesc_.size());
        let width = u16::from(buf[1]) << 8 | u16::from(buf[0]);
        let height = u16::from(buf[3]) << 8 | u16::from(buf[2]);
        let flags = buf[4];
        let bg_color = buf[5];
        let aspect = buf[6];
        Ok(LogicalScreenDesc::default()
            .with_screen_width(width)
            .with_screen_height(height)
            .with_flags(flags)
            .with_background_color_idx(bg_color)
            .with_pixel_aspect_ratio(aspect))
    }
}

impl GlobalColorTable {
    /// Decode a Global Color Table block from a buffer
    fn from_buf(buf: &[u8]) -> Self {
        Self::with_colors(buf)
    }
}

impl ImageDesc {
    /// Decode an Image Descriptor block from a buffer
    fn from_buf(buf: &[u8]) -> Result<Self, Error> {
        assert_eq!(buf.len(), BlockCode::ImageDesc_.size());
        let left = u16::from(buf[2]) << 8 | u16::from(buf[1]);
        let top = u16::from(buf[4]) << 8 | u16::from(buf[3]);
        let width = u16::from(buf[6]) << 8 | u16::from(buf[5]);
        let height = u16::from(buf[8]) << 8 | u16::from(buf[7]);
        let flags = buf[9];
        Ok(Self::default()
            .with_left(left)
            .with_top(top)
            .with_width(width)
            .with_height(height)
            .with_flags(flags))
    }
}

impl LocalColorTable {
    /// Decode a Local Color Table block from a buffer
    fn from_buf(buf: &[u8]) -> Self {
        Self::with_colors(buf)
    }
}

impl ImageData {
    /// Decode an Image Data block from a buffer
    fn from_buf(image_sz: usize, buf: &[u8]) -> Result<Self, Error> {
        assert_eq!(buf.len(), BlockCode::ImageData_.size());
        let min_code_size = buf[0];
        let mut selfy = Self::new(image_sz);
        selfy.set_min_code_size(min_code_size);
        // check if min_code_size was valid
        if selfy.min_code_size() == min_code_size {
            Ok(selfy)
        } else {
            Err(Error::InvalidCodeSize)
        }
    }
}

impl Block {
    /// Parse an extension block
    fn parse_extension(buf: &[u8]) -> Self {
        use crate::block::ExtensionCode::*;
        assert_eq!(buf.len(), BlockCode::Extension_.size());
        let et: ExtensionCode = buf[1].into();
        match et {
            PlainText_ => PlainText::default().into(),
            GraphicControl_ => GraphicControl::default().into(),
            Comment_ => Comment::default().into(),
            Application_ => Application::default().into(),
            Unknown_(n) => Unknown::new(n).into(),
        }
    }
}

impl PlainText {
    /// Parse a Plain Text extension block
    fn parse_buf(&mut self, buf: &[u8]) {
        self.add_sub_block(buf);
    }
}

impl GraphicControl {
    /// Parse a Graphic Control extension block
    fn parse_buf(&mut self, buf: &[u8]) -> Result<(), Error> {
        if buf.len() == 4 {
            self.set_flags(buf[0]);
            let delay = u16::from(buf[2]) << 8 | u16::from(buf[1]);
            self.set_delay_time_cs(delay);
            self.set_transparent_color_idx(buf[3]);
            Ok(())
        } else {
            Err(Error::MalformedGraphicControlExtension)
        }
    }
}

impl Comment {
    /// Parse a Comment extension block
    fn parse_buf(&mut self, buf: &[u8]) {
        self.add_comment(buf);
    }
}

impl Application {
    /// Parse an Application extension block
    fn parse_buf(&mut self, buf: &[u8]) {
        self.add_app_data(buf);
    }
}

impl Unknown {
    /// Create a new Unknown extension block
    fn new(ext_id: u8) -> Self {
        let mut b = Unknown::default();
        b.add_sub_block(&[ext_id]);
        b
    }

    /// Parse an Unknown extension block
    fn parse_buf(&mut self, buf: &[u8]) {
        self.add_sub_block(buf);
    }
}

impl ImageData {
    /// Parse an Image Data block
    fn parse_buf(&mut self, buf: &[u8]) {
        self.add_data(buf);
    }
}

/// An Iterator for [Frame]s within a GIF file.
///
/// Build with Decoder.[into_frames].
///
/// ## Example: Count frames in a GIF
/// ```
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let gif = &[
/// #   0x47, 0x49, 0x46, 0x38, 0x39, 0x61, 0x02, 0x00,
/// #   0x02, 0x00, 0x80, 0x01, 0x00, 0x00, 0x00, 0x00,
/// #   0xff, 0xff, 0xff, 0x2c, 0x00, 0x00, 0x00, 0x00,
/// #   0x02, 0x00, 0x02, 0x00, 0x00, 0x02, 0x03, 0x0c,
/// #   0x10, 0x05, 0x00, 0x3b,
/// # ][..];
/// // ... open a File as "gif"
/// let frames = gift::Decoder::new(gif).into_frames();
/// println!("frame count: {}", frames.count());
/// # Ok(())
/// # }
/// ```
///
/// [Frame]: ../block/struct.Frame.html
/// [into_frames]: ../struct.Decoder.html#method.into_frames
///
pub struct Frames<R: Read> {
    /// Block decoder
    blocks: Blocks<R>,
    /// Preamble blocks
    preamble: Option<Preamble>,
    /// Graphic control block
    graphic_control_ext: Option<GraphicControl>,
    /// Image description block
    image_desc: Option<ImageDesc>,
    /// Local color table block
    local_color_table: Option<LocalColorTable>,
}

impl<R: Read> Iterator for Frames<R> {
    type Item = Result<Frame, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(block) = self.blocks.next() {
            match block {
                Ok(b) => {
                    match self.handle_block(b) {
                        Ok(Some(f)) => return Some(Ok(f)), // transpose
                        Ok(None) => {}                     // need more blocks
                        Err(e) => return Some(Err(e)),
                    }
                }
                Err(e) => return Some(Err(e)),
            }
        }
        None
    }
}

impl<R: Read> Frames<R> {
    /// Create a new frame decoder
    pub(crate) fn new(blocks: Blocks<R>) -> Self {
        Frames {
            blocks,
            preamble: None,
            graphic_control_ext: None,
            image_desc: None,
            local_color_table: None,
        }
    }

    /// Read preamble blocks.  These are the blocks at the beginning of the
    /// file, before any frame blocks.
    pub fn preamble(&mut self) -> Result<Option<Preamble>, Error> {
        if self.has_frame() {
            return Ok(None);
        }
        self.preamble = Some(Preamble::default());
        while let Some(block) = self.blocks.next() {
            self.handle_block(block?)?;
            if self.has_frame() {
                return Ok(self.preamble.take());
            }
        }
        Err(Error::InvalidBlockSequence)
    }

    /// Check if any frame blocks exist
    fn has_frame(&self) -> bool {
        self.graphic_control_ext.is_some()
            || self.image_desc.is_some()
            || self.local_color_table.is_some()
    }

    /// Handle one block
    fn handle_block(&mut self, block: Block) -> Result<Option<Frame>, Error> {
        match block {
            Block::Header(b) => {
                if let Some(ref mut f) = &mut self.preamble {
                    f.header = b;
                }
            }
            Block::LogicalScreenDesc(b) => {
                if let Some(ref mut f) = &mut self.preamble {
                    f.logical_screen_desc = b;
                }
            }
            Block::GlobalColorTable(b) => {
                if let Some(ref mut f) = &mut self.preamble {
                    f.global_color_table = Some(b);
                }
            }
            Block::Application(b) => {
                if let (Some(ref mut f), Some(_)) =
                    (&mut self.preamble, b.loop_count())
                {
                    f.loop_count_ext = Some(b);
                }
            }
            Block::Comment(b) => {
                if let Some(ref mut f) = &mut self.preamble {
                    f.comments.push(b);
                }
            }
            Block::GraphicControl(b) => {
                if self.has_frame() {
                    return Err(Error::InvalidBlockSequence);
                }
                self.graphic_control_ext = Some(b);
            }
            Block::ImageDesc(b) => {
                if self.image_desc.is_some() {
                    return Err(Error::InvalidBlockSequence);
                }
                self.image_desc = Some(b);
            }
            Block::LocalColorTable(b) => {
                self.local_color_table = Some(b);
            }
            Block::ImageData(image_data) => {
                let graphic_control_ext = self.graphic_control_ext.take();
                let image_desc = self.image_desc.take();
                let local_color_table = self.local_color_table.take();
                if let Some(image_desc) = image_desc {
                    let f = Frame::new(
                        graphic_control_ext,
                        image_desc,
                        local_color_table,
                        image_data,
                    );
                    return Ok(Some(f));
                } else {
                    return Err(Error::InvalidBlockSequence);
                }
            }
            _ => {}
        }
        Ok(None)
    }
}

/// An Iterator for `Raster`s within a GIF file.
///
/// Build with Decoder.[into_iter] (or [into_rasters]).
///
/// ## Example: Get the last raster in a GIF animation
/// ```
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let gif = &[
/// #   0x47, 0x49, 0x46, 0x38, 0x39, 0x61, 0x02, 0x00,
/// #   0x02, 0x00, 0x80, 0x01, 0x00, 0x00, 0x00, 0x00,
/// #   0xff, 0xff, 0xff, 0x2c, 0x00, 0x00, 0x00, 0x00,
/// #   0x02, 0x00, 0x02, 0x00, 0x00, 0x02, 0x03, 0x0c,
/// #   0x10, 0x05, 0x00, 0x3b,
/// # ][..];
/// // ... open a File as "gif"
/// if let Some(raster) = gift::Decoder::new(gif).into_iter().last() {
///     // was there a decoding error?
///     let raster = raster?;
///     // ... work with raster
/// }
/// # Ok(())
/// # }
/// ```
///
/// [into_iter]: ../struct.Decoder.html#method.into_iter
/// [into_rasters]: ../struct.Decoder.html#method.into_rasters
///
pub struct Rasters<R: Read> {
    /// Frame decoder
    frames: Frames<R>,
    /// Global color table block
    global_color_table: Option<GlobalColorTable>,
    /// Current raster
    raster: Option<Raster<SRgba8>>, // TODO: parameterize pix trait
}

impl<R: Read> Iterator for Rasters<R> {
    // TODO: need delay time (and color table for indexed rasters)
    type Item = Result<Raster<SRgba8>, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.raster.is_none() {
            if let Err(e) = self.make_raster() {
                return Some(Err(e));
            }
        }
        match self.raster {
            Some(_) => self.next_raster(),
            None => None,
        }
    }
}

impl<R: Read> Rasters<R> {
    /// Create a new raster decoder
    pub(crate) fn new(frames: Frames<R>) -> Self {
        Rasters {
            frames,
            global_color_table: None,
            raster: None,
        }
    }

    /// Make the initial raster
    fn make_raster(&mut self) -> Result<(), Error> {
        if let Some(mut p) = self.frames.preamble()? {
            self.global_color_table = p.global_color_table.take();
            let w = p.screen_width().into();
            let h = p.screen_height().into();
            self.raster = Some(Raster::with_clear(w, h));
            Ok(())
        } else {
            warn!("Preamble not found!");
            Ok(())
        }
    }

    /// Get the next raster
    fn next_raster(&mut self) -> Option<Result<Raster<SRgba8>, Error>> {
        assert!(self.raster.is_some());
        match self.frames.next() {
            Some(Ok(f)) => Some(self.apply_frame(f)),
            Some(Err(e)) => Some(Err(e)),
            None => None,
        }
    }

    /// Apply a frame to the raster
    fn apply_frame(&mut self, frame: Frame) -> Result<Raster<SRgba8>, Error> {
        let raster = if let DisposalMethod::Previous = frame.disposal_method() {
            let raster = self.raster.as_ref().unwrap();
            let mut raster = Raster::with_raster(raster);
            update_raster(&mut raster, &frame, &self.global_color_table)?;
            raster
        } else {
            let mut raster = self.raster.as_mut().unwrap();
            update_raster(&mut raster, &frame, &self.global_color_table)?;
            Raster::with_raster(raster)
        };
        if let DisposalMethod::Background = frame.disposal_method() {
            let x = frame.left().into();
            let y = frame.top().into();
            let w = frame.width().into();
            let h = frame.height().into();
            let reg = Region::new(x, y, w, h);
            let rs = self.raster.as_mut().unwrap();
            rs.copy_color(reg, SRgba8::default());
        }
        Ok(raster)
    }
}

/// Update a raster with a new frame
fn update_raster(
    raster: &mut Raster<SRgba8>,
    frame: &Frame,
    global_tbl: &Option<GlobalColorTable>,
) -> Result<(), Error> {
    let x = u32::from(frame.left());
    let y = u32::from(frame.top());
    let width = u32::from(frame.width());
    let height = u32::from(frame.height());
    let rwidth = raster.width();
    let rheight = raster.height();
    if x + width <= rwidth && y + height <= rheight {
        let clrs = if let Some(tbl) = &frame.local_color_table {
            tbl.colors()
        } else if let Some(tbl) = global_tbl {
            tbl.colors()
        } else {
            return Err(Error::MissingColorTable);
        };
        let trans_clr = frame.transparent_color();
        let reg = (frame.left().into(), frame.top().into(), width, height);
        let width = width as usize;
        let height = height as usize;
        let data = frame.image_data.data();
        for (row, frow) in raster.rows_mut(reg).zip(data.chunks_exact(width)) {
            for (p, fp) in row.iter_mut().zip(frow) {
                let idx = *fp;
                let i = 3 * idx as usize;
                if i + 2 > clrs.len() {
                    return Err(Error::InvalidColorIndex);
                }
                let entry = match trans_clr {
                    Some(trans_idx) if trans_idx == idx => SRgba8::default(),
                    _ => SRgba8::new(clrs[i], clrs[i + 1], clrs[i + 2], 255),
                };
                *p = entry;
            }
        }
        Ok(())
    } else {
        Err(Error::InvalidFrameDimensions)
    }
}

#[cfg(test)]
mod test {
    use super::super::Decoder;
    use std::error::Error;

    #[rustfmt::skip]
    const GIF_1: &[u8] = &[
        0x47, 0x49, 0x46, 0x38, 0x39, 0x61, 0x0A, 0x00, 0x0A, 0x00, 0x91, 0x00,
        0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00, 0xFF, 0x00, 0x00,
        0x00, 0x21, 0xF9, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x2C, 0x00, 0x00,
        0x00, 0x00, 0x0A, 0x00, 0x0A, 0x00, 0x00, 0x02, 0x16, 0x8C, 0x2D, 0x99,
        0x87, 0x2A, 0x1C, 0xDC, 0x33, 0xA0, 0x02, 0x75, 0xEC, 0x95, 0xFA, 0xA8,
        0xDE, 0x60, 0x8C, 0x04, 0x91, 0x4C, 0x01, 0x00, 0x3B,
    ];

    #[rustfmt::skip]
    const IMAGE_1: &[u8] = &[
        1, 1, 1, 1, 1, 2, 2, 2, 2, 2,
        1, 1, 1, 1, 1, 2, 2, 2, 2, 2,
        1, 1, 1, 1, 1, 2, 2, 2, 2, 2,
        1, 1, 1, 0, 0, 0, 0, 2, 2, 2,
        1, 1, 1, 0, 0, 0, 0, 2, 2, 2,
        2, 2, 2, 0, 0, 0, 0, 1, 1, 1,
        2, 2, 2, 0, 0, 0, 0, 1, 1, 1,
        2, 2, 2, 2, 2, 1, 1, 1, 1, 1,
        2, 2, 2, 2, 2, 1, 1, 1, 1, 1,
        2, 2, 2, 2, 2, 1, 1, 1, 1, 1,
    ];

    #[test]
    fn block_1() -> Result<(), Box<dyn Error>> {
        use crate::block::*;
        #[rustfmt::skip]
        let colors = &[
            0xFF, 0xFF, 0xFF,
            0xFF, 0x00, 0x00,
            0x00, 0x00, 0xFF,
            0x00, 0x00, 0x00,
        ][..];
        let mut dec = Decoder::new(GIF_1).into_blocks();
        match dec.next() {
            Some(Ok(Block::Header(b))) => assert_eq!(b, Header::default()),
            _ => panic!(),
        }
        match dec.next() {
            Some(Ok(Block::LogicalScreenDesc(b))) => assert_eq!(
                b,
                LogicalScreenDesc::default()
                    .with_screen_width(10)
                    .with_screen_height(10)
                    .with_flags(0x91)
            ),
            _ => panic!(),
        }
        match dec.next() {
            Some(Ok(Block::GlobalColorTable(b))) => {
                assert_eq!(b, GlobalColorTable::with_colors(colors))
            }
            _ => panic!(),
        }
        match dec.next() {
            Some(Ok(Block::GraphicControl(b))) => {
                assert_eq!(b, GraphicControl::default())
            }
            _ => panic!(),
        }
        match dec.next() {
            Some(Ok(Block::ImageDesc(b))) => assert_eq!(
                b,
                ImageDesc::default().with_width(10).with_height(10)
            ),
            _ => panic!(),
        }
        match dec.next() {
            Some(Ok(Block::ImageData(b))) => {
                let mut d = ImageData::new(100);
                d.add_data(IMAGE_1);
                assert_eq!(b, d);
            }
            _ => panic!(),
        }
        match dec.next() {
            Some(Ok(Block::Trailer(b))) => assert_eq!(b, Trailer::default()),
            _ => panic!(),
        }
        Ok(())
    }

    #[test]
    fn frame_1() -> Result<(), Box<dyn Error>> {
        for f in Decoder::new(GIF_1).into_frames() {
            assert_eq!(f?.image_data.data(), IMAGE_1);
        }
        Ok(())
    }

    #[test]
    fn image_1() -> Result<(), Box<dyn Error>> {
        use pix::rgb::SRgba8;
        let red = SRgba8::new(0xFF, 0x00, 0x00, 0xFF);
        let blu = SRgba8::new(0x00, 0x00, 0xFF, 0xFF);
        let wht = SRgba8::new(0xFF, 0xFF, 0xFF, 0xFF);
        #[rustfmt::skip]
        let image = &[
            red, red, red, red, red, blu, blu, blu, blu, blu,
            red, red, red, red, red, blu, blu, blu, blu, blu,
            red, red, red, red, red, blu, blu, blu, blu, blu,
            red, red, red, wht, wht, wht, wht, blu, blu, blu,
            red, red, red, wht, wht, wht, wht, blu, blu, blu,
            blu, blu, blu, wht, wht, wht, wht, red, red, red,
            blu, blu, blu, wht, wht, wht, wht, red, red, red,
            blu, blu, blu, blu, blu, red, red, red, red, red,
            blu, blu, blu, blu, blu, red, red, red, red, red,
            blu, blu, blu, blu, blu, red, red, red, red, red,
        ][..];
        for r in Decoder::new(GIF_1) {
            assert_eq!(r?.pixels(), image);
        }
        Ok(())
    }

    const HEADER: &[u8] = &[0x47, 0x49, 0x46, 0x38, 0x39, 0x60];

    #[test]
    fn iterator() {
        use crate::error::Error;
        let mut dec = Decoder::new(HEADER).into_blocks();
        match dec.next().unwrap() {
            Err(Error::UnsupportedVersion(_)) => (),
            _ => panic!(),
        }
        match dec.next() {
            None => (),
            _ => panic!(),
        }
    }

    #[test]
    fn empty() {
        use crate::error::Error;
        let mut dec = Decoder::new(std::io::Cursor::new(b"")).into_frames();
        match dec.next().unwrap() {
            Err(Error::UnexpectedEndOfFile) => (),
            _ => panic!(),
        }
        match dec.next() {
            None => (),
            _ => panic!(),
        }
    }
}
