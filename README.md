# GIF*t*

A Rust library for encoding and decoding GIF images.

## Decoding example

```rust
// ... open a File as "gif"
let mut frame_dec = gift::Decoder::new(gif).into_frame_decoder();
let preamble = frame_dec.preamble()?;
println!("preamble: {:?}", preamble);
for frame in frame_dec {
    println!("frame: {:?}", frame?);
}
```

## TODO

* Interlaced images
