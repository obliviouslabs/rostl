#![allow(missing_docs)]
use criterion::{
  black_box, criterion_group, criterion_main, measurement::Measurement, AxisScale, BenchmarkId,
  Criterion, PlotConfiguration,
};

use rand::seq::SliceRandom;
use rostl_datastructures::{
  array::{DynamicArray, FixedArray, LongArray, ShortArray},
  map::UnsortedMap,
  queue::ShortQueue,
  vector::EagerVector,
};

use rostl_primitives::utils::min;
use seq_macro::seq;

pub fn benchmark_array_initialization<T: Measurement + 'static>(c: &mut Criterion<T>) {
  let mut group = c.benchmark_group(format!(
    "Array_Initialization/{}",
    std::any::type_name::<T>().split(':').next_back().unwrap()
  ));
  let plot_config = PlotConfiguration::default().summary_scale(AxisScale::Logarithmic);
  group.plot_config(plot_config);

  const TEST_SET: [usize; 3] = [128, 1 << 10, 1 << 20];

  // UNDONE(git-59): Also test 1<<20
  seq!(SIZE_IDX in 0..2 {{
    const SIZE: usize = TEST_SET[SIZE_IDX];

    if SIZE < (1 << 16) {
      group.bench_with_input(BenchmarkId::new("ShortArray", SIZE), &SIZE, |b, _size| {
        b.iter(|| {
          black_box(Box::new(ShortArray::<u64, SIZE>::new()));
        });
      });

      group.bench_with_input(BenchmarkId::new("ShortQueue", SIZE), &SIZE, |b, _size| {
        b.iter(|| {
          black_box(Box::new(ShortQueue::<u64, SIZE>::new()));
        });
      });

      group.bench_with_input(BenchmarkId::new("FixedArray", SIZE), &SIZE, |b, _size| {
        b.iter(|| {
          let w = Box::new(FixedArray::<u64, SIZE>::new());
          black_box(w);
        });
      });
    }
  }});

  seq!(SIZE_IDX in 0..3 {{
    const SIZE: usize = TEST_SET[SIZE_IDX];

    group.bench_with_input(BenchmarkId::new("LongArray", SIZE), &SIZE, |b, _size| {
      b.iter(|| {
        black_box(Box::new(LongArray::<u64, SIZE>::new()));
      });
    });

    group.bench_with_input(BenchmarkId::new("DynamicArray", SIZE), &SIZE, |b, _size| {
      b.iter(|| {
        black_box(Box::new(DynamicArray::<u64>::new(black_box(SIZE))));
      });
    });

    group.bench_with_input(BenchmarkId::new("UnsortedMap", SIZE), &SIZE, |b, _size| {
      b.iter(|| {
        black_box(Box::new(UnsortedMap::<usize, u64>::new(SIZE)));
      });
    });
  }});

  group.bench_function("EagerVector", |b| {
    b.iter(|| {
      black_box(Box::new(EagerVector::<u64>::new()));
    });
  });
}

#[allow(clippy::redundant_clone)]
pub fn benchmark_array_ops<T: Measurement + 'static>(c: &mut Criterion<T>) {
  let mut group = c.benchmark_group(format!(
    "Array_Operations/{}",
    std::any::type_name::<T>().split(':').next_back().unwrap()
  ));
  let plot_config = PlotConfiguration::default().summary_scale(AxisScale::Logarithmic);
  group.plot_config(plot_config);

  const TEST_SET: [usize; 3] = [128, 1 << 10, 1 << 20];

  // UNDONE(git-59): Also test 1<<20
  seq!(SIZE_IDX in 0..2 {{
    const SIZE: usize = TEST_SET[SIZE_IDX];

    if SIZE < 1 << 20  {
      group.bench_with_input(BenchmarkId::new("ShortArray", SIZE), &SIZE, |b, _size| {
        let mut array = Box::new(ShortArray::<u64, SIZE>::new());
        let mut pattern = Box::new((0..SIZE).map(|x| x as u64).collect::<Vec<_>>());
        pattern.extend_from_slice(&pattern.clone());
        pattern.extend_from_slice(&pattern.clone());
        pattern.shuffle(&mut rand::rng());

        let mut cnt = 0;
        b.iter(|| {
          let idx = pattern[cnt] as usize;
          cnt = (cnt + 1) % pattern.len();
          array.write(black_box(idx), black_box( idx as u64));
        });
      });

      group.bench_with_input(BenchmarkId::new("ShortQueue_pushpop", SIZE), &SIZE, |b, _size| {
        let mut queue = Box::new(ShortQueue::<u64, SIZE>::new());

        b.iter(|| {
          let val = 123;
          queue.maybe_push(black_box(true), black_box(val));
          let mut ret = 0;
          queue.maybe_pop(black_box(true), black_box(&mut ret));
        });
      });
    }
  }});

  seq!(SIZE_IDX in 0..2 {{
    const SIZE: usize = TEST_SET[SIZE_IDX];

    group.bench_with_input(BenchmarkId::new("LongArray_Read", SIZE), &SIZE, |b, _size| {
      let mut array = Box::new(LongArray::<u64, SIZE>::new());
      let mut pattern = Box::new((0..SIZE).map(|x| x as u64).collect::<Vec<_>>());
      pattern.extend_from_slice(&pattern.clone());
      pattern.extend_from_slice(&pattern.clone());
      pattern.shuffle(&mut rand::rng());

      let mut cnt = 0;
      b.iter(|| {
        let mut ret = 0;
        let idx = pattern[cnt] as usize;
        cnt = (cnt + 1) % pattern.len();
        array.read(black_box(idx), black_box(&mut ret));
      });
    });

    group.bench_with_input(BenchmarkId::new("LongArray_Write", SIZE), &SIZE, |b, _size| {
      let mut array = Box::new(LongArray::<u64, SIZE>::new());
      let mut pattern = Box::new((0..SIZE).map(|x| x as u64).collect::<Vec<_>>());
      pattern.extend_from_slice(&pattern.clone());
      pattern.extend_from_slice(&pattern.clone());
      pattern.shuffle(&mut rand::rng());

      let mut cnt = 0;
      b.iter(|| {
        let ret = pattern[cnt];
        let idx = pattern[cnt] as usize;
        cnt = (cnt + 1) % pattern.len();
        array.write(black_box(idx), black_box(ret));
      });
    });

    group.bench_with_input(BenchmarkId::new("FixedArray_Read", SIZE), &SIZE, |b, _size| {
      let mut array = Box::new(FixedArray::<u64, SIZE>::new());
      let mut pattern = Box::new((0..SIZE).map(|x| x as u64).collect::<Vec<_>>());
      pattern.extend_from_slice(&pattern.clone());
      pattern.extend_from_slice(&pattern.clone());
      pattern.shuffle(&mut rand::rng());

      let mut cnt = 0;
      b.iter(|| {
        let mut ret = 0;
        let idx = pattern[cnt] as usize;
        cnt = (cnt + 1) % pattern.len();
        array.read(black_box(idx), black_box(&mut ret));
      });
    });

    group.bench_with_input(BenchmarkId::new("DynamicArray_Read", SIZE), &SIZE, |b, _size| {
      let mut array = Box::new(DynamicArray::<u64>::new(SIZE));
      let mut pattern = Box::new((0..SIZE).map(|x| x as u64).collect::<Vec<_>>());
      pattern.extend_from_slice(&pattern.clone());
      pattern.extend_from_slice(&pattern.clone());
      pattern.shuffle(&mut rand::rng());

      let mut cnt = 0;
      b.iter(|| {
        let mut ret = 0;
        let idx = pattern[cnt] as usize;
        cnt = (cnt + 1) % pattern.len();
        array.read(black_box(idx), black_box(&mut ret));
      });
    });

    group.bench_with_input(BenchmarkId::new("EagerVector_Read", SIZE), &SIZE, |b, _size| {
      const TOP: usize = min(SIZE, 1024);

      let mut vector = Box::new(EagerVector::<u64>::new());
      let mut pattern = Box::new((0..TOP).map(|x| x as u64).collect::<Vec<_>>());
      pattern.extend_from_slice(&pattern.clone());
      pattern.extend_from_slice(&pattern.clone());
      pattern.shuffle(&mut rand::rng());
      for i in 0..TOP {
        vector.push_back(i as u64);
      }
      let mut cnt = 0;
      b.iter(|| {
        let mut ret = 0;
        let idx = pattern[cnt] as usize;
        cnt = (cnt + 1) % pattern.len();
        vector.read(black_box(idx), black_box(&mut ret));
      });
    });

    group.bench_with_input(BenchmarkId::new("UnsortedMap_Read", SIZE), &SIZE, |b, _size| {
      const TOP: usize = min(SIZE, 1024);

      let mut map = Box::new(UnsortedMap::<usize, u64>::new(SIZE));
      let mut pattern = Box::new((0..TOP).map(|x| x as u64).collect::<Vec<_>>());
      pattern.extend_from_slice(&pattern.clone());
      pattern.extend_from_slice(&pattern.clone());
      pattern.shuffle(&mut rand::rng());
      for i in 0..TOP {
        map.insert(i, i as u64);
      }
      let mut cnt = 0;
      b.iter(|| {
        let mut ret = 0;
        let idx = pattern[cnt] as usize;
        cnt = (cnt + 1) % pattern.len();
        map.get(black_box(idx), black_box(&mut ret));
      });
    });
  }});
}

criterion_group!(name = benches_time;
    config = Criterion::default().warm_up_time(std::time::Duration::from_millis(500)).measurement_time(std::time::Duration::from_secs(3));
    targets = benchmark_array_initialization, benchmark_array_ops);
criterion_main!(benches_time);
