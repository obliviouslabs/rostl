#![allow(clippy::collection_is_never_read)]
#![allow(missing_docs)]
use criterion::{
  criterion_group, criterion_main, measurement::Measurement, AxisScale, BenchmarkId, Criterion,
  PlotConfiguration,
};
use rand::seq::SliceRandom;
#[allow(deprecated)]
use rods_sort::batcher::batcher_sort;
#[allow(deprecated)]
use rods_sort::bose_nelson::bose_nelson_sort;

use rods_sort::bitonic::bitonic_sort;
use rods_sort::shuffle::shuffle;
use std::hint::black_box;

pub fn benchmark_sort<T: Measurement + 'static>(c: &mut Criterion<T>) {
  let mut group =
    c.benchmark_group(format!("Sorting/{}", std::any::type_name::<T>().split(':').last().unwrap()));
  let plot_config = PlotConfiguration::default().summary_scale(AxisScale::Logarithmic);
  group.plot_config(plot_config);

  for &size in &[32, 100, 320, 1_000] {
    group.bench_with_input(BenchmarkId::new("Bitonic", size), &size, |b, &size| {
      let mut data: Vec<i32> = (0..size).collect();
      data.shuffle(&mut rand::rng());
      let data = data;
      b.iter(|| {
        let mut data_clone = black_box(data.clone());
        bitonic_sort(&mut data_clone);
      });
    });

    group.bench_with_input(BenchmarkId::new("Batcher", size), &size, |b, &size| {
      let mut data: Vec<i32> = (0..size).collect();
      data.shuffle(&mut rand::rng());
      let data = data;
      b.iter(|| {
        let mut data_clone = black_box(data.clone());
        #[allow(deprecated)]
        batcher_sort(&mut data_clone);
      });
    });

    group.bench_with_input(BenchmarkId::new("BoseNelson", size), &size, |b, &size| {
      let mut data: Vec<i32> = (0..size).collect();
      data.shuffle(&mut rand::rng());
      let data = data;
      b.iter(|| {
        let mut data_clone = black_box(data.clone());
        #[allow(deprecated)]
        bose_nelson_sort(&mut data_clone);
      });
    });

    group.bench_with_input(BenchmarkId::new("std::sort", size), &size, |b, &size| {
      let mut data: Vec<i32> = (0..size).collect();
      data.shuffle(&mut rand::rng());
      let data = data;
      b.iter(|| {
        let mut data_clone = black_box(data.clone());
        data_clone.sort();
      });
    });

    group.bench_with_input(BenchmarkId::new("Shuffle", size), &size, |b, &size| {
      let mut data: Vec<i32> = (0..size).collect();
      b.iter(|| {
        shuffle(&mut data);
      });
    });
  }

  group.finish();
}

criterion_group!(name = benches_time;
  config = Criterion::default().warm_up_time(std::time::Duration::from_millis(500)).measurement_time(std::time::Duration::from_secs(3));
  targets = benchmark_sort);
criterion_main!(benches_time);
