use criterion::{black_box, criterion_group, criterion_main, Criterion};
use gift::Decoder;
use std::io::Cursor;

fn decode_logo_frames(crit: &mut Criterion) {
    let logo = include_bytes!("../res/gift_logo.gif") as &[u8];

    crit.bench_function("decode_frames", |b| {
        b.iter(|| {
            let decoder =
                Decoder::new(Cursor::new(black_box(logo))).into_frames();
            for frame in decoder {
                black_box(frame.unwrap());
            }
        })
    });
}

criterion_group!(benches, decode_logo_frames);
criterion_main!(benches);
