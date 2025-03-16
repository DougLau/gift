# GIF*t*

![](https://github.com/DougLau/gift/workflows/Test/badge.svg)

A Rust library for encoding and decoding GIF images.

## Documentation
[https://docs.rs/gift](https://docs.rs/gift)

## Decoding

```rust
use gift::Decoder;
use std::fs::File;
use std::io::BufReader;

let gif = BufReader::new(File::open("example.gif")?);
for step in Decoder::new(gif) {
    // was there a decoding error?
    let raster = step?.raster();
    // ... work with raster
}
```

## Encoding

```rust
use gift::{Encoder, Step};
use pix::{gray::Gray8, Palette, Raster, rgb::SRgb8};
use std::error::Error;
use std::io::Write;

fn encode<W: Write>(mut w: W) -> Result<(), Box<dyn Error>> {
    let mut raster = Raster::<Gray8>::with_clear(4, 4);
    // ... initialize raster ...
    let mut palette = Palette::new(2);
    // ... initialize palette ...
    let step = Step::with_indexed(raster, palette);
    let mut enc = Encoder::new(&mut w).into_step_enc();
    enc.encode_step(&step)?;
    Ok(())
}
```

NOTE: building a palette from 24- or 32-bit rasters is not yet implemented.

## Utility

The library comes with a `gift` command-line utility, which can show the blocks
within GIF files.
```
cargo install gift --features=cmd
```

NOTE: This utility is a work-in-progress, and some features are not implemented.
