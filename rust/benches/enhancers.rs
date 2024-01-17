use divan::{black_box, Bencher};

use rust_ophio::enhancers::{Enhancements, ExceptionData, Frame};
use smol_str::SmolStr;

fn main() {
    divan::main();
}

#[divan::bench]
fn parse_enhancers(bencher: Bencher) {
    let enhancers = std::fs::read_to_string("../tests/fixtures/newstyle@2023-01-11.txt").unwrap();
    bencher.bench(|| {
        black_box(Enhancements::parse(&enhancers).unwrap());
    })
}

#[divan::bench]
fn apply_modifications(bencher: Bencher) {
    let enhancers = std::fs::read_to_string("../tests/fixtures/newstyle@2023-01-11.txt").unwrap();
    let enhancers = Enhancements::parse(&enhancers).unwrap();

    let platform = "cocoa";

    let stacktraces = std::fs::read_to_string("../tests/fixtures/cocoa-stacktraces.json").unwrap();
    let stacktraces: serde_json::Value = serde_json::from_str(&stacktraces).unwrap();
    let mut stacktraces: Vec<_> = stacktraces
        .as_array()
        .unwrap()
        .iter()
        .map(|frames| {
            frames
                .as_array()
                .unwrap()
                .iter()
                .map(|f| Frame::from_test(f, platform))
                .collect::<Vec<_>>()
        })
        .collect();

    let exception_data = ExceptionData {
        ty: Some(SmolStr::new("App Hanging")),
        value: Some(SmolStr::new("App hanging for at least 2000 ms.")),
        mechanism: Some(SmolStr::new("AppHang")),
    };

    bencher.bench_local(move || {
        for frames in &mut stacktraces {
            enhancers.apply_modifications_to_frames(frames, &exception_data);
        }
    })
}
