use bencher::{benchmark_group, benchmark_main, black_box, Bencher};
use gift::Decoder;
use std::io::Cursor;

fn decode_logo_frames(bencher: &mut Bencher) {
    let logo = include_bytes!("../res/gift_logo.gif") as &[u8];

    bencher.iter(|| {
        let decoder = Decoder::new(Cursor::new(black_box(logo))).into_frames();
        for frame in decoder {
            black_box(frame.unwrap());
        }
    });
}

benchmark_group!(benches, decode_logo_frames);
benchmark_main!(benches);
