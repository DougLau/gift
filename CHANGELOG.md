## [Unreleased]

### Added
* Decoder::into_raster_decoder gives iterator of Rgba8 rasters in animation.
* Preamble::screen_width / screen_height methods
### Fixed
* GraphicControl::transparent_color() always false
### Changed
* Decoder::into_iter now returns a RasterDecoder
* Made default max_image_size 33,554,432 bytes (2<sup>25</sup>)

## [0.1.1 - 2019-04-28]
### Added
* Preamble now contains Comment blocks
* Logo!
* Decoder::into_block_decoder()
### Changed
* Fixed assert failure with 256 byte sub-blocks
* Made Header / LogicalScreenDesc not optional in Preamble

## [0.1.0] - 2019-04-25
* Initial version
