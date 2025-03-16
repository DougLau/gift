// cargo fuzz run decode corpus/decode -- -timeout=30

#![no_main]

use std::io::Cursor;
use libfuzzer_sys::fuzz_target;

use gift::Decoder;

fuzz_target!(|data: &[u8]| {
    for frame in Decoder::new(Cursor::new(data)) {
        if frame.is_err() {
            return;
        }
    }
});
