// Raster decoding example
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;

fn main() -> Result<(), Box<dyn Error>> {
    let path = env::args().nth(1).expect("usage: dec_raster [filename]");
    decode(&path)
}

fn decode(path: &str) -> Result<(), Box<dyn Error>> {
    let gif = BufReader::new(File::open(path)?);
    for step in gift::Decoder::new(gif) {
        let step = step?;
        let raster = step.raster();
        println!("raster: {:?}x{:?}", raster.width(), raster.height());
    }
    Ok(())
}
