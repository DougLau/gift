use criterion::{Criterion, black_box, criterion_group, criterion_main};
use gift::{Decoder, Encoder, block::Block};
use std::io::Cursor;

const LOGO: &[u8] = include_bytes!("../res/gift_logo.gif") as &[u8];

fn encode_blocks(crit: &mut Criterion) {
    let blocks: Vec<Block> = Decoder::new(Cursor::new(LOGO))
        .into_blocks()
        .map(|b| b.unwrap())
        .collect();
    crit.bench_function("encode_blocks", |b| {
        b.iter(|| {
            let mut encoder =
                Encoder::new(Cursor::new(black_box(Vec::with_capacity(32768))))
                    .into_block_enc();
            for block in &blocks {
                encoder.encode(black_box(block.clone())).unwrap();
            }
        })
    });
}

criterion_group!(benches, encode_blocks);
criterion_main!(benches);
