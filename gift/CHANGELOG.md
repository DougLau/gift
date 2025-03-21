## [Unreleased]

## [0.11.0]
### Changed
* Updated to Rust 2024 edition

## [0.10.6]
### Fixed
* Bit flags w/ set_transparent_color(None) (graphic control ext)

## [0.10.5]
### Fixed
* Bit flag twiddling for graphic control ext flags

## [0.10.4]
### Changed
* Default disposal method NoAction instead of Keep

## [0.10.3]
### Fixed
* Wrong LZW min code size encoding 256 color images
* Fixed updating graphic control extension

## [0.10.2]
### Added
* `Step::transparent_color()`

## [0.10.1] -
### Added
* `Step::with_transparent_color()`

## [0.10.0] - 2020-10-24
### Removed
* `Decoder::new_unbuffered` and `Encoder::new_unbuffered`
* `ImageData::add_data` (use `data_mut()` instead)
### Changed
* `Decoder::new` and `Encoder::new` are now unbuffered
* Replaced lzw crate usage with new, faster implmentatiton
* `Decoder::into_iter()` repeats Steps using GIF animation loop count
* Replaced `ImageData::buffer_mut()` with `data_mut()`

## [0.9.0] - 2020-06-04
### Added
* Result type for gift crate
* `Step` struct to contain an animation step
### Changed
* Blocks now implement Debug + Eq
* `Encoder::into_raster_enc` to `into_step_enc` (`RasterEnc` to `StepEnc`)
* `Decoder::into_rasters` to `into_steps` (`Rasters` to `Steps`)

## [0.8.0] - 2020-05-20
### Changed
* Updated pix dependency to 0.13

## [0.7.0] - 2020-04-24
### Changed
* Updated pix dependency to 0.12

## [0.6.0] - 2020-04-11
### Changed
* Iterators now return None after first error
* Updated pix dependency to 0.11

## [0.5.0] - 2020-03-28
### Changed
* Updated pix dependency to 0.10
* Implemented some micro-optimizations

## [0.4.0] - 2020-02-26
### Added
* `Encoder::into_block_enc`
* `Encoder::into_frame_enc`
* `Encoder::into_raster_enc`
### Changed
* BlockDecoder to decoder::Blocks
* FrameDecoder to decoder::Frames
* RasterDecoder to decoder::Rasters
* `Decoder::into_block_decoder` to `into_blocks`
* `Decoder::into_frame_decoder` to `into_frames`
* `Decoder::into_raster_decoder` to `into_rasters`
* FrameEncoder to encoder::FrameEnc

## [0.3.1] - 2019-05-28
### Changed
* Fixed u8 overflow in `ImageData.add_data` method.

## [0.3.0] - 2019-05-24
### Added
* `ImageData::set_min_code_size` method
* FrameEncoder (and `Encoder.into_frame_encoder`)
### Changed
* Updated pix dep to 0.6
* Automatically calculate LZW min code size for ImageData blocks.
### Removed
* FrameDecoder::new and RasterDecoder::new no longer public.

## [0.2.0] - 2019-05-01
### Added
* `Decoder::into_raster_decoder` gives iterator of Rgba8 rasters in animation.
* `Preamble::screen_width` / `screen_height` methods
### Fixed
* `GraphicControl::transparent_color` always false
### Changed
* `Decoder::into_iter` now returns a RasterDecoder
* Made default `max_image_size` 33,554,432 bytes (2<sup>25</sup>)

## [0.1.1] - 2019-04-28
### Added
* Preamble now contains Comment blocks
* Logo!
* `Decoder::into_block_decoder`
### Changed
* Fixed assert failure with 256 byte sub-blocks
* Made Header / LogicalScreenDesc not optional in Preamble

## [0.1.0] - 2019-04-25
* Initial version
