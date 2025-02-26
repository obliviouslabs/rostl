#![allow(missing_docs)]
use criterion::{
  black_box, criterion_group, criterion_main, AxisScale, BenchmarkId, Criterion, PlotConfiguration,
};
use rods_shuffling::basic_shuffle::basic_shuffle;

pub fn benchmark_basic_shuffle(c: &mut Criterion) {
  let mut group = c.benchmark_group("BasicShuffle");
  let plot_config = PlotConfiguration::default().summary_scale(AxisScale::Logarithmic);
  group.plot_config(plot_config);

  for &size in &[32, 100, 320, 1_000, 3_200, 10_000] {
    group.bench_with_input(BenchmarkId::new("BasicShuffle", size), &size, |b, &size| {
      let data: Vec<i32> = (0..size).collect();
      b.iter(|| {
        let mut data_clone = black_box(data.clone());
        basic_shuffle(&mut data_clone);
      });
    });
  }

  group.finish();
}

criterion_group!(benches_time, benchmark_basic_shuffle);
criterion_main!(benches_time);
