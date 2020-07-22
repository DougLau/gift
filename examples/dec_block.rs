// Block decoding example
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;

fn main() -> Result<(), Box<dyn Error>> {
    let path = env::args().nth(1).expect("usage: dec_block [filename]");
    decode(&path)
}

fn decode(path: &str) -> Result<(), Box<dyn Error>> {
    let f = BufReader::new(File::open(path)?);
    let block_dec = gift::Decoder::new(f).into_blocks();
    for block in block_dec {
        println!("block: {:?}", block?);
    }
    Ok(())
}
