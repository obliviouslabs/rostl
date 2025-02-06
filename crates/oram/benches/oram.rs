#![allow(clippy::collection_is_never_read)]
#![allow(missing_docs)]
use criterion::{
  criterion_group, criterion_main, measurement::Measurement, AxisScale, BenchmarkId, Criterion,
  PlotConfiguration,
};
use rand::seq::SliceRandom;
use rand::Rng;

use rods_oram::linear_oram::LinearOram;
use std::hint::black_box;

pub fn benchmark_linear_oram<T: Measurement + 'static>(c: &mut Criterion<T>) {
  let mut group = c.benchmark_group(format!(
    "LinearORAM/{}",
    std::any::type_name::<T>().split(':').last().unwrap()
  ));
  let plot_config = PlotConfiguration::default().summary_scale(AxisScale::Logarithmic);
  group.plot_config(plot_config);

  let test_set = &[100, 1_000, 1_000_000];

  for &size in test_set {
    group.bench_with_input(BenchmarkId::new("Read", size), &size, |b, &size| {
      let default_value = 25;
      let mut data = vec![default_value; size];
      data.shuffle(&mut rand::thread_rng());
      let data = data;
      b.iter(|| {
        let data_clone = black_box(data.clone());
        let mut rng = rand::thread_rng();
        let index: usize = rng.gen_range(1..=size);
        let oram = LinearOram::<Vec<u32>, u32>::new(data_clone);
        let mut ret = 0;
        oram.read(index, &mut ret);
      });
    });
  }

  for &size in test_set {
    group.bench_with_input(BenchmarkId::new("Write", size), &size, |b, &size| {
      let default_value = 25;
      let new_value = 3;
      let mut data = vec![default_value; size];
      data.shuffle(&mut rand::thread_rng());
      let data = data;
      b.iter(|| {
        let data_clone = black_box(data.clone());
        let mut rng = rand::thread_rng();
        let index: usize = rng.gen_range(1..=size);
        let mut oram = LinearOram::<Vec<u32>, u32>::new(data_clone);
        oram.write(index, new_value);
      });
    });
  }

  group.finish();
}

criterion_group!(name = benches_time;
    config = Criterion::default().warm_up_time(std::time::Duration::from_millis(500)).measurement_time(std::time::Duration::from_secs(3));
    targets = benchmark_linear_oram);
criterion_main!(benches_time);
