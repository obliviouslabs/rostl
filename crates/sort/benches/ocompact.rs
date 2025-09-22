#![allow(clippy::collection_is_never_read)]
#![allow(missing_docs)]
use criterion::{
  criterion_group, criterion_main, measurement::Measurement, AxisScale, BenchmarkId, Criterion,
  PlotConfiguration,
};
use rand::seq::SliceRandom;

#[allow(deprecated)]
use rostl_sort::compaction::{compact, compact_goodrich};
use std::hint::black_box;

#[allow(deprecated)]
pub fn benchmark_compact<T: Measurement + 'static>(c: &mut Criterion<T>) {
  let mut group = c.benchmark_group(format!(
    "Sorting/{}",
    std::any::type_name::<T>().split(':').next_back().unwrap()
  ));
  let plot_config = PlotConfiguration::default().summary_scale(AxisScale::Logarithmic);
  group.plot_config(plot_config);

  for &size in &[32, 100, 320, 1_000, 4_096] {
    group.bench_with_input(BenchmarkId::new("Compact_goodrich", size), &size, |b, &size| {
      let mut data: Vec<u64> = (0u64..size as u64).collect();
      data.shuffle(&mut rand::rng());
      let data = data;
      b.iter(|| {
        let mut data_clone = black_box(data.clone());
        compact_goodrich(&mut data_clone, |x| *x % 2 == 0);
      });
    });

    group.bench_with_input(BenchmarkId::new("Compact_ffocs", size), &size, |b, &size| {
      let mut data: Vec<u128> = (0u128..size as u128).collect();
      data.shuffle(&mut rand::rng());
      let data = data;
      b.iter(|| {
        let mut data_clone = black_box(data.clone());
        compact(&mut data_clone, |x| *x % 2 == 0);
      });
    });
  }

  group.finish();
}

criterion_group!(name = benches_time;
  config = Criterion::default().warm_up_time(std::time::Duration::from_millis(3000)).measurement_time(std::time::Duration::from_secs(5));
  targets = benchmark_compact);
criterion_main!(benches_time);
