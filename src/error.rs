// error.rs
//
// Copyright (c) 2019  Douglas Lau
//

use std::fmt;
use std::io;

#[derive(Debug)]
pub enum DecodeError {
    Io(io::Error),
    MalformedHeader,
    UnsupportedVersion([u8; 3]),
    MalformedGif,
    MalformedGraphicControlExtension,
    UnexpectedEndOfFile,
    InvalidCodeSize,
    TooLargeImage,
    IncompleteImageData,
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
