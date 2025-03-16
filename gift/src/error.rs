// error.rs
//
// Copyright (c) 2019-2023  Douglas Lau
//
use std::fmt;
use std::io;
use std::num::TryFromIntError;

/// Errors encountered while decoding or encoding
#[derive(Debug)]
pub enum Error {
    /// A wrapped I/O error.
    Io(io::Error),
    /// Integer out of bounds.
    TryFromInt(TryFromIntError),
    /// [Header](block/struct.Header.html) block malformed or missing.
    MalformedHeader,
    /// GIF version not supported (87a or 89a only).
    UnsupportedVersion([u8; 3]),
    /// Invalid [Block](block/enum.Block.html) code (signature).
    InvalidBlockCode,
    /// [Block](block/enum.Block.html)s arranged in invalid sequence.
    InvalidBlockSequence,
    /// [GraphicControl](block/struct.GraphicControl.html) block has invalid
    /// length.
    MalformedGraphicControlExtension,
    /// File ends with incomplete block.
    UnexpectedEndOfFile,
    /// Compressed LZW data invalid or corrupt
    InvalidLzwData,
    /// Image larger than specified by
    /// [max_image_sz](struct.Decoder.html#method.max_image_sz).
    TooLargeImage,
    /// [ImageData](block/struct.ImageData.html) block is incomplete.
    IncompleteImageData,
    /// Frame location / size larger than sreen size.
    InvalidFrameDimensions,
    /// Missing color table for a frame.
    MissingColorTable,
    /// Invalid color index in a frame.
    InvalidColorIndex,
    /// Invalid Raster dimensions
    InvalidRasterDimensions,
}

/// Gift result type
pub type Result<T> = std::result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Io(err) => err.fmt(fmt),
            Error::TryFromInt(err) => err.fmt(fmt),
            _ => fmt::Debug::fmt(self, fmt),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            Error::Io(ref err) => Some(err),
            Error::TryFromInt(ref err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::Io(err)
    }
}

impl From<TryFromIntError> for Error {
    fn from(err: TryFromIntError) -> Self {
        Error::TryFromInt(err)
    }
}
