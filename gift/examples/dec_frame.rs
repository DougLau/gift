// Frame decoding example
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;

fn main() -> Result<(), Box<dyn Error>> {
    let path = env::args().nth(1).expect("usage: dec_frame [filename]");
    decode(&path)
}

fn decode(path: &str) -> Result<(), Box<dyn Error>> {
    let f = BufReader::new(File::open(path)?);
    let mut frame_dec = gift::Decoder::new(f).into_frames();
    let preamble = frame_dec.preamble()?;
    println!("preamble: {:?}", preamble);
    for frame in frame_dec {
        println!("frame: {:?}", frame?);
    }
    Ok(())
}
