// decode.rs
//
// Copyright (c) 2019-2020  Douglas Lau
//
//! GIF file decoding
use crate::block::*;
use crate::error::{Error, Result};
use crate::lzw::Decompressor;
use crate::private::Step;
use pix::{rgb::SRgba8, Raster, Region};
use std::io::{ErrorKind, Read};

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
    /// Reader for blocks
    reader: R,
    /// Maximum image size in bytes
    max_image_sz: Option<usize>,
    /// Expected next block and size
    expected_next: Option<(BlockCode, usize)>,
    /// Size of image data
    image_sz: usize,
    /// LZW decompressor
    decompressor: Option<Decompressor>,
    /// Flag when done
    done: bool,
}

impl<R: Read> Iterator for Blocks<R> {
    type Item = Result<Block>;

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
            expected_next: Some((Header_, Header_.size())),
            image_sz: 0,
            done: false,
            decompressor: None,
        }
    }

    /// Decode the next block (including all sub-blocks).
    fn next_block(&mut self) -> Result<Block> {
        let mut block = self.decode_block()?;
        if block.has_sub_blocks() {
            while self.decode_sub_block(&mut block)? {}
        }
        self.check_block_end(&mut block)?;
        Ok(block)
    }

    /// Decode one block
    fn decode_block(&mut self) -> Result<Block> {
        let block = match self.expected_next {
            Some((bc, sz)) => self.parse_expected(bc, sz)?,
            None => self.parse_block()?,
        };
        self.expected_next = self.expected_next(&block);
        Ok(block)
    }

    /// Parse an expected block
    fn parse_expected(&mut self, bc: BlockCode, sz: usize) -> Result<Block> {
        use crate::block::BlockCode::*;
        match bc {
            Header_ => self.parse_header(),
            LogicalScreenDesc_ => self.parse_logical_screen_desc(),
            GlobalColorTable_ => self.parse_global_color_table(sz),
            LocalColorTable_ => self.parse_local_color_table(sz),
            ImageData_ => self.parse_image_data(),
            _ => Err(Error::InvalidBlockCode),
        }
    }

    /// Parse a Header block
    fn parse_header(&mut self) -> Result<Block> {
        let mut buf = vec![0; BlockCode::Header_.size()];
        self.fill_buffer(&mut buf)?;
        if &buf[..3] == b"GIF" {
            let version = [buf[3], buf[4], buf[5]];
            match &version {
                b"87a" | b"89a" => Ok(Header::with_version(version).into()),
                _ => Err(Error::UnsupportedVersion(version)),
            }
        } else {
            Err(Error::MalformedHeader)
        }
    }

    /// Parse a Logical Screen Descriptor block
    fn parse_logical_screen_desc(&mut self) -> Result<Block> {
        let mut buf = vec![0; BlockCode::LogicalScreenDesc_.size()];
        self.fill_buffer(&mut buf)?;
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
            .with_pixel_aspect_ratio(aspect)
            .into())
    }

    /// Parse a Global Color Table block
    fn parse_global_color_table(&mut self, sz: usize) -> Result<Block> {
        let mut buf = vec![0; sz];
        self.fill_buffer(&mut buf)?;
        Ok(GlobalColorTable::with_colors(&buf).into())
    }

    /// Parse a Local Color Table block
    fn parse_local_color_table(&mut self, sz: usize) -> Result<Block> {
        let mut buf = vec![0; sz];
        self.fill_buffer(&mut buf)?;
        Ok(LocalColorTable::with_colors(&buf).into())
    }

    /// Parse an Image Data block
    fn parse_image_data(&mut self) -> Result<Block> {
        let mut buf = vec![0; BlockCode::ImageData_.size()];
        self.fill_buffer(&mut buf)?;
        let min_code_bits = buf[0];
        if 2 <= min_code_bits && min_code_bits <= 12 {
            self.decompressor = Some(Decompressor::new(min_code_bits));
            Ok(ImageData::new(self.image_sz).into())
        } else {
            Err(Error::InvalidLzwCodeSize)
        }
    }

    /// Parse a block
    fn parse_block(&mut self) -> Result<Block> {
        use crate::block::BlockCode::*;
        let mut buf = [0; 1];
        self.fill_buffer(&mut buf)?;
        match BlockCode::from_u8(buf[0]) {
            Some(Extension_) => self.parse_extension(),
            Some(ImageDesc_) => self.parse_image_desc(),
            Some(Trailer_) => Ok(Trailer::default().into()),
            _ => Err(Error::InvalidBlockCode),
        }
    }

    /// Parse an extension block
    fn parse_extension(&mut self) -> Result<Block> {
        use crate::block::ExtensionCode::*;
        let mut buf = [0; 1];
        self.fill_buffer(&mut buf)?;
        let et: ExtensionCode = buf[0].into();
        Ok(match et {
            PlainText_ => PlainText::default().into(),
            GraphicControl_ => GraphicControl::default().into(),
            Comment_ => Comment::default().into(),
            Application_ => Application::default().into(),
            Unknown_(n) => Unknown::new(n).into(),
        })
    }

    /// Parse an Image Descriptor block
    fn parse_image_desc(&mut self) -> Result<Block> {
        let mut buf = vec![0; BlockCode::ImageDesc_.size() - 1];
        self.fill_buffer(&mut buf)?;
        let left = u16::from(buf[1]) << 8 | u16::from(buf[0]);
        let top = u16::from(buf[3]) << 8 | u16::from(buf[2]);
        let width = u16::from(buf[5]) << 8 | u16::from(buf[4]);
        let height = u16::from(buf[7]) << 8 | u16::from(buf[6]);
        let flags = buf[8];
        let b = ImageDesc::default()
            .with_left(left)
            .with_top(top)
            .with_width(width)
            .with_height(height)
            .with_flags(flags);
        self.image_sz = b.image_sz();
        if let Some(sz) = self.max_image_sz {
            if self.image_sz > sz {
                return Err(Error::TooLargeImage);
            }
        }
        Ok(b.into())
    }

    /// Fill a buffer from reader
    fn fill_buffer(&mut self, buffer: &mut [u8]) -> Result<()> {
        let mut len = 0;
        while len < buffer.len() {
            match self.reader.read(&mut buffer[len..]) {
                Ok(0) => return Err(Error::UnexpectedEndOfFile),
                Ok(n) => len += n,
                Err(ref e) if e.kind() == ErrorKind::Interrupted => {}
                Err(e) => return Err(e.into()),
            }
        }
        Ok(())
    }

    /// Get the expected next block code and size
    fn expected_next(&mut self, block: &Block) -> Option<(BlockCode, usize)> {
        use crate::block::BlockCode::*;
        match block {
            Block::Header(_) => {
                Some((LogicalScreenDesc_, LogicalScreenDesc_.size()))
            }
            Block::LogicalScreenDesc(b) => {
                let sz = b.color_table_config().size_bytes();
                if sz > 0 {
                    Some((GlobalColorTable_, sz))
                } else {
                    None
                }
            }
            Block::ImageDesc(b) => {
                let sz = b.color_table_config().size_bytes();
                if sz > 0 {
                    Some((LocalColorTable_, sz))
                } else {
                    Some((ImageData_, ImageData_.size()))
                }
            }
            Block::LocalColorTable(_) => Some((ImageData_, ImageData_.size())),
            Block::Trailer(_) => Some((Header_, Header_.size())),
            _ => None,
        }
    }

    /// Check end of block (after sub-blocks)
    fn check_block_end(&mut self, block: &mut Block) -> Result<()> {
        if let Block::ImageData(ref mut b) = block {
            match self.decompressor.take() {
                Some(decompressor) => b.finish(decompressor, self.image_sz)?,
                _ => panic!("Invalid state in check_block_end!"),
            }
        }
        Ok(())
    }

    /// Decode one sub-block
    fn decode_sub_block(&mut self, block: &mut Block) -> Result<bool> {
        let mut buf = [0; 256];
        self.fill_buffer(&mut buf[..1])?;
        let len = buf[0] as usize;
        if len > 0 {
            let blk_sz = len + 1;
            self.fill_buffer(&mut buf[1..blk_sz])?;
            debug!("sub-block: {:?} {:?}", block, blk_sz);
            self.parse_sub_block(block, &buf[1..blk_sz])?;
        }
        return Ok(len > 0);
    }

    /// Parse a sub-block in the buffer
    fn parse_sub_block(
        &mut self,
        block: &mut Block,
        bytes: &[u8],
    ) -> Result<()> {
        use crate::block::Block::*;
        match block {
            PlainText(b) => b.parse_sub_block(bytes),
            GraphicControl(b) => b.parse_sub_block(bytes)?,
            Comment(b) => b.parse_sub_block(bytes),
            Application(b) => b.parse_sub_block(bytes),
            Unknown(b) => b.parse_sub_block(bytes),
            ImageData(b) => b.parse_sub_block(bytes, &mut self.decompressor)?,
            _ => panic!("Invalid state in parse_sub_block!"),
        }
        Ok(())
    }
}

impl ImageData {
    /// Parse an Image Data block
    fn parse_sub_block(
        &mut self,
        bytes: &[u8],
        decompressor: &mut Option<Decompressor>,
    ) -> Result<()> {
        if let Some(ref mut dec) = decompressor {
            dec.decompress(bytes, self.data_mut())?;
            return Ok(());
        }
        panic!("Invalid state in decode_image_data!");
    }

    /// Finish LZW decompression
    fn finish(
        &mut self,
        mut decompressor: Decompressor,
        image_sz: usize,
    ) -> Result<()> {
        decompressor.decompress_finish(self.data_mut())?;
        if self.data_mut().len() > image_sz {
            warn!("Extra image data: {:?}", &self.data_mut()[image_sz..]);
            self.data_mut().truncate(image_sz);
            self.data_mut().shrink_to_fit();
        }
        if self.data().len() == image_sz {
            return Ok(());
        } else {
            return Err(Error::IncompleteImageData);
        }
    }
}

impl PlainText {
    /// Parse a Plain Text extension sub-block
    fn parse_sub_block(&mut self, bytes: &[u8]) {
        self.add_sub_block(bytes);
    }
}

impl GraphicControl {
    /// Parse a Graphic Control extension sub-block
    fn parse_sub_block(&mut self, bytes: &[u8]) -> Result<()> {
        if bytes.len() == 4 {
            self.set_flags(bytes[0]);
            let delay = u16::from(bytes[2]) << 8 | u16::from(bytes[1]);
            self.set_delay_time_cs(delay);
            self.set_transparent_color_idx(bytes[3]);
            Ok(())
        } else {
            Err(Error::MalformedGraphicControlExtension)
        }
    }
}

impl Comment {
    /// Parse a Comment extension sub-block
    fn parse_sub_block(&mut self, bytes: &[u8]) {
        self.add_comment(bytes);
    }
}

impl Application {
    /// Parse an Application extension sub-block
    fn parse_sub_block(&mut self, bytes: &[u8]) {
        self.add_app_data(bytes);
    }
}

impl Unknown {
    /// Create a new Unknown extension block
    fn new(ext_id: u8) -> Self {
        let mut b = Unknown::default();
        b.add_sub_block(&[ext_id]);
        b
    }

    /// Parse an Unknown extension sub-block
    fn parse_sub_block(&mut self, bytes: &[u8]) {
        self.add_sub_block(bytes);
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
    type Item = Result<Frame>;

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
    pub fn preamble(&mut self) -> Result<Option<Preamble>> {
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
    fn handle_block(&mut self, block: Block) -> Result<Option<Frame>> {
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

/// An Iterator for [Step]s within a GIF file.
///
/// Build with Decoder.[into_iter] (or [into_steps]).
///
/// ## Example: Get the last raster in a GIF animation
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
/// // ... open a File as "gif"
/// if let Some(step) = Decoder::new(gif).into_steps().last() {
///     let raster = step?.raster();
///     // ... work with raster
/// }
/// # Ok(())
/// # }
/// ```
///
/// [into_iter]: ../struct.Decoder.html#method.into_iter
/// [into_steps]: ../struct.Decoder.html#method.into_steps
/// [Step]: ../struct.Step.html
///
pub struct Steps<R: Read> {
    /// Frame decoder
    frames: Frames<R>,
    /// Global color table block
    global_color_table: Option<GlobalColorTable>,
    /// Current raster of animation
    raster: Option<Raster<SRgba8>>,
}

impl<R: Read> Iterator for Steps<R> {
    type Item = Result<Step>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.raster.is_none() {
            if let Err(e) = self.make_raster() {
                return Some(Err(e));
            }
        }
        match self.raster {
            Some(_) => self.next_step(),
            None => None,
        }
    }
}

impl<R: Read> Steps<R> {
    /// Create a new raster step decoder
    pub(crate) fn new(frames: Frames<R>) -> Self {
        Steps {
            frames,
            global_color_table: None,
            raster: None,
        }
    }

    /// Make the initial raster
    fn make_raster(&mut self) -> Result<()> {
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

    /// Get the next step
    fn next_step(&mut self) -> Option<Result<Step>> {
        debug_assert!(self.raster.is_some());
        match self.frames.next() {
            Some(Ok(f)) => Some(self.apply_frame(f)),
            Some(Err(e)) => Some(Err(e)),
            None => None,
        }
    }

    /// Apply a frame to the raster
    fn apply_frame(&mut self, frame: Frame) -> Result<Step> {
        let transparent_color = frame
            .graphic_control_ext
            .unwrap_or_default()
            .transparent_color();
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
            let rs = self.raster.as_mut().unwrap();
            rs.copy_color(frame.region(), SRgba8::default());
        }
        Ok(Step::with_true_color(raster)
            .with_transparent_color(transparent_color))
    }
}

/// Update a raster with a new frame
fn update_raster(
    raster: &mut Raster<SRgba8>,
    frame: &Frame,
    global_tbl: &Option<GlobalColorTable>,
) -> Result<()> {
    let reg = frame.region();
    if raster.intersection(reg) == reg {
        let clrs = if let Some(tbl) = &frame.local_color_table {
            tbl.colors()
        } else if let Some(tbl) = global_tbl {
            tbl.colors()
        } else {
            return Err(Error::MissingColorTable);
        };
        update_frame(raster, reg, frame, clrs)
    } else {
        Err(Error::InvalidFrameDimensions)
    }
}

/// Update a region of a raster with a new frame
fn update_frame(
    raster: &mut Raster<SRgba8>,
    reg: Region,
    frame: &Frame,
    clrs: &[u8],
) -> Result<()> {
    let trans_clr = frame.transparent_color();
    let width = usize::from(frame.width());
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
                d.data_mut().extend(IMAGE_1);
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
        let mut n_frames = 0;
        for step in Decoder::new(GIF_1) {
            assert_eq!(step?.raster().pixels(), image);
            n_frames += 1;
        }
        assert_eq!(n_frames, 1);
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
