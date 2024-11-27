#![allow(missing_docs)]
use criterion::{criterion_group, criterion_main, measurement::Measurement, Criterion};
use criterion_cycles_per_byte::CyclesPerByte;
use rods_primitives::asm::_Cmovbase;
use rods_primitives::traits::Cmov;
use std::hint::black_box;

pub fn benchmark_cmov_u64<T: Measurement + 'static>(c: &mut Criterion<T>) {
  let t_as_str = std::any::type_name::<T>();
  c.bench_function(format!("cmov {}", t_as_str).as_str(), |b| {
    let mut op_a: u64 = 0;
    let op_b: u64 = 0x12345678;
    b.iter(|| {
      black_box(op_a.cmov(black_box(&op_b), black_box(true)));
    })
  });

  c.bench_function(format!("cmov2 {}", t_as_str).as_str(), |b| {
    let mut op_a: u64 = 0;
    let op_b: u64 = 0x12345678;
    b.iter(|| {
      black_box(op_a.cmov_base(black_box(&op_b), black_box(true)));
    })
  });

  c.bench_function(format!("mov {}", t_as_str).as_str(), |b| {
    let mut op_a: u64 = 0;
    let op_b: u64 = 0x12345678;
    b.iter(|| {
      black_box(op_a = black_box(op_b));
    })
  });
}

criterion_group!(benches_time, benchmark_cmov_u64);
criterion_group!(
  name = benches_cycles;
  config = Criterion::default().with_measurement(CyclesPerByte);
  targets = benchmark_cmov_u64<CyclesPerByte>
);
criterion_main!(benches_time, benches_cycles);
