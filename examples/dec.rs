use std::env;
use std::error::Error;
use std::fs::File;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::Builder::from_default_env()
        .default_format_timestamp(false)
        .init();
    if let Some(path) = env::args().nth(1) {
        decode(&path)?;
    } else {
        eprintln!("usage: dec [filename]");
    }
    Ok(())
}

fn decode(path: &str) -> Result<(), Box<dyn Error>> {
    let f = File::open(path)?;
    let mut frame_dec = gift::Decoder::new(f).into_frame_decoder();
    let preamble = frame_dec.preamble()?;
    println!("preamble: {:?}", preamble);
    for frame in frame_dec {
        println!("frame: {:?}", frame?);
    }
    Ok(())
}
