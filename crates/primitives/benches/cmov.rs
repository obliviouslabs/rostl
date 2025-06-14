#![allow(missing_docs)]

#[allow(unused_imports)]
use criterion::{
  criterion_group, criterion_main,
  measurement::{Measurement, WallTime},
  Criterion,
};

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use criterion_cycles_per_byte::CyclesPerByte;
use rostl_primitives::traits::Cmov;
use rostl_primitives::traits::_Cmovbase;
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

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
criterion_group!(
  name = benches_cycles;
  config = Criterion::default().with_measurement(CyclesPerByte).warm_up_time(std::time::Duration::from_millis(500)).measurement_time(std::time::Duration::from_secs(1));
  targets = benchmark_cmov_u64<CyclesPerByte>
);

#[cfg(target_arch = "aarch64")]
criterion_group!(
  name = benches_cycles;
  config = Criterion::default().warm_up_time(std::time::Duration::from_millis(500)).measurement_time(std::time::Duration::from_secs(1));
  targets = benchmark_cmov_u64<WallTime>
);

criterion_main!(benches_cycles);
