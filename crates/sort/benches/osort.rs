#![allow(clippy::collection_is_never_read)]
#![allow(missing_docs)]
use criterion::{
  criterion_group, criterion_main, measurement::Measurement, AxisScale, BenchmarkId, Criterion,
  PlotConfiguration,
};
use rand::seq::SliceRandom;
#[allow(deprecated)]
use rods_sort::batcher::batcher_sort;
use rods_sort::bitonic::bitonic_sort;
use std::hint::black_box;

pub fn benchmark_sort<T: Measurement + 'static>(c: &mut Criterion<T>) {
  let mut group =
    c.benchmark_group(format!("Sorting/{}", std::any::type_name::<T>().split(':').last().unwrap()));
  let plot_config = PlotConfiguration::default().summary_scale(AxisScale::Logarithmic);
  group.plot_config(plot_config);

  for &size in &[100, 1_000, 3_162, 10_000, 31_623, 100_000] {
    group.bench_with_input(BenchmarkId::new("Bitonic", size), &size, |b, &size| {
      let mut data: Vec<i32> = (0..size).collect();
      data.shuffle(&mut rand::thread_rng());
      let data = data;
      b.iter(|| {
        let mut data_clone = black_box(data.clone());
        bitonic_sort(&mut data_clone);
      });
    });

    group.bench_with_input(BenchmarkId::new("Batcher", size), &size, |b, &size| {
      let mut data: Vec<i32> = (0..size).collect();
      data.shuffle(&mut rand::thread_rng());
      let data = data;
      b.iter(|| {
        let mut data_clone = black_box(data.clone());
        #[allow(deprecated)]
        batcher_sort(&mut data_clone);
      });
    });

    group.bench_with_input(BenchmarkId::new("std::sort", size), &size, |b, &size| {
      let mut data: Vec<i32> = (0..size).collect();
      data.shuffle(&mut rand::thread_rng());
      let data = data;
      b.iter(|| {
        let mut data_clone = black_box(data.clone());
        data_clone.sort();
      });
    });
  }

  group.finish();
}

criterion_group!(name = benches_time;
  config = Criterion::default().warm_up_time(std::time::Duration::from_millis(500)).measurement_time(std::time::Duration::from_secs(3));
  targets = benchmark_sort);
criterion_main!(benches_time);
