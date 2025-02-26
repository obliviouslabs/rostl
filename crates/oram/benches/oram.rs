#![allow(clippy::collection_is_never_read)]
#![allow(missing_docs)]
use criterion::{
  criterion_group, criterion_main, measurement::Measurement, AxisScale, BenchmarkId, Criterion,
  PlotConfiguration,
};
use rand::Rng;

use rods_oram::linear_oram::LinearORAM;

pub fn benchmark_linear_oram<T: Measurement + 'static>(c: &mut Criterion<T>) {
  let mut group = c.benchmark_group(format!(
    "LinearORAM/{}",
    std::any::type_name::<T>().split(':').last().unwrap()
  ));
  let plot_config = PlotConfiguration::default().summary_scale(AxisScale::Logarithmic);
  group.plot_config(plot_config);

  let test_set = &[100, 1_000, 1_000_000];

  for &size in test_set {
    // UNDONE(git-4): This isn't benchmarking correctly, it's just testing the overhead of the RNG + constructor
    group.bench_with_input(BenchmarkId::new("Read", size), &size, |b, &size| {
      b.iter(|| {
        let mut rng = rand::rng();
        let index: usize = rng.random_range(1..=size);
        let oram = LinearORAM::<u32>::new(25);
        let mut ret = 0;
        oram.read(index, &mut ret);
      });
    });
  }

  for &size in test_set {
    // UNDONE(git-4): This isn't benchmarking correctly, it's just testing the overhead of the RNG + constructor
    group.bench_with_input(BenchmarkId::new("Write", size), &size, |b, &size| {
      let new_value = 3;
      b.iter(|| {
        let mut rng = rand::rng();
        let index: usize = rng.random_range(1..=size);
        let mut oram = LinearORAM::<u32>::new(size);
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
