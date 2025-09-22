#![allow(missing_docs)]

#[allow(unused_imports)]
use criterion::{
  criterion_group, criterion_main,
  measurement::{Measurement, WallTime},
  Criterion,
};

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use criterion_cycles_per_byte::CyclesPerByte;
#[allow(deprecated)]
use rostl_primitives::utils::{
  get_strictly_bigger_power_of_two, get_strictly_bigger_power_of_two_clz,
};
use std::hint::black_box;

#[allow(deprecated)]
pub fn benchmark_next_power_of_two<T: Measurement + 'static>(c: &mut Criterion<T>) {
  c.bench_function("np2_bitmagic", |b| {
    let mut op_a: usize = 0;
    let op_b: usize = 0x8000;
    b.iter(|| {
      for _ in 0..1000 {
        op_a = get_strictly_bigger_power_of_two(black_box(op_b));
      }
    })
  });

  c.bench_function("np2_clz", |b| {
    let mut op_a: usize = 0;
    let op_b: usize = 0x8000;
    b.iter(|| {
      for _ in 0..1000 {
        op_a = get_strictly_bigger_power_of_two_clz(black_box(op_b));
      }
    })
  });
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
criterion_group!(
  name = benches_cycles;
  config = Criterion::default().with_measurement(CyclesPerByte).warm_up_time(std::time::Duration::from_millis(500)).measurement_time(std::time::Duration::from_secs(1));
  targets = benchmark_next_power_of_two<CyclesPerByte>
);

#[cfg(target_arch = "aarch64")]
criterion_group!(
  name = benches_cycles;
  config = Criterion::default().warm_up_time(std::time::Duration::from_millis(500)).measurement_time(std::time::Duration::from_secs(1));
  targets = benchmark_next_power_of_two<WallTime>
);

criterion_main!(benches_cycles);
