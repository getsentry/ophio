// you can run this with:
// > DIVAN_MIN_TIME=2 cargo bench -p rust-ophio --all-features
// and then profile with:
// > DIVAN_MIN_TIME=2 samply record target/release/deps/enhancers-XXXX --bench

use std::path::PathBuf;

use divan::{black_box, Bencher};

use rust_ophio::enhancers::{Cache, Enhancements, ExceptionData, Frame};
use smol_str::SmolStr;

fn main() {
    divan::main();
}

fn read_fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../tests/fixtures")
        .join(name);
    std::fs::read(path).unwrap()
}

#[divan::bench]
fn parse_enhancers(bencher: Bencher) {
    let enhancers = String::from_utf8(read_fixture("newstyle@2023-01-11.txt")).unwrap();
    bencher.bench(|| {
        black_box(Enhancements::parse(&enhancers, &mut Cache::default()).unwrap());
    })
}

#[divan::bench]
fn parse_enhancers_cached(bencher: Bencher) {
    let enhancers = String::from_utf8(read_fixture("newstyle@2023-01-11.txt")).unwrap();
    let mut cache = Cache::new(1_000);
    bencher.bench_local(|| {
        black_box(Enhancements::parse(&enhancers, &mut cache).unwrap());
    })
}

#[divan::bench]
fn parse_encoded_enhancers(bencher: Bencher) {
    let enhancers = read_fixture("newstyle@2023-01-11.bin");
    bencher.bench(|| {
        black_box(Enhancements::from_config_structure(&enhancers, &mut Cache::default()).unwrap());
    })
}

#[divan::bench]
fn parse_encoded_enhancers_cached(bencher: Bencher) {
    let enhancers = read_fixture("newstyle@2023-01-11.bin");
    let mut cache = Cache::new(1_000);
    bencher.bench_local(|| {
        black_box(Enhancements::from_config_structure(&enhancers, &mut cache).unwrap());
    })
}

#[divan::bench]
fn apply_modifications(bencher: Bencher) {
    let enhancers = String::from_utf8(read_fixture("newstyle@2023-01-11.txt")).unwrap();
    let enhancers = Enhancements::parse(&enhancers, &mut Cache::default()).unwrap();

    let platform = "cocoa";

    let stacktraces = read_fixture("cocoa-stacktraces.json");
    let stacktraces: serde_json::Value = serde_json::from_slice(&stacktraces).unwrap();
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
