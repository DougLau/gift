// dots.rs
use gift::{Encoder, Step};
use pix::{
    gray::{Gray, Gray8},
    rgb::{Rgb, SRgb8},
    Palette, Raster,
};
use std::error::Error;
use std::fs::File;
use std::io::BufWriter;

fn page1(p: &mut Palette) -> Raster<Gray8> {
    let amber = SRgb8::new(255, 208, 0);
    let red = SRgb8::new(255, 0, 0);
    let mut r = Raster::with_clear(32, 32);
    render_circle(&mut r, p, 12.0, 12.0, 3.0, amber);
    render_circle(&mut r, p, 20.0, 12.0, 3.0, amber);
    render_circle(&mut r, p, 12.0, 20.0, 3.0, amber);
    render_circle(&mut r, p, 20.0, 20.0, 3.0, amber);
    render_circle(&mut r, p, 16.0, 16.0, 3.5, red);
    r
}

fn page2(p: &mut Palette) -> Raster<Gray8> {
    let amber = SRgb8::new(255, 208, 0);
    let mut r = Raster::with_clear(32, 32);
    render_circle(&mut r, p, 12.0, 12.0, 3.0, amber);
    render_circle(&mut r, p, 20.0, 12.0, 3.0, amber);
    render_circle(&mut r, p, 12.0, 20.0, 3.0, amber);
    render_circle(&mut r, p, 20.0, 20.0, 3.0, amber);
    r
}

fn render_circle(
    raster: &mut Raster<Gray8>,
    palette: &mut Palette,
    cx: f32,
    cy: f32,
    r: f32,
    clr: SRgb8,
) {
    let x0 = (cx - r).floor().max(0.0) as u32;
    let x1 = (cx + r).ceil().min(raster.width() as f32) as u32;
    let y0 = (cy - r).floor().max(0.0) as u32;
    let y1 = (cy + r).ceil().min(raster.height() as f32) as u32;
    let rs = r.powi(2);
    for y in y0..y1 {
        let yd = (cy - y as f32 - 0.5).abs();
        let ys = yd.powi(2);
        for x in x0..x1 {
            let xd = (cx - x as f32 - 0.5).abs();
            let xs = xd.powi(2);
            let mut ds = xs + ys;
            // If center is within this pixel, make it brighter
            if ds < 1.0 {
                ds = ds.powi(2);
            }
            // compare distance squared with radius squared
            let drs = ds / rs;
            let v = 1.0 - drs.powi(2).min(1.0);
            if v > 0.0 {
                // blend with existing pixel
                let i = u8::from(Gray::value(raster.pixel(x as i32, y as i32)));
                if let Some(p) = palette.entry(i as usize) {
                    let red = (Rgb::red(clr) * v).max(Rgb::red(p));
                    let green = (Rgb::green(clr) * v).max(Rgb::green(p));
                    let blue = (Rgb::blue(clr) * v).max(Rgb::blue(p));
                    let rgb = SRgb8::new(red, green, blue);
                    if let Some(d) = palette.set_entry(rgb) {
                        *raster.pixel_mut(x as i32, y as i32) =
                            Gray8::new(d as u8);
                    }
                }
            }
        }
    }
}

/// Get the difference threshold for SRgb8 with 256 capacity palette
fn palette_threshold_rgb8_256(v: usize) -> SRgb8 {
    let i = match v as u8 {
        0x00..=0x0F => 0,
        0x10..=0x1E => 1,
        0x1F..=0x2D => 2,
        0x2E..=0x3B => 3,
        0x3C..=0x49 => 4,
        0x4A..=0x56 => 5,
        0x57..=0x63 => 6,
        0x64..=0x6F => 7,
        0x70..=0x7B => 8,
        0x7C..=0x86 => 9,
        0x87..=0x91 => 10,
        0x92..=0x9B => 11,
        0x9C..=0xA5 => 12,
        0xA6..=0xAE => 13,
        0xAF..=0xB7 => 14,
        0xB8..=0xBF => 15,
        0xC0..=0xC7 => 16,
        0xC8..=0xCE => 17,
        0xCF..=0xD5 => 18,
        0xD6..=0xDB => 19,
        0xDC..=0xE1 => 20,
        0xE2..=0xE6 => 21,
        0xE7..=0xEB => 22,
        0xEC..=0xEF => 23,
        0xF0..=0xF3 => 24,
        0xF4..=0xF6 => 25,
        0xF7..=0xF9 => 26,
        0xFA..=0xFB => 27,
        0xFC..=0xFD => 28,
        0xFE..=0xFE => 29,
        0xFF..=0xFF => 30,
    };
    SRgb8::new(i * 4, i * 4, i * 5)
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut palette = Palette::new(256);
    palette.set_entry(SRgb8::default());
    palette.set_threshold_fn(palette_threshold_rgb8_256);
    let mut fl = BufWriter::new(File::create("dots.gif")?);
    let mut enc = Encoder::new(&mut fl).into_step_enc().with_loop_count(0);
    let raster = page1(&mut palette);
    let step = Step::with_indexed(raster, palette.clone())
        .with_delay_time_cs(Some(200));
    enc.encode_step(&step)?;
    let raster = page2(&mut palette);
    let step = Step::with_indexed(raster, palette.clone())
        .with_delay_time_cs(Some(200));
    enc.encode_step(&step)?;
    Ok(())
}
