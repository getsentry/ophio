// you can run this with:
// > DIVAN_MIN_TIME=2 cargo bench -p rust-ophio --all-features
// and then profile with:
// > DIVAN_MIN_TIME=2 samply record target/release/deps/enhancers-XXXX --bench

use std::path::PathBuf;

use divan::{black_box, Bencher};

use rust_ophio::enhancers::{Enhancements, ExceptionData, Frame, LruCache, NoopCache};
use smol_str::SmolStr;

fn main() {
    divan::main();
}

fn read_fixture(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../tests/fixtures")
        .join(name);
    std::fs::read_to_string(path).unwrap()
}

#[divan::bench]
fn parse_enhancers(bencher: Bencher) {
    let enhancers = read_fixture("newstyle@2023-01-11.txt");
    bencher.bench(|| {
        black_box(Enhancements::parse(&enhancers, NoopCache).unwrap());
    })
}

#[divan::bench]
fn parse_enhancers_cached(bencher: Bencher) {
    let enhancers = read_fixture("newstyle@2023-01-11.txt");
    let mut cache = LruCache::new(1_000.try_into().unwrap());
    bencher.bench_local(|| {
        black_box(Enhancements::parse(&enhancers, &mut cache).unwrap());
    })
}

#[divan::bench]
fn apply_modifications(bencher: Bencher) {
    let enhancers = read_fixture("newstyle@2023-01-11.txt");
    let enhancers = Enhancements::parse(&enhancers, NoopCache).unwrap();

    let platform = "cocoa";

    let stacktraces = read_fixture("cocoa-stacktraces.json");
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
