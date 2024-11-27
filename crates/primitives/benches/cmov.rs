#![allow(missing_docs)]
use criterion::{criterion_group, criterion_main, measurement::Measurement, Criterion};
use criterion_cycles_per_byte::CyclesPerByte;
use rods_primitives::asm::_Cmovbase;
use rods_primitives::traits::Cmov;
use std::hint::black_box;

pub fn benchmark_cmov_u64<T: Measurement + 'static>(c: &mut Criterion<T>) {
  c.bench_function("cmov", |b| {
    let mut op_a: u64 = 0;
    let op_b: u64 = 0x12345678;
    b.iter(|| {
      for _ in 0..1000 {
        op_a.cmov(black_box(&op_b), black_box(true));
      }
    })
  });

  c.bench_function("cmov2", |b| {
    let mut op_a: u64 = 0;
    let op_b: u64 = 0x12345678;
    b.iter(|| {
      for _ in 0..1000 {
        op_a.cmov_base(black_box(&op_b), black_box(true));
      }
    })
  });

  c.bench_function("mov", |b| {
    let mut op_a: u64 = 0;
    let op_b: u64 = 0x12345678;
    b.iter(|| {
      for _ in 0..1000 {
        op_a = black_box(op_b);
      }
    })
  });
}

criterion_group!(
  name = benches_cycles;
  config = Criterion::default().with_measurement(CyclesPerByte).warm_up_time(std::time::Duration::from_millis(500)).measurement_time(std::time::Duration::from_secs(1));
  targets = benchmark_cmov_u64<CyclesPerByte>
);
criterion_main!(benches_cycles);
