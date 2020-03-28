// Raster encoding example
use gift::Encoder;
use pix::{Palette, RasterBuilder, SRgb8};
use std::error::Error;
use std::fs::File;

fn main() -> Result<(), Box<dyn Error>> {
    let mut f = File::create("enc_raster.gif")?;
    let mut enc = Encoder::new(&mut f).into_raster_enc();
    let mut raster = RasterBuilder::new().with_clear(4, 4);
    raster.set_pixel(0, 0, 1u8);
    raster.set_pixel(1, 1, 1u8);
    raster.set_pixel(2, 2, 1u8);
    raster.set_pixel(3, 3, 1u8);
    let mut palette = Palette::new(2);
    palette.set_entry(SRgb8::new(0xFF, 0, 0));
    palette.set_entry(SRgb8::new(0xFF, 0xFF, 0));
    enc.encode_indexed_raster(&raster, palette)?;
    Ok(())
}
