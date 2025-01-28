#![allow(clippy::collection_is_never_read)]
#![allow(missing_docs)]
use criterion::{criterion_group, criterion_main, measurement::Measurement, Criterion};
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
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
      black_box(src.read_page(black_box(0), black_box(&mut dst))).unwrap();
    })
  });

  // c.bench_function("mov-40k", |b| {
  //   let src: Vec<u8> = vec![0; 4096 * 10];
  //   let mut dst: Vec<u8> = vec![0; 4096 * 10];
  //   b.iter(|| {
  //     dst.copy_from_slice(black_box(&src));
  //   })
  // });

  // c.bench_function("memstore-40k", |b| {
  //   let src = MemStore::open(String::new(), 10).unwrap();
  //   let mut dst: Vec<u8> = vec![0; 4096];
  //   b.iter(|| {
  //     black_box(src.read_page(black_box(0), black_box(&mut dst))).unwrap();
  //     black_box(src.read_page(black_box(1), black_box(&mut dst))).unwrap();
  //     black_box(src.read_page(black_box(2), black_box(&mut dst))).unwrap();
  //     black_box(src.read_page(black_box(3), black_box(&mut dst))).unwrap();
  //     black_box(src.read_page(black_box(4), black_box(&mut dst))).unwrap();
  //     black_box(src.read_page(black_box(5), black_box(&mut dst))).unwrap();
  //     black_box(src.read_page(black_box(6), black_box(&mut dst))).unwrap();
  //     black_box(src.read_page(black_box(7), black_box(&mut dst))).unwrap();
  //     black_box(src.read_page(black_box(8), black_box(&mut dst))).unwrap();
  //     black_box(src.read_page(black_box(9), black_box(&mut dst))).unwrap();
  //   })
  // });

  // NOTE: mov-4k and memstore-4k should have similar performance.
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
criterion_group!(name = benches_cycles;
  config = Criterion::default().with_measurement(CyclesPerByte).warm_up_time(std::time::Duration::from_millis(500)).measurement_time(std::time::Duration::from_secs(1));
  targets = benchmark_storage);
  
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
criterion_main!(benches_cycles);
