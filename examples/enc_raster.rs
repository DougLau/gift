// Raster encoding example
use gift::{Encoder, Step};
use pix::gray::Gray8;
use pix::rgb::SRgb8;
use pix::{Palette, Raster};
use std::error::Error;
use std::fs::File;
use std::io::BufWriter;

fn main() -> Result<(), Box<dyn Error>> {
    let mut f = BufWriter::new(File::create("enc_raster.gif")?);
    let mut enc = Encoder::new(&mut f).into_step_enc();
    let mut raster = Raster::with_clear(4, 4);
    *raster.pixel_mut(0, 0) = Gray8::new(1);
    *raster.pixel_mut(1, 1) = Gray8::new(1);
    *raster.pixel_mut(2, 2) = Gray8::new(1);
    *raster.pixel_mut(3, 3) = Gray8::new(1);
    let mut palette = Palette::new(2);
    palette.set_entry(SRgb8::new(0xFF, 0, 0));
    palette.set_entry(SRgb8::new(0xFF, 0xFF, 0));
    let step = Step::with_indexed(raster, palette);
    enc.encode_step(&step)?;
    Ok(())
}
