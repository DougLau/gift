# GIF*t*

A Rust library for encoding and decoding GIF images.

## Decoding example

```rust
// ... open a `File` as "gif"
for raster in gift::Decoder::new(gif) {
    // was there a decoding error?
    let raster = raster?;
    // ... work with raster
}
```

## Utility

The library comes with a `gift` command-line utility, which can show the blocks
within GIF files.
```
cargo install gift --features=cmd
```

## TODO

* Interlaced images
