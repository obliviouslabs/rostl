#![allow(missing_docs)]
use criterion::{
  black_box, criterion_group, criterion_main, measurement::Measurement, AxisScale, BenchmarkId,
  Criterion, PlotConfiguration,
};

use rand::Rng;
use rods_datastructures::heap::Heap;

pub fn benchmark_heap_initialization<T: Measurement + 'static>(c: &mut Criterion<T>) {
  let mut group = c.benchmark_group(format!(
    "Heap_Initialization/{}",
    std::any::type_name::<T>().split(':').next_back().unwrap()
  ));
  let plot_config = PlotConfiguration::default().summary_scale(AxisScale::Logarithmic);
  group.plot_config(plot_config);

  let test_set = &[16, 64, 256]; // Adjusted sizes for heap

  for &size in test_set {
    group.bench_with_input(BenchmarkId::new("ObliviousHeap", size), &size, |b, &size| {
      b.iter(|| {
        black_box(Heap::<u64>::new(size));
      });
    });
  }
}

pub fn benchmark_heap_ops<T: Measurement + 'static>(c: &mut Criterion<T>) {
  let mut group = c.benchmark_group(format!(
    "Heap_Ops/{}",
    std::any::type_name::<T>().split(':').next_back().unwrap()
  ));
  let plot_config = PlotConfiguration::default().summary_scale(AxisScale::Logarithmic);
  group.plot_config(plot_config);

  let test_set = &[16, 64, 256]; // Adjusted sizes for heap

  for &size in test_set {
    // Benchmark Insert operation
    group.bench_with_input(BenchmarkId::new("Heap_Insert", size), &size, |b, &size| {
      let mut heap = Heap::<u64>::new(size);
      let mut rng = rand::rng();

      // Pre-insert some elements (half capacity)
      for _ in 0..size / 2 {
        let key = rng.random_range(0..usize::MAX);
        let value = rng.random::<u64>();
        heap.insert(key, value);
      }

      b.iter(|| {
        let key = rng.random_range(0..usize::MAX);
        let value = rng.random::<u64>();
        black_box(heap.insert(black_box(key), black_box(value)));
      });
    });

    // Benchmark Find Min operation
    group.bench_with_input(BenchmarkId::new("Heap_FindMin", size), &size, |b, &size| {
      let mut heap = Heap::<u64>::new(size);
      let mut rng = rand::rng();

      // Fill the heap with random values
      for i in 0..size / 2 {
        let key = rng.random_range(0..usize::MAX);
        heap.insert(key, i as u64);
      }

      b.iter(|| {
        black_box(heap.find_min());
      });
    });

    // Benchmark Extract Min operation
    group.bench_with_input(BenchmarkId::new("Heap_ExtractMin", size), &size, |b, &size| {
      b.iter_batched(
        || {
          // Setup: Create a new heap and fill it
          let mut heap = Heap::<u64>::new(size);
          let mut rng = rand::rng();

          for i in 0..size / 2 {
            let key = rng.random_range(0..usize::MAX);
            heap.insert(key, i as u64);
          }
          heap
        },
        |mut heap| {
          heap.extract_min();
          black_box(());
        },
        criterion::BatchSize::SmallInput,
      );
    });

    // Benchmark Delete operation
    group.bench_with_input(BenchmarkId::new("Heap_Delete", size), &size, |b, &size| {
      b.iter_batched(
        || {
          // Setup: Create a new heap, fill it, and keep track of inserted elements
          let mut heap = Heap::<u64>::new(size);
          let mut rng = rand::rng();
          let mut locations = Vec::new();

          for i in 0..size / 2 {
            let key = rng.random_range(0..usize::MAX);
            let location = heap.insert(key, i as u64);
            locations.push(location);
          }

          (heap, locations)
        },
        |(mut heap, locations)| {
          let idx = locations.len() / 2; // Delete an element from the middle
          heap.delete(black_box(locations[idx].0), black_box(locations[idx].1));
        },
        criterion::BatchSize::SmallInput,
      );
    });
  }

  group.finish();
}

criterion_group!(name = benches_time;
    config = Criterion::default()
        .warm_up_time(std::time::Duration::from_millis(500))
        .measurement_time(std::time::Duration::from_secs(3));
    targets = benchmark_heap_initialization, benchmark_heap_ops);
criterion_main!(benches_time);
