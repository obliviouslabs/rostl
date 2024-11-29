#![allow(clippy::collection_is_never_read)]
#![allow(missing_docs)]
use criterion::{criterion_group, criterion_main, measurement::Measurement, Criterion};
use criterion_cycles_per_byte::CyclesPerByte;
use rods_storage::{memstore::MemStore, traits::PageStorage};
use std::hint::black_box;

pub fn benchmark_storage<T: Measurement + 'static>(c: &mut Criterion<T>) {
  c.bench_function("mov-4k", |b| {
    let src: Vec<u8> = vec![0; 4096];
    let mut dst: Vec<u8> = vec![0; 4096];
    b.iter(|| {
      dst.copy_from_slice(black_box(&src));
    })
  });

  c.bench_function("memstore-4k", |b| {
    let src = MemStore::open(String::new(), 2).unwrap();
    let mut dst: Vec<u8> = vec![0; 4096];
    b.iter(|| {
      src.read_page(black_box(0), black_box(&mut dst)).unwrap();
    })
  });

  // NOTE: mov-4k and memstore-4k should have similar performance.
}

criterion_group!(name = benches_cycles;
  config = Criterion::default().with_measurement(CyclesPerByte).warm_up_time(std::time::Duration::from_millis(500)).measurement_time(std::time::Duration::from_secs(1));
  targets = benchmark_storage);
criterion_main!(benches_cycles);
