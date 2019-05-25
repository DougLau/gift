// error.rs
//
// Copyright (c) 2019  Douglas Lau
//
use std::fmt;
use std::io;

/// Errors encountered while decoding a GIF file.
#[derive(Debug)]
pub enum DecodeError {
    /// A wrapped I/O error.
    Io(io::Error),
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
    /// LZW code size must be less than or equal to 12.
    InvalidCodeSize,
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
}

impl fmt::Display for DecodeError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DecodeError::Io(err) => err.fmt(fmt),
            _ => fmt::Debug::fmt(self, fmt),
        }
    }
}

impl std::error::Error for DecodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            DecodeError::Io(ref err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for DecodeError {
    fn from(e: io::Error) -> Self {
        DecodeError::Io(e)
    }
}

/// Errors encountered while encoding a GIF file.
#[derive(Debug)]
pub enum EncodeError {
    /// A wrapped I/O error.
    Io(io::Error),
    /// [Block](block/enum.Block.html)s arranged in invalid sequence.
    InvalidBlockSequence,
}

impl fmt::Display for EncodeError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            EncodeError::Io(err) => err.fmt(fmt),
            _ => fmt::Debug::fmt(self, fmt),
        }
    }
}

impl std::error::Error for EncodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            EncodeError::Io(ref err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for EncodeError {
    fn from(e: io::Error) -> Self {
        EncodeError::Io(e)
    }
}
