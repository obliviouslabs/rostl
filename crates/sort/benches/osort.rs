#![allow(clippy::collection_is_never_read)]
#![allow(missing_docs)]
use criterion::{
  criterion_group, criterion_main, measurement::Measurement, AxisScale, BenchmarkId, Criterion,
  PlotConfiguration,
};
use rand::seq::SliceRandom;
#[allow(deprecated)]
use rostl_sort::batcher::batcher_sort;
#[allow(deprecated)]
use rostl_sort::bose_nelson::bose_nelson_sort;

use rostl_sort::bitonic::{bitonic_payload_sort, bitonic_sort};
use rostl_sort::shuffle::shuffle;
use std::hint::black_box;

pub fn benchmark_sort<T: Measurement + 'static>(c: &mut Criterion<T>) {
  let mut group = c.benchmark_group(format!(
    "Sorting/{}",
    std::any::type_name::<T>().split(':').next_back().unwrap()
  ));
  let plot_config = PlotConfiguration::default().summary_scale(AxisScale::Logarithmic);
  group.plot_config(plot_config);

  for &size in &[32, 100, 320, 1_000, 4_096] {
    group.bench_with_input(BenchmarkId::new("Bitonic", size), &size, |b, &size| {
      let mut data: Vec<u64> = (0u64..size as u64).collect();
      data.shuffle(&mut rand::rng());
      let data = data;
      b.iter(|| {
        let mut data_clone = black_box(data.clone());
        bitonic_sort(&mut data_clone);
      });
    });

    group.bench_with_input(BenchmarkId::new("Bitonic_innerpl", size), &size, |b, &size| {
      let mut data: Vec<u128> = (0u128..size as u128).collect();
      data.shuffle(&mut rand::rng());
      let data = data;
      b.iter(|| {
        let mut data_clone = black_box(data.clone());
        bitonic_sort(&mut data_clone);
      });
    });

    group.bench_with_input(BenchmarkId::new("Bitonic_payload_sort", size), &size, |b, &size| {
      let mut data: Vec<u64> = (0u64..size as u64).collect();
      let mut indexes: Vec<u64> = (0u64..size as u64).collect();
      data.shuffle(&mut rand::rng());
      let data = data;
      b.iter(|| {
        let mut data_clone = black_box(data.clone());
        bitonic_payload_sort(&mut data_clone, &mut indexes);
      });
    });

    group.bench_with_input(BenchmarkId::new("Batcher", size), &size, |b, &size| {
      let mut data: Vec<u64> = (0u64..size as u64).collect();
      data.shuffle(&mut rand::rng());
      let data = data;
      b.iter(|| {
        let mut data_clone = black_box(data.clone());
        #[allow(deprecated)]
        batcher_sort(&mut data_clone);
      });
    });

    group.bench_with_input(BenchmarkId::new("BoseNelson", size), &size, |b, &size| {
      let mut data: Vec<u64> = (0u64..size as u64).collect();
      data.shuffle(&mut rand::rng());
      let data = data;
      b.iter(|| {
        let mut data_clone = black_box(data.clone());
        #[allow(deprecated)]
        bose_nelson_sort(&mut data_clone);
      });
    });

    group.bench_with_input(BenchmarkId::new("std::sort", size), &size, |b, &size| {
      let mut data: Vec<u64> = (0u64..size as u64).collect();
      data.shuffle(&mut rand::rng());
      let data = data;
      b.iter(|| {
        let mut data_clone = black_box(data.clone());
        data_clone.sort();
      });
    });

    group.bench_with_input(BenchmarkId::new("Shuffle", size), &size, |b, &size| {
      let mut data: Vec<u64> = (0u64..size as u64).collect();
      b.iter(|| {
        shuffle(&mut data);
      });
    });
  }

  group.finish();
}

criterion_group!(name = benches_time;
  config = Criterion::default().warm_up_time(std::time::Duration::from_millis(3000)).measurement_time(std::time::Duration::from_secs(5));
  targets = benchmark_sort);
criterion_main!(benches_time);
