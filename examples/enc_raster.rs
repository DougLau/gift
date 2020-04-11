// Raster encoding example
use gift::Encoder;
use pix::{Gray8, Palette, Raster, SRgb8};
use std::error::Error;
use std::fs::File;

fn main() -> Result<(), Box<dyn Error>> {
    let mut f = File::create("enc_raster.gif")?;
    let mut enc = Encoder::new(&mut f).into_raster_enc();
    let mut raster = Raster::with_clear(4, 4);
    *raster.pixel_mut(0, 0) = Gray8::new(1);
    *raster.pixel_mut(1, 1) = Gray8::new(1);
    *raster.pixel_mut(2, 2) = Gray8::new(1);
    *raster.pixel_mut(3, 3) = Gray8::new(1);
    let mut palette = Palette::new(2);
    palette.set_entry(SRgb8::new(0xFF, 0, 0));
    palette.set_entry(SRgb8::new(0xFF, 0xFF, 0));
    enc.encode_indexed_raster(&raster, palette)?;
    Ok(())
}
