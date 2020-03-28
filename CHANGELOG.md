## [Unreleased]

## [0.5.0] - 2020-03-28
### Changed
* Updated pix dependency to 0.10
* Implemented some micro-optimizations

## [0.4.0] - 2020-02-26
### Added
* Encoder::into_block_enc()
* Encoder::into_frame_enc()
* Encoder::into_raster_enc()
### Changed
* BlockDecoder to decoder::Blocks
* FrameDecoder to decoder::Frames
* RasterDecoder to decoder::Rasters
* Decoder::into_block_decoder() to into_blocks()
* Decoder::into_frame_decoder() to into_frames()
* Decoder::into_raster_decoder() to into_rasters()
* FrameEncoder to encoder::FrameEnc

## [0.3.1] - 2019-05-28
### Changed
* Fixed u8 overflow in ImageData.add_data method.

## [0.3.0] - 2019-05-24
### Added
* ImageData::set_min_code_size method
* FrameEncoder (and Encoder.into_frame_encoder)
### Changed
* Updated pix dep to 0.6
* Automatically calculate LZW min code size for ImageData blocks.
### Removed
* FrameDecoder::new and RasterDecoder::new no longer public.

## [0.2.0] - 2019-05-01
### Added
* Decoder::into_raster_decoder gives iterator of Rgba8 rasters in animation.
* Preamble::screen_width / screen_height methods
### Fixed
* GraphicControl::transparent_color() always false
### Changed
* Decoder::into_iter now returns a RasterDecoder
* Made default max_image_size 33,554,432 bytes (2<sup>25</sup>)

## [0.1.1] - 2019-04-28
### Added
* Preamble now contains Comment blocks
* Logo!
* Decoder::into_block_decoder()
### Changed
* Fixed assert failure with 256 byte sub-blocks
* Made Header / LogicalScreenDesc not optional in Preamble

## [0.1.0] - 2019-04-25
* Initial version
