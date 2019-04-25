// block.rs
//
// Copyright (c) 2019  Douglas Lau
//

const CHANNELS: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorTableExistence {
    Absent,
    Present,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorTableOrdering {
    NotSorted,
    Sorted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColorTableConfig {
    existence: ColorTableExistence,
    ordering: ColorTableOrdering,
    table_len: usize,   // must be between 2...256
}

impl Default for ColorTableConfig {
    fn default() -> Self {
        let existence = ColorTableExistence::Absent;
        let ordering = ColorTableOrdering::NotSorted;
        let table_len = 2;
        ColorTableConfig { existence, ordering, table_len }
    }
}

impl ColorTableConfig {
    pub fn new(existence: ColorTableExistence, ordering: ColorTableOrdering,
        table_len: u16) -> Self
    {
        let table_len = (table_len as usize).max(2).next_power_of_two().min(256);
        ColorTableConfig { existence, ordering, table_len }
    }
    pub fn existence(&self) -> ColorTableExistence {
        self.existence
    }
    pub fn ordering(&self) -> ColorTableOrdering {
        self.ordering
    }
    pub fn len(&self) -> usize {
        match self.existence {
            ColorTableExistence::Absent => 0,
            ColorTableExistence::Present => self.table_len,
        }
    }
    fn len_bits(&self) -> u8 {
        let sz = self.table_len;
        for b in 0..7 {
            if (sz >> (b + 1)) == 1 {
                return b;
            }
        }
        7
    }
    pub fn size_bytes(&self) -> usize {
        self.len() * CHANNELS
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum DisposalMethod {
    NoAction,
    Keep,
    Background,
    Previous,
    Reserved(u8),
}

impl Default for DisposalMethod {
    fn default() -> Self {
        DisposalMethod::Keep
    }
}

impl From<u8> for DisposalMethod {
    fn from(n: u8) -> Self {
        use self::DisposalMethod::*;
        match n & 0b0111 {
            0 => NoAction,
            1 => Keep,
            2 => Background,
            3 => Previous,
            _ => Reserved(n),
        }
    }
}

impl From<DisposalMethod> for u8 {
    fn from(d: DisposalMethod) -> Self {
        use self::DisposalMethod::*;
        match d {
            NoAction => 0,
            Keep => 1,
            Background => 2,
            Previous => 3,
            Reserved(n) => n & 0b0111,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum BlockCode {
    Header_,
    LogicalScreenDesc_,
    GlobalColorTable_,
    Extension_,
    ImageDesc_,
    LocalColorTable_,
    ImageData_,
    Trailer_,
}

impl BlockCode {
    pub fn from_u8(t: u8) -> Option<Self> {
        use self::BlockCode::*;
        match t {
            b',' => Some(ImageDesc_),   // (0x2C) Image separator
            b'!' => Some(Extension_),   // (0x21) Extension introducer
            b';' => Some(Trailer_),     // (0x3B) GIF trailer
            _ => None,
        }
    }
    pub fn signature(&self) -> &'static [u8] {
        use self::BlockCode::*;
        match self {
            ImageDesc_ => b",", // (0x2C) Image separator
            Extension_ => b"!", // (0x21) Extension introducer
            Trailer_ => b";",   // (0x3B) GIF trailer
            _ => &[],
        }
    }
    pub fn size(&self) -> usize {
        use self::BlockCode::*;
        match self {
            Header_ => 6,
            LogicalScreenDesc_ => 7,
            ImageDesc_ => 10,
            Trailer_ => 1,
            Extension_ => 2, // +sub-blocks
            ImageData_ => 1, // +sub-blocks
            _ => 0,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum ExtensionCode {
    PlainText_,
    GraphicControl_,
    Comment_,
    Application_,
    Unknown_(u8),
}

impl From<u8> for ExtensionCode {
    fn from(n: u8) -> Self {
        use self::ExtensionCode::*;
        match n {
            0x01 => PlainText_,
            0xF9 => GraphicControl_,
            0xFE => Comment_,
            0xFF => Application_,
            _ => Unknown_(n),
        }
    }
}

impl From<ExtensionCode> for u8 {
    fn from(t: ExtensionCode) -> Self {
        use self::ExtensionCode::*;
        match t {
            PlainText_ => 0x01,
            GraphicControl_ => 0xF9,
            Comment_ => 0xFE,
            Application_ => 0xFF,
            Unknown_(n) => n,
        }
    }
}

#[derive(Debug)]
pub struct Header {
    version: [u8; 3],
}

impl Header {
    pub fn with_version(version: [u8; 3]) -> Self {
        Header { version }
    }
    pub fn version(&self) -> [u8; 3] {
        self.version
    }
}

#[derive(Debug, Default)]
pub struct LogicalScreenDesc {
    screen_width: u16,
    screen_height: u16,
    flags: u8,
    background_color_idx: u8,   // index into global color table
    pixel_aspect_ratio: u8,
}

impl LogicalScreenDesc {
    const COLOR_TABLE_PRESENT: u8  = 0b1000_0000;
    const COLOR_RESOLUTION: u8     = 0b0111_0000;
    const COLOR_TABLE_ORDERING: u8 = 0b0000_1000;
    const COLOR_TABLE_SIZE: u8     = 0b0000_0111;

    pub fn with_screen_width(mut self, screen_width: u16) -> Self {
        self.screen_width = screen_width;
        self
    }
    pub fn screen_width(&self) -> u16 {
        self.screen_width
    }
    pub fn with_screen_height(mut self, screen_height: u16) -> Self {
        self.screen_height = screen_height;
        self
    }
    pub fn screen_height(&self) -> u16 {
        self.screen_height
    }
    pub fn with_flags(mut self, flags: u8) -> Self {
        self.flags = flags;
        self
    }
    pub fn flags(&self) -> u8 {
        self.flags
    }
    fn color_table_existence(&self) -> ColorTableExistence {
        if self.flags & Self::COLOR_TABLE_PRESENT != 0 {
            ColorTableExistence::Present
        } else {
            ColorTableExistence::Absent
        }
    }
    pub fn color_resolution(&self) -> u16 {
        2 << ((self.flags & Self::COLOR_RESOLUTION) >> 4 as u16)
    }
    fn color_table_ordering(&self) -> ColorTableOrdering {
        if self.flags & Self::COLOR_TABLE_ORDERING != 0 {
            ColorTableOrdering::Sorted
        } else {
            ColorTableOrdering::NotSorted
        }
    }
    fn color_table_len(&self) -> usize {
        2 << ((self.flags & Self::COLOR_TABLE_SIZE) as usize)
    }
    pub fn color_table_config(&self) -> ColorTableConfig {
        let existence = self.color_table_existence();
        let ordering = self.color_table_ordering();
        let table_len = self.color_table_len();
        ColorTableConfig { existence, ordering, table_len }
    }
    pub fn with_color_table_config(mut self, tbl: &ColorTableConfig) -> Self {
        let mut flags = tbl.len_bits() & Self::COLOR_TABLE_SIZE;
        flags |= (flags << 4) & Self::COLOR_RESOLUTION;
        if tbl.existence == ColorTableExistence::Present {
            flags |= Self::COLOR_TABLE_PRESENT;
        }
        if tbl.ordering == ColorTableOrdering::Sorted {
            flags |= Self::COLOR_TABLE_ORDERING;
        }
        self.flags = flags;
        self
    }
    pub fn with_background_color_idx(mut self, background_color_idx: u8)
        -> Self
    {
        self.background_color_idx = background_color_idx;
        self
    }
    pub fn background_color_idx(&self) -> u8 {
        self.background_color_idx
    }
    pub fn with_pixel_aspect_ratio(mut self, pixel_aspect_ratio: u8)
        -> Self
    {
        self.pixel_aspect_ratio = pixel_aspect_ratio;
        self
    }
    pub fn pixel_aspect_ratio(&self) -> u8 {
        self.pixel_aspect_ratio
    }
}

#[derive(Debug)]
pub struct GlobalColorTable {
    colors: Vec<u8>,
}

impl GlobalColorTable {
    pub fn with_colors(colors: &[u8]) -> Self {
        assert_eq!(colors.len() / CHANNELS * CHANNELS, colors.len());
        let colors = colors.to_vec();
        GlobalColorTable { colors }
    }
    pub fn len(&self) -> usize {
        self.colors.len()
    }
    pub fn colors(&self) -> &[u8] {
        &self.colors
    }
}

#[derive(Debug, Default)]
pub struct PlainText {
    sub_blocks: Vec<Vec<u8>>,   // sequence of sub-blocks
}

impl PlainText {
    pub fn add_sub_block(&mut self, b: &[u8]) {
        assert!(b.len() < 256);
        self.sub_blocks.push(b.to_vec());
    }
    pub fn sub_blocks(&self) -> &Vec<Vec<u8>> {
        &self.sub_blocks
    }
}

#[derive(Debug, Default)]
pub struct GraphicControl {
    flags: u8,
    delay_time_cs: u16,      // delay in centiseconds (hundredths of a second)
    transparent_color_idx: u8,
}

impl GraphicControl {
    #[allow(dead_code)]
    const RESERVED: u8          = 0b1110_0000;
    const DISPOSAL_METHOD: u8   = 0b0001_1100;
    const USER_INPUT: u8        = 0b0000_0010;
    const TRANSPARENT_COLOR: u8 = 0b0000_0001;

    pub fn set_flags(&mut self, flags: u8) {
        self.flags = flags;
    }
    pub fn flags(&self) -> u8 {
        self.flags
    }
    pub fn disposal_method(&self) -> DisposalMethod {
        ((self.flags & Self::DISPOSAL_METHOD) >> 2).into()
    }
    pub fn set_disposal_method(&mut self, disposal_method: DisposalMethod) {
        let d: u8 = disposal_method.into();
        self.flags = (self.flags | !Self::DISPOSAL_METHOD) | (d << 2);
    }
    pub fn user_input(&self) -> bool {
        ((self.flags & Self::USER_INPUT) >> 1) != 0
    }
    pub fn set_user_input(&mut self, user_input: bool) {
        let u = (user_input as u8) << 1;
        self.flags = (self.flags | !Self::USER_INPUT) | u;
    }
    pub fn delay_time_cs(&self) -> u16 {
        self.delay_time_cs
    }
    pub fn set_delay_time_cs(&mut self, delay_time_cs: u16) {
        self.delay_time_cs = delay_time_cs;
    }
    pub fn transparent_color(&self) -> Option<u8> {
        let t = ((self.flags & Self::TRANSPARENT_COLOR) >> 1) != 0;
        match t {
            true => Some(self.transparent_color_idx),
            false => None,
        }
    }
    pub fn transparent_color_idx(&self) -> u8 {
        self.transparent_color_idx
    }
    pub fn set_transparent_color_idx(&mut self, transparent_color_idx: u8) {
        self.transparent_color_idx = transparent_color_idx;
    }
    pub fn set_transparent_color(&mut self, transparent_color: Option<u8>) {
        match transparent_color {
            Some(t) => {
                self.flags |= Self::TRANSPARENT_COLOR;
                self.transparent_color_idx = t;
            },
            None => {
                self.flags |= !Self::TRANSPARENT_COLOR;
                self.transparent_color_idx = 0;
            },
        }
    }
}

#[derive(Debug, Default)]
pub struct Comment {
    comments: Vec<Vec<u8>>, // ascii only comments recommended
}

impl Comment {
    pub fn add_comment(&mut self, b: &[u8]) {
        assert!(b.len() < 256);
        self.comments.push(b.to_vec());
    }
    pub fn comments(&self) -> &Vec<Vec<u8>> {
        &self.comments
    }
}

#[derive(Debug, Default)]
pub struct Application {
    app_data: Vec<Vec<u8>>,     // sequence of sub-blocks
}

impl Application {
    fn is_looping(app_id: &[u8]) -> bool {
        app_id == b"NETSCAPE2.0" || app_id == b"ANIMEXTS1.0"
    }
    pub fn with_loop_count(loop_count: u16) -> Self {
        let mut app_data = vec![];
        app_data.push(b"NETSCAPE2.0".to_vec());
        let mut v = vec![1];
        v.push((loop_count >> 8) as u8);
        v.push(loop_count as u8);
        app_data.push(v);
        Application { app_data }
    }
    pub fn add_app_data(&mut self, b: &[u8]) {
        assert!(b.len() < 256);
        self.app_data.push(b.to_vec());
    }
    pub fn app_data(&self) -> &Vec<Vec<u8>> {
        &self.app_data
    }
    pub fn loop_count(&self) -> Option<u16> {
        // NOTE: this block must follow immediately after GlobalColorTable
        //       (or LogicalScreenDesc if there is no GlobalColorTable).
        let d = &self.app_data;
        let exists = d.len() == 2 &&            // 2 sub-blocks
                     Self::is_looping(&d[0]) && // app ID / auth code
                     d[1].len() == 3 &&         // app data sub-block length
                     d[1][0] == 1;              // sub-block ID
        if exists {
            // Number of times to loop animation (zero means loop forever)
            let c = (d[1][1] as u16) << 8 | d[1][2] as u16;
            Some(c)
        } else {
            None
        }
    }
}

#[derive(Debug, Default)]
pub struct Unknown {
    sub_blocks: Vec<Vec<u8>>,   // sequence of sub-blocks (first has ext_id)
}

impl Unknown {
    pub fn ext_id(&self) -> &[u8] {
        if self.sub_blocks.len() > 0 {
            &self.sub_blocks[0]
        } else {
            &[]
        }
    }
    pub fn add_sub_block(&mut self, b: &[u8]) {
        assert!(b.len() < 256);
        self.sub_blocks.push(b.to_vec());
    }
    pub fn sub_blocks(&self) -> &[Vec<u8>] {
        if self.sub_blocks.len() > 0 {
            &self.sub_blocks[1..]
        } else {
            &[]
        }
    }
}

#[derive(Debug, Default)]
pub struct ImageDesc {
    left: u16,
    top: u16,
    width: u16,
    height: u16,
    flags: u8,
}

impl ImageDesc {
    const COLOR_TABLE_PRESENT: u8  = 0b1000_0000;
    const INTERLACED: u8           = 0b0100_0000;
    const COLOR_TABLE_ORDERING: u8 = 0b0010_0000;
    #[allow(dead_code)]
    const RESERVED: u8             = 0b0001_1000;
    const COLOR_TABLE_SIZE: u8     = 0b0000_0111;

    pub fn with_left(mut self, left: u16) -> Self {
        self.left = left;
        self
    }
    pub fn left(&self) -> u16 {
        self.left
    }
    pub fn with_top(mut self, top: u16) -> Self {
        self.top = top;
        self
    }
    pub fn top(&self) -> u16 {
        self.top
    }
    pub fn with_width(mut self, width: u16) -> Self {
        self.width = width;
        self
    }
    pub fn width(&self) -> u16 {
        self.width
    }
    pub fn with_height(mut self, height: u16) -> Self {
        self.height = height;
        self
    }
    pub fn height(&self) -> u16 {
        self.height
    }
    pub fn with_flags(mut self, flags: u8) -> Self {
        self.flags = flags;
        self
    }
    pub fn flags(&self) -> u8 {
        self.flags
    }
    pub fn interlaced(&self) -> bool {
        (self.flags & Self::INTERLACED) != 0
    }
    fn color_table_existence(&self) -> ColorTableExistence {
        if self.flags & Self::COLOR_TABLE_PRESENT != 0 {
            ColorTableExistence::Present
        } else {
            ColorTableExistence::Absent
        }
    }
    fn color_table_ordering(&self) -> ColorTableOrdering {
        if self.flags & Self::COLOR_TABLE_ORDERING != 0 {
            ColorTableOrdering::Sorted
        } else {
            ColorTableOrdering::NotSorted
        }
    }
    fn color_table_len(&self) -> usize {
        2 << ((self.flags & Self::COLOR_TABLE_SIZE) as u16)
    }
    pub fn color_table_config(&self) -> ColorTableConfig {
        let existence = self.color_table_existence();
        let ordering = self.color_table_ordering();
        let table_len = self.color_table_len();
        ColorTableConfig { existence, ordering, table_len }
    }
    pub fn with_color_table_config(mut self, tbl: &ColorTableConfig) -> Self {
        let mut flags = self.flags & (Self::INTERLACED | Self::RESERVED);
        flags |= tbl.len_bits() & Self::COLOR_TABLE_SIZE;
        if tbl.existence == ColorTableExistence::Present {
            flags |= Self::COLOR_TABLE_PRESENT;
        }
        if tbl.ordering == ColorTableOrdering::Sorted {
            flags |= Self::COLOR_TABLE_ORDERING;
        }
        self.flags = flags;
        self
    }
    pub fn image_sz(&self) -> usize {
        self.width as usize * self.height as usize
    }
}

#[derive(Debug, Default)]
pub struct LocalColorTable {
    colors: Vec<u8>,
}

impl LocalColorTable {
    pub fn with_colors(colors: &[u8]) -> Self {
        assert_eq!(colors.len() / CHANNELS * CHANNELS, colors.len());
        let colors = colors.to_vec();
        LocalColorTable { colors }
    }
    pub fn len(&self) -> usize {
        self.colors.len()
    }
    pub fn colors(&self) -> &[u8] {
        &self.colors
    }
}

#[derive(Debug)]
pub struct ImageData {
    data: Vec<u8>,  // first byte of data is LZW minimum code size
}

impl ImageData {
    pub fn new(image_sz: usize, min_code_size: u8) -> Self {
        // Reserve an extra byte for min_code_size (first)
        let mut data = Vec::with_capacity(image_sz + 1);
        data.push(min_code_size);
        ImageData { data }
    }
    pub fn is_complete(&self) -> bool {
        self.data.len() == self.data.capacity()
    }
    pub fn add_data(&mut self, data: &[u8]) {
        let rem = self.data.capacity() - self.data.len();
        let fits = data.len() <= rem;
        if fits {
            self.data.extend_from_slice(data);
        } else {
            self.data.extend_from_slice(&data[..rem]);
            warn!("Extra image data: {:?}", &data[rem..]);
        }
    }
    pub fn min_code_size(&self) -> u8 {
        if self.data.len() > 0 {
            self.data[0]
        } else {
            2
        }.max(2)    // must be >= 2
    }
    pub fn data(&self) -> &[u8] {
        // Remove the LZW minimum code size
        if self.data.len() > 0 {
            &self.data[1..]
        } else {
            b""
        }
    }
}

#[derive(Debug, Default)]
pub struct Trailer { }

#[derive(Debug)]
pub enum Block {
    Header(Header),
    LogicalScreenDesc(LogicalScreenDesc),
    GlobalColorTable(GlobalColorTable),
    PlainText(PlainText),
    GraphicControl(GraphicControl),
    Comment(Comment),
    Application(Application),
    Unknown(Unknown),
    ImageDesc(ImageDesc),
    LocalColorTable(LocalColorTable),
    ImageData(ImageData),
    Trailer(Trailer),
}

impl Block {
    pub fn has_sub_blocks(&self) -> bool {
        use self::Block::*;
        match self {
            PlainText(_) | GraphicControl(_) | Comment(_) | Application(_) |
            Unknown(_) | ImageData(_) => true,
            _ => false,
        }
    }
}

impl From<Header> for Block {
    fn from(b: Header) -> Self {
        Block::Header(b)
    }
}

impl From<LogicalScreenDesc> for Block {
    fn from(b: LogicalScreenDesc) -> Self {
        Block::LogicalScreenDesc(b)
    }
}

impl From<GlobalColorTable> for Block {
    fn from(b: GlobalColorTable) -> Self {
        Block::GlobalColorTable(b)
    }
}

impl From<PlainText> for Block {
    fn from(b: PlainText) -> Self {
        Block::PlainText(b)
    }
}

impl From<GraphicControl> for Block {
    fn from(b: GraphicControl) -> Self {
        Block::GraphicControl(b)
    }
}

impl From<Comment> for Block {
    fn from(b: Comment) -> Self {
        Block::Comment(b)
    }
}

impl From<Application> for Block {
    fn from(b: Application) -> Self {
        Block::Application(b)
    }
}

impl From<Unknown> for Block {
    fn from(b: Unknown) -> Self {
        Block::Unknown(b)
    }
}

impl From<ImageDesc> for Block {
    fn from(b: ImageDesc) -> Self {
        Block::ImageDesc(b)
    }
}

impl From<LocalColorTable> for Block {
    fn from(b: LocalColorTable) -> Self {
        Block::LocalColorTable(b)
    }
}

impl From<ImageData> for Block {
    fn from(b: ImageData) -> Self {
        Block::ImageData(b)
    }
}

impl From<Trailer> for Block {
    fn from(b: Trailer) -> Self {
        Block::Trailer(b)
    }
}

#[derive(Debug, Default)]
pub struct Preamble {
    pub header: Option<Header>,
    pub logical_screen_desc: Option<LogicalScreenDesc>,
    pub global_color_table: Option<GlobalColorTable>,
    pub loop_count_ext: Option<Application>,
}

#[derive(Debug, Default)]
pub struct Frame {
    pub graphic_control_ext: Option<GraphicControl>,
    pub image_desc: Option<ImageDesc>,
    pub local_color_table: Option<LocalColorTable>,
    pub image_data: Option<ImageData>,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn block_size() {
        assert!(std::mem::size_of::<Block>() <= 32);
    }

    #[test]
    fn color_table_len() {
        let t = ColorTableConfig::new(ColorTableExistence::Present,
            ColorTableOrdering::NotSorted, 0); // 0-2
        assert_eq!(t.len_bits(), 0);
        let t = ColorTableConfig::new(ColorTableExistence::Present,
            ColorTableOrdering::NotSorted, 4); // 3-4
        assert_eq!(t.len_bits(), 1);
        let t = ColorTableConfig::new(ColorTableExistence::Present,
            ColorTableOrdering::NotSorted, 7); // 5-8
        assert_eq!(t.len_bits(), 2);
        let t = ColorTableConfig::new(ColorTableExistence::Present,
            ColorTableOrdering::NotSorted, 16); // 9-16
        assert_eq!(t.len_bits(), 3);
        let t = ColorTableConfig::new(ColorTableExistence::Present,
            ColorTableOrdering::NotSorted, 17); // 17-32
        assert_eq!(t.len_bits(), 4);
        let t = ColorTableConfig::new(ColorTableExistence::Present,
            ColorTableOrdering::NotSorted, 64); // 33-64
        assert_eq!(t.len_bits(), 5);
        let t = ColorTableConfig::new(ColorTableExistence::Present,
            ColorTableOrdering::NotSorted, 65); // 65-128
        assert_eq!(t.len_bits(), 6);
        let t = ColorTableConfig::new(ColorTableExistence::Present,
            ColorTableOrdering::NotSorted, 130); // 129-256
        assert_eq!(t.len_bits(), 7);
        let t = ColorTableConfig::default();
        assert_eq!(t.len_bits(), 0);
    }

    #[test]
    fn loop_count() {
        let b = Application::default();
        assert_eq!(b.loop_count(), None);
        let b = Application::with_loop_count(0);
        assert_eq!(b.loop_count(), Some(0));
        let b = Application::with_loop_count(4);
        assert_eq!(b.loop_count(), Some(4));
    }
}
