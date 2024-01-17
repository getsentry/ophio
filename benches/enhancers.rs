use divan::{black_box, Bencher};

use sentry_ophio::enhancers::Enhancements;

fn main() {
    divan::main();
}

#[divan::bench]
fn parse_enhancers(bencher: Bencher) {
    let enhancers = std::fs::read_to_string("tests/fixtures/newstyle@2023-01-11.txt").unwrap();
    bencher.bench(|| {
        black_box(Enhancements::parse(&enhancers).unwrap());
    })
}
