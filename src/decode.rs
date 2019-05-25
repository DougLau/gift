// decode.rs
//
// Copyright (c) 2019  Douglas Lau
//

use std::io::{ErrorKind, BufReader, Read};
use lzw;
use pix::{Raster, RasterBuilder, Region, Rgba8};
use crate::error::DecodeError;
use crate::block::*;

/// Buffer size (must be at least as large as a color table with 256 entries)
const BUF_SZ: usize = 1024;

/// Builder for GIF decoders.
///
/// * [BlockDecoder](struct.BlockDecoder.html) — low-level,
///   [Block](block/enum.Block.html)s
/// * [FrameDecoder](struct.FrameDecoder.html) — mid-level,
///   [Frame](block/struct.Frame.html)s
/// * [RasterDecoder](struct.RasterDecoder.html) — high-level, Rasters
pub struct Decoder<R: Read> {
    reader: BufReader<R>,
    max_image_sz: Option<usize>,
}

impl<R: Read> Decoder<R> {
    /// Create a new Decoder
    pub fn new(r: R) -> Self {
        Decoder {
            reader: BufReader::new(r),
            max_image_sz: Some(1 << 25),
        }
    }
    /// Set the maximum image size (in bytes) to allow for decoding.
    pub fn max_image_sz(mut self, max_image_sz: Option<usize>) -> Self {
        self.max_image_sz = max_image_sz;
        self
    }
    /// Convert into a block decoder.
    pub fn into_block_decoder(self) -> BlockDecoder<R> {
        BlockDecoder::new(self.reader, self.max_image_sz)
    }
    /// Convert into a frame decoder.
    pub fn into_frame_decoder(self) -> FrameDecoder<R> {
        FrameDecoder::new(self.into_block_decoder())
    }
    /// Convert into a raster decoder.
    pub fn into_raster_decoder(self) -> RasterDecoder<R> {
        RasterDecoder::new(self.into_frame_decoder())
    }
}

impl<R: Read> IntoIterator for Decoder<R> {
    type Item = Result<Raster<Rgba8>, DecodeError>;
    type IntoIter = RasterDecoder<R>;

    /// Convert into a raster decoder
    fn into_iter(self) -> Self::IntoIter {
        self.into_raster_decoder()
    }
}

/// Decoder for iterating [Block](block/enum.Block.html)s within a GIF file.
///
/// Build with
/// Decoder.[into_block_iter](struct.Decoder.html#method.into_block_iter).
///
/// ## Example: Read comments in a GIF
/// ```
/// # use crate::gift::block::Block;
/// # fn main() -> Result<(), Box<std::error::Error>> {
/// # let gif = &[
/// #   0x47, 0x49, 0x46, 0x38, 0x39, 0x61, 0x02, 0x00,
/// #   0x02, 0x00, 0x80, 0x01, 0x00, 0x00, 0x00, 0x00,
/// #   0xff, 0xff, 0xff, 0x2c, 0x00, 0x00, 0x00, 0x00,
/// #   0x02, 0x00, 0x02, 0x00, 0x00, 0x02, 0x03, 0x0c,
/// #   0x10, 0x05, 0x00, 0x3b,
/// # ][..];
/// // ... open a File as "gif"
/// let block_dec = gift::Decoder::new(gif).into_block_decoder();
/// for block in block_dec {
///     if let Block::Comment(b) = block? {
///         for c in b.comments() {
///             println!("{}", &String::from_utf8_lossy(&c));
///         }
///     }
/// }
/// # Ok(())
/// # }
/// ```
pub struct BlockDecoder<R: Read> {
    reader: BufReader<R>,
    max_image_sz: Option<usize>,
    buffer: Vec<u8>,
    expected_next: Option<(BlockCode, usize)>,
    image_sz: usize,
    decoder: Option<lzw::Decoder<lzw::LsbReader>>,
    done: bool,
}

impl<R: Read> Iterator for BlockDecoder<R> {
    type Item = Result<Block, DecodeError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            self.done = false;
            None
        } else {
            let res = self.next_block();
            if let Ok(Block::Trailer(_)) = res {
                self.done = true;
            }
            Some(res)
        }
    }
}

impl<R: Read> BlockDecoder<R> {
    /// Create a new block decoder
    fn new(reader: BufReader<R>, max_image_sz: Option<usize>) -> Self {
        use self::BlockCode::Header_;
        BlockDecoder {
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
    fn examine_buffer(&mut self) -> Result<(BlockCode, usize), DecodeError> {
        let buf = &self.buffer[..];
        let t = if buf.len() > 0 { buf[0] } else { 0 };
        let bc_sz = self.expected_next.take().or_else(||
            match BlockCode::from_u8(t) {
                Some(b) => Some((b, b.size())),
                None => None,
            }
        );
        match bc_sz {
            Some(b) => {
                self.expected_next = self.expected(b.0);
                Ok(b)
            },
            None => Err(DecodeError::InvalidBlockCode),
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
            },
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
            },
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
            },
            LocalColorTable_ => Some((ImageData_, ImageData_.size())),
            Trailer_ => Some((Header_, Header_.size())),
            _ => None,
        }
    }
    /// Decode the next block (including all sub-blocks).
    fn next_block(&mut self) -> Result<Block, DecodeError> {
        self.fill_buffer()?;
        let (bc, sz) = self.examine_buffer()?;
        let mut block = self.decode_block(bc, sz)?;
        if block.has_sub_blocks() {
            while self.decode_sub_block(&mut block)? { }
        }
        self.check_block_end(&block)?;
        Ok(block)
    }
    /// Check end of block (after sub-blocks)
    fn check_block_end(&mut self, block: &Block) -> Result<(), DecodeError> {
        if let Block::ImageData(b) = block {
            self.decoder = None;
            if !b.is_complete() {
                return Err(DecodeError::IncompleteImageData)
            }
        }
        Ok(())
    }
    /// Fill the buffer from reader
    fn fill_buffer(&mut self) -> Result<(), DecodeError> {
        let mut len = self.buffer.len();
        self.buffer.resize(BUF_SZ, 0);
        while len < BUF_SZ {
            match self.reader.read(&mut self.buffer[len..]) {
                Ok(0) => break, // EOF
                Ok(n) => len += n,
                Err(ref e) if e.kind() == ErrorKind::Interrupted => {},
                Err(e) => return Err(e.into()),
            }
        }
        self.buffer.resize(len, 0);
        Ok(())
    }
    /// Decode one block
    fn decode_block(&mut self, bc: BlockCode, sz: usize)
        -> Result<Block, DecodeError>
    {
        let len = self.buffer.len();
        if len >= sz {
            debug!("  block  : {:?} {:?}", bc, sz);
            let block = self.parse_block(bc, sz)?;
            self.buffer.drain(..sz);
            self.check_block_start(&block)?;
            Ok(block)
        } else {
            Err(DecodeError::UnexpectedEndOfFile)
        }
    }
    /// Parse a block in the buffer
    fn parse_block(&self, bc: BlockCode, sz: usize)
        -> Result<Block, DecodeError>
    {
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
    fn check_block_start(&mut self, block: &Block) -> Result<(), DecodeError> {
        match block {
            Block::ImageDesc(b) => {
                self.image_sz = b.image_sz();
                if let Some(sz) = self.max_image_sz {
                    if self.image_sz > sz {
                        return Err(DecodeError::TooLargeImage);
                    }
                }
            },
            Block::ImageData(b) => {
                self.decoder = Some(lzw::Decoder::new(lzw::LsbReader::new(),
                    b.min_code_size()));
            },
            _ => {},
        }
        Ok(())
    }
    /// Decode one sub-block
    fn decode_sub_block(&mut self, block: &mut Block)
        -> Result<bool, DecodeError>
    {
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
        Err(DecodeError::UnexpectedEndOfFile)
    }
    /// Parse a sub-block in the buffer
    fn parse_sub_block(&mut self, block: &mut Block, sz: usize)
        -> Result<(), DecodeError>
    {
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
    fn decode_image_data(&mut self, b: &mut ImageData, sz: usize)
        -> Result<(), DecodeError>
    {
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
    fn from_buf(buf: &[u8]) -> Result<Self, DecodeError> {
        assert_eq!(buf.len(), BlockCode::Header_.size());
        if &buf[..3] == b"GIF" {
            let version = [buf[3], buf[4], buf[5]];
            match &version {
                b"87a" | b"89a" => {
                    Ok(Header::with_version(version))
                },
                _ => Err(DecodeError::UnsupportedVersion(version)),
            }
        } else {
            Err(DecodeError::MalformedHeader)
        }
    }
}

impl LogicalScreenDesc {
    /// Decode a Logical Screen Descriptor block from a buffer
    fn from_buf(buf: &[u8]) -> Result<Self, DecodeError> {
        assert_eq!(buf.len(), BlockCode::LogicalScreenDesc_.size());
        let width  = u16::from(buf[1]) << 8 | u16::from(buf[0]);
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
    fn from_buf(buf: &[u8]) -> Result<Self, DecodeError> {
        assert_eq!(buf.len(), BlockCode::ImageDesc_.size());
        let left   = u16::from(buf[2]) << 8 | u16::from(buf[1]);
        let top    = u16::from(buf[4]) << 8 | u16::from(buf[3]);
        let width  = u16::from(buf[6]) << 8 | u16::from(buf[5]);
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
    fn from_buf(image_sz: usize, buf: &[u8]) -> Result<Self, DecodeError> {
        assert_eq!(buf.len(), BlockCode::ImageData_.size());
        let min_code_size = buf[0];
        let mut selfy = Self::new(image_sz);
        selfy.set_min_code_size(min_code_size);
        // check if min_code_size was valid
        if selfy.min_code_size() == min_code_size {
            Ok(selfy)
        } else {
            Err(DecodeError::InvalidCodeSize)
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
    fn parse_buf(&mut self, buf: &[u8]) -> Result<(), DecodeError> {
        if buf.len() == 4 {
            self.set_flags(buf[0]);
            let delay = u16::from(buf[2]) << 8 | u16::from(buf[1]);
            self.set_delay_time_cs(delay);
            self.set_transparent_color_idx(buf[3]);
            Ok(())
        } else {
            Err(DecodeError::MalformedGraphicControlExtension)
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

/// Decoder for iterating [Frame](block/struct.Frame.html)s within a GIF file.
///
/// Build with
/// Decoder.[into_frame_decoder](struct.Decoder.html#method.into_frame_decoder).
///
/// ## Example: Count frames in a GIF
/// ```
/// # fn main() -> Result<(), Box<std::error::Error>> {
/// # let gif = &[
/// #   0x47, 0x49, 0x46, 0x38, 0x39, 0x61, 0x02, 0x00,
/// #   0x02, 0x00, 0x80, 0x01, 0x00, 0x00, 0x00, 0x00,
/// #   0xff, 0xff, 0xff, 0x2c, 0x00, 0x00, 0x00, 0x00,
/// #   0x02, 0x00, 0x02, 0x00, 0x00, 0x02, 0x03, 0x0c,
/// #   0x10, 0x05, 0x00, 0x3b,
/// # ][..];
/// // ... open a File as "gif"
/// let frame_dec = gift::Decoder::new(gif).into_frame_decoder();
/// println!("frame count: {}", frame_dec.count());
/// # Ok(())
/// # }
/// ```
pub struct FrameDecoder<R: Read> {
    block_dec: BlockDecoder<R>,
    preamble: Option<Preamble>,
    graphic_control_ext: Option<GraphicControl>,
    image_desc: Option<ImageDesc>,
    local_color_table: Option<LocalColorTable>,
}

impl<R: Read> Iterator for FrameDecoder<R> {
    type Item = Result<Frame, DecodeError>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(block) = self.block_dec.next() {
            match block {
                Ok(b) => {
                    match self.handle_block(b) {
                        Ok(Some(f)) => return Some(Ok(f)),  // transpose
                        Ok(None) => {}, // need more blocks
                        Err(e) => return Some(Err(e)),
                    }
                },
                Err(e) => return Some(Err(e)),
            }
        }
        None
    }
}

impl<R: Read> FrameDecoder<R> {
    /// Create a new frame decoder
    fn new(block_dec: BlockDecoder<R>) -> Self {
        FrameDecoder {
            block_dec,
            preamble: None,
            graphic_control_ext: None,
            image_desc: None,
            local_color_table: None,
        }
    }
    /// Read preamble blocks.  These are the blocks at the beginning of the
    /// file, before any frame blocks.
    pub fn preamble(&mut self) -> Result<Option<Preamble>, DecodeError> {
        if self.has_frame() {
            return Ok(None);
        }
        self.preamble = Some(Preamble::default());
        while let Some(block) = self.block_dec.next() {
            self.handle_block(block?)?;
            if self.has_frame() {
                return Ok(self.preamble.take());
            }
        }
        Err(DecodeError::InvalidBlockSequence)
    }
    /// Check if any frame blocks exist
    fn has_frame(&self) -> bool {
        self.graphic_control_ext.is_some() ||
        self.image_desc.is_some() ||
        self.local_color_table.is_some()
    }
    /// Handle one block
    fn handle_block(&mut self, block: Block)
        -> Result<Option<Frame>, DecodeError>
    {
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
            },
            Block::GlobalColorTable(b) => {
                if let Some(ref mut f) = &mut self.preamble {
                    f.global_color_table = Some(b);
                }
            },
            Block::Application(b) => {
                if let (Some(ref mut f), Some(_)) =
                    (&mut self.preamble, b.loop_count())
                {
                    f.loop_count_ext = Some(b);
                }
            },
            Block::Comment(b) => {
                if let Some(ref mut f) = &mut self.preamble {
                    f.comments.push(b);
                }
            },
            Block::GraphicControl(b) => {
                if self.has_frame() {
                    return Err(DecodeError::InvalidBlockSequence);
                }
                self.graphic_control_ext = Some(b);
            },
            Block::ImageDesc(b) => {
                if self.image_desc.is_some() {
                    return Err(DecodeError::InvalidBlockSequence);
                }
                self.image_desc = Some(b);
            },
            Block::LocalColorTable(b) => {
                self.local_color_table = Some(b);
            },
            Block::ImageData(image_data) => {
                let graphic_control_ext = self.graphic_control_ext.take();
                let image_desc = self.image_desc.take();
                let local_color_table = self.local_color_table.take();
                if let Some(image_desc) = image_desc {
                    let f = Frame::new(graphic_control_ext, image_desc,
                        local_color_table, image_data);
                    return Ok(Some(f));
                } else {
                    return Err(DecodeError::InvalidBlockSequence);
                }
            },
            _ => {},
        }
        Ok(None)
    }
}

/// Decoder for iterating `Raster`s within a GIF file.
///
/// Build with
/// Decoder.[into_raster_decoder](struct.Decoder.html#method.into_raster_decoder).
///
/// ## Example: Get the last raster in a GIF animation
/// ```
/// # fn main() -> Result<(), Box<std::error::Error>> {
/// # let gif = &[
/// #   0x47, 0x49, 0x46, 0x38, 0x39, 0x61, 0x02, 0x00,
/// #   0x02, 0x00, 0x80, 0x01, 0x00, 0x00, 0x00, 0x00,
/// #   0xff, 0xff, 0xff, 0x2c, 0x00, 0x00, 0x00, 0x00,
/// #   0x02, 0x00, 0x02, 0x00, 0x00, 0x02, 0x03, 0x0c,
/// #   0x10, 0x05, 0x00, 0x3b,
/// # ][..];
/// // ... open a File as "gif"
/// let raster_dec = gift::Decoder::new(gif).into_raster_decoder();
/// if let Some(raster) = raster_dec.last() {
///     // work with raster
/// }
/// # Ok(())
/// # }
/// ```
pub struct RasterDecoder<R: Read> {
    frame_dec: FrameDecoder<R>,
    global_color_table: Option<GlobalColorTable>,
    raster: Option<Raster<Rgba8>>, // FIXME: parameterize pix trait
}

impl<R: Read> Iterator for RasterDecoder<R> {
    // FIXME: need delay time (and color table for indexed rasters)
    type Item = Result<Raster<Rgba8>, DecodeError>;

    fn next(&mut self) -> Option<Self::Item> {
        if let None = self.raster {
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

impl<R: Read> RasterDecoder<R> {
    /// Create a new raster decoder
    fn new(frame_dec: FrameDecoder<R>) -> Self {
        RasterDecoder {
            frame_dec,
            global_color_table: None,
            raster: None,
        }
    }
    /// Make the initial raster
    fn make_raster(&mut self) -> Result<(), DecodeError> {
        if let Some(mut p) = self.frame_dec.preamble()? {
            self.global_color_table = p.global_color_table.take();
            let w = p.screen_width().into();
            let h = p.screen_height().into();
            self.raster = Some(RasterBuilder::<Rgba8>::new().with_clear(w, h));
            Ok(())
        } else {
            warn!("Preamble not found!");
            Ok(())
        }
    }
    /// Get the next raster
    fn next_raster(&mut self) -> Option<Result<Raster<Rgba8>, DecodeError>> {
        assert!(self.raster.is_some());
        match self.frame_dec.next() {
            Some(Ok(f)) => Some(self.apply_frame(f)),
            Some(Err(e)) => Some(Err(e)),
            None => None,
        }
    }
    /// Apply a frame to the raster
    fn apply_frame(&mut self, f: Frame) -> Result<Raster<Rgba8>, DecodeError> {
        let r = if let DisposalMethod::Previous = f.disposal_method() {
            let r = self.raster.as_ref().unwrap();
            let mut r = RasterBuilder::new().with_raster(r);
            update_raster(&mut r, &f, &self.global_color_table)?;
            r
        } else {
            let mut r = self.raster.as_mut().unwrap();
            update_raster(&mut r, &f, &self.global_color_table)?;
            RasterBuilder::new().with_raster(r)
        };
        if let DisposalMethod::Background = f.disposal_method() {
            let x = f.left().into();
            let y = f.top().into();
            let w = f.width().into();
            let h = f.height().into();
            let reg = Region::new(x, y, w, h);
            let rs = self.raster.as_mut().unwrap();
            rs.set_region(reg, Rgba8::default());
        }
        Ok(r)
    }
}

/// Update a raster with a new frame
fn update_raster(r: &mut Raster<Rgba8>, f: &Frame, t: &Option<GlobalColorTable>)
    -> Result<(), DecodeError>
{
    let x: u32 = f.left().into();
    let y: u32 = f.top().into();
    let w: u32 = f.width().into();
    let h: u32 = f.height().into();
    if x + w <= r.width() && y + h <= r.height() {
        let clrs = if let Some(c) = &f.local_color_table {
            c.colors()
        } else if let Some(c) = t {
            c.colors()
        } else {
            return Err(DecodeError::MissingColorTable);
        };
        let trans = f.transparent_color();
        for yi in y..y+h {
            let yr = yi * r.width();
            for xi in x..x+w {
                let idx = f.image_data.data()[(yr + xi) as usize];
                let i = 3 * idx as usize;
                if i + 2 > clrs.len() {
                    return Err(DecodeError::InvalidColorIndex);
                }
                let p = match trans {
                    Some(t) if t == idx => Rgba8::default(),
                    _ => Rgba8::new(clrs[i], clrs[i+1], clrs[i+2]),
                };
                r.set_pixel(xi, yi, p);
            }
        }
        Ok(())
    } else {
        Err(DecodeError::InvalidFrameDimensions)
    }
}

#[cfg(test)]
mod test {
    use std::error::Error;
    use super::Decoder;
    const GIF_1: &[u8] = &[
        0x47, 0x49, 0x46, 0x38, 0x39, 0x61, 0x0A, 0x00,
        0x0A, 0x00, 0x91, 0x00, 0x00, 0xFF, 0xFF, 0xFF,
        0xFF, 0x00, 0x00, 0x00, 0x00, 0xFF, 0x00, 0x00,
        0x00, 0x21, 0xF9, 0x04, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x2C, 0x00, 0x00, 0x00, 0x00, 0x0A, 0x00,
        0x0A, 0x00, 0x00, 0x02, 0x16, 0x8C, 0x2D, 0x99,
        0x87, 0x2A, 0x1C, 0xDC, 0x33, 0xA0, 0x02, 0x75,
        0xEC, 0x95, 0xFA, 0xA8, 0xDE, 0x60, 0x8C, 0x04,
        0x91, 0x4C, 0x01, 0x00, 0x3B,
    ];
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
    fn block_1() -> Result<(), Box<Error>> {
        use crate::block::*;
        let colors = &[
            0xFF, 0xFF, 0xFF,
            0xFF, 0x00, 0x00,
            0x00, 0x00, 0xFF,
            0x00, 0x00, 0x00,
        ][..];
        let mut dec = Decoder::new(GIF_1).into_block_decoder();
        match dec.next() {
            Some(Ok(Block::Header(b))) => assert_eq!(b, Header::default()),
            _ => panic!(),
        }
        match dec.next() {
            Some(Ok(Block::LogicalScreenDesc(b))) => {
                assert_eq!(b, LogicalScreenDesc::default()
                    .with_screen_width(10)
                    .with_screen_height(10)
                    .with_flags(0x91))
            },
            _ => panic!(),
        }
        match dec.next() {
            Some(Ok(Block::GlobalColorTable(b))) => {
                assert_eq!(b, GlobalColorTable::with_colors(colors))
            },
            _ => panic!(),
        }
        match dec.next() {
            Some(Ok(Block::GraphicControl(b))) => {
                assert_eq!(b, GraphicControl::default())
            },
            _ => panic!(),
        }
        match dec.next() {
            Some(Ok(Block::ImageDesc(b))) => {
                assert_eq!(b, ImageDesc::default()
                    .with_width(10)
                    .with_height(10))
            },
            _ => panic!(),
        }
        match dec.next() {
            Some(Ok(Block::ImageData(b))) => {
                let mut d = ImageData::new(100);
                d.add_data(IMAGE_1);
                assert_eq!(b, d);
            },
            _ => panic!(),
        }
        match dec.next() {
            Some(Ok(Block::Trailer(b))) => assert_eq!(b, Trailer::default()),
            _ => panic!(),
        }
        Ok(())
    }
    #[test]
    fn frame_1() -> Result<(), Box<Error>> {
        for f in Decoder::new(GIF_1).into_frame_decoder() {
            assert_eq!(f?.image_data.data(), IMAGE_1);
        }
        Ok(())
    }
    #[test]
    fn image_1() -> Result<(), Box<Error>> {
        use pix::Rgba8;
        let red = Rgba8::new(0xFF, 0x00, 0x00);
        let blu = Rgba8::new(0x00, 0x00, 0xFF);
        let wht = Rgba8::new(0xFF, 0xFF, 0xFF);
        let image = &[
            red,red,red,red,red,blu,blu,blu,blu,blu,
            red,red,red,red,red,blu,blu,blu,blu,blu,
            red,red,red,red,red,blu,blu,blu,blu,blu,
            red,red,red,wht,wht,wht,wht,blu,blu,blu,
            red,red,red,wht,wht,wht,wht,blu,blu,blu,
            blu,blu,blu,wht,wht,wht,wht,red,red,red,
            blu,blu,blu,wht,wht,wht,wht,red,red,red,
            blu,blu,blu,blu,blu,red,red,red,red,red,
            blu,blu,blu,blu,blu,red,red,red,red,red,
            blu,blu,blu,blu,blu,red,red,red,red,red,
        ][..];
        for r in Decoder::new(GIF_1) {
            assert_eq!(r?.as_slice(), image);
        }
        Ok(())
    }
}
