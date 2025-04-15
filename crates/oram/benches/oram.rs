#![allow(missing_docs)]
use criterion::{
  black_box, criterion_group, criterion_main, measurement::Measurement, AxisScale, BenchmarkId,
  Criterion, PlotConfiguration,
};

use rods_oram::{
  circuit_oram::CircuitORAM, linear_oram::LinearORAM, recursive_oram::RecursivePositionMap,
};

pub fn benchmark_oram_initialization<T: Measurement + 'static>(c: &mut Criterion<T>) {
  let mut group = c.benchmark_group(format!(
    "ORAM_Initialization/{}",
    std::any::type_name::<T>().split(':').next_back().unwrap()
  ));
  let plot_config = PlotConfiguration::default().summary_scale(AxisScale::Logarithmic);
  group.plot_config(plot_config);

  let test_set = &[128, 1 << 10, 1 << 20];

  for &size in test_set {
    group.bench_with_input(BenchmarkId::new("LinearORAM", size), &size, |b, &size| {
      b.iter(|| {
        black_box(LinearORAM::<u64>::new(size));
      });
    });
    group.bench_with_input(BenchmarkId::new("CircuitORAM", size), &size, |b, &size| {
      b.iter(|| {
        black_box(CircuitORAM::<u64>::new(size));
      });
    });
    group.bench_with_input(BenchmarkId::new("RecursivePositionMap", size), &size, |b, &size| {
      b.iter(|| {
        black_box(RecursivePositionMap::new(size));
      });
    });
  }
}

pub fn benchmark_oram_ops<T: Measurement + 'static>(c: &mut Criterion<T>) {
  let mut group = c.benchmark_group(format!(
    "ORAM_Ops/{}",
    std::any::type_name::<T>().split(':').next_back().unwrap()
  ));
  let plot_config = PlotConfiguration::default().summary_scale(AxisScale::Logarithmic);
  group.plot_config(plot_config);

  let test_set = &[128, 1 << 10, 1 << 20];

  for &size in test_set {
    group.bench_with_input(BenchmarkId::new("LinearORAM_Read", size), &size, |b, &size| {
      let mut oram = LinearORAM::<u64>::new(size);
      oram.write(0, 0);
      b.iter(|| {
        let mut _ign = black_box(0);
        oram.read(black_box(0), black_box(&mut _ign));
      });
    });
    group.bench_with_input(BenchmarkId::new("CircuitORAM_Read", size), &size, |b, &size| {
      let mut oram = CircuitORAM::<u64>::new(size);
      oram.write_or_insert(0, 0, 0, 0);
      b.iter(|| {
        let mut _ign = black_box(0);
        oram.read(black_box(0), black_box(0), black_box(0), &mut _ign);
      });
    });
    group.bench_with_input(BenchmarkId::new("CircuitORAM_Write", size), &size, |b, &size| {
      let mut oram = CircuitORAM::<u64>::new(size);
      oram.write_or_insert(0, 0, 0, 0);
      b.iter(|| {
        oram.write(black_box(0), black_box(0), black_box(0), black_box(0));
      });
    });
    group.bench_with_input(BenchmarkId::new("RecursivePositionMap", size), &size, |b, &size| {
      let mut oram = RecursivePositionMap::new(size);
      b.iter(|| {
        oram.access_position(black_box(0), black_box(0));
      });
    });
  }

  group.finish();
}

criterion_group!(name = benches_time;
    config = Criterion::default().warm_up_time(std::time::Duration::from_millis(500)).measurement_time(std::time::Duration::from_secs(3));
    targets = benchmark_oram_initialization, benchmark_oram_ops);
criterion_main!(benches_time);
