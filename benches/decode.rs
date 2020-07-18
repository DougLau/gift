use criterion::{black_box, criterion_group, criterion_main, Criterion};
use gift::Decoder;
use std::io::Cursor;

const LOGO: &[u8] = include_bytes!("../res/gift_logo.gif") as &[u8];

fn decode_blocks(crit: &mut Criterion) {
    crit.bench_function("decode_blocks", |b| {
        b.iter(|| {
            let decoder =
                Decoder::new(Cursor::new(black_box(LOGO))).into_blocks();
            for block in decoder {
                black_box(block.unwrap());
            }
        })
    });
}

fn decode_frames(crit: &mut Criterion) {
    crit.bench_function("decode_frames", |b| {
        b.iter(|| {
            let decoder =
                Decoder::new(Cursor::new(black_box(LOGO))).into_frames();
            for frame in decoder {
                black_box(frame.unwrap());
            }
        })
    });
}

fn decode_steps(crit: &mut Criterion) {
    crit.bench_function("decode_steps", |b| {
        b.iter(|| {
            let decoder = Decoder::new(Cursor::new(black_box(LOGO)));
            for step in decoder {
                black_box(step.unwrap());
            }
        })
    });
}

criterion_group!(benches, decode_blocks, decode_frames, decode_steps);
criterion_main!(benches);
