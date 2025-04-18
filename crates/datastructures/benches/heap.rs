#![allow(missing_docs)]
use criterion::{
    black_box, criterion_group, criterion_main, measurement::Measurement, AxisScale, BenchmarkId,
    Criterion, PlotConfiguration,
};

use rand::{rngs::ThreadRng, seq::SliceRandom, Rng};
use rods_oram::prelude::K;
use bytemuck::Pod;
// Import your heap implementation
use rods_datastructures::heap::Heap;

pub fn benchmark_heap_initialization<T: Measurement + 'static>(c: &mut Criterion<T>) {
    let mut group = c.benchmark_group(format!(
        "Heap_Initialization/{}",
        std::any::type_name::<T>().split(':').next_back().unwrap()
    ));
    let plot_config = PlotConfiguration::default().summary_scale(AxisScale::Logarithmic);
    group.plot_config(plot_config);

    // Test with different heap sizes
    const TEST_SET: [usize; 3] = [16, 64, 256]; // Adjust sizes as needed for your implementation

    for &size in &TEST_SET {
        group.bench_with_input(BenchmarkId::new("ObliviousHeap", size), &size, |b, &size| {
            b.iter(|| {
                black_box(Box::new(Heap::<u64>::new(size)));
            });
        });
    }
}

pub fn benchmark_heap_insert<T: Measurement + 'static>(c: &mut Criterion<T>) {
    let mut group = c.benchmark_group(format!(
        "Heap_Insert/{}",
        std::any::type_name::<T>().split(':').next_back().unwrap()
    ));
    let plot_config = PlotConfiguration::default().summary_scale(AxisScale::Logarithmic);
    group.plot_config(plot_config);

    const TEST_SET: [usize; 3] = [16, 64, 256]; // Adjust sizes as needed

    for &size in &TEST_SET {
        group.bench_with_input(BenchmarkId::new("ObliviousHeap", size), &size, |b, &size| {
            let mut heap = Heap::<u64>::new(size);
            let mut rng = rand::thread_rng();
            
            // Pre-insert some elements (half capacity)
            for _ in 0..size/2 {
                let key = rng.random_range(0..usize::MAX);
                let value = rng.gen::<u64>();
                heap.insert(key, value);
            }
            
            b.iter(|| {
                let key = rng.random_range(0..usize::MAX);
                let value = rng.gen::<u64>();
                black_box(heap.insert(black_box(key), black_box(value)));
            });
        });
    }
}

pub fn benchmark_heap_find_min<T: Measurement + 'static>(c: &mut Criterion<T>) {
    let mut group = c.benchmark_group(format!(
        "Heap_FindMin/{}",
        std::any::type_name::<T>().split(':').next_back().unwrap()
    ));
    let plot_config = PlotConfiguration::default().summary_scale(AxisScale::Logarithmic);
    group.plot_config(plot_config);

    const TEST_SET: [usize; 3] = [16, 64, 256]; // Adjust sizes as needed

    for &size in &TEST_SET {
        group.bench_with_input(BenchmarkId::new("ObliviousHeap", size), &size, |b, &size| {
            let mut heap = Heap::<u64>::new(size);
            let mut rng = rand::thread_rng();
            
            // Fill the heap with random values
            for i in 0..size/2 {
                let key = rng.random_range(0..usize::MAX);
                heap.insert(key, i as u64);
            }
            
            b.iter(|| {
                black_box(heap.find_min());
            });
        });
    }
}

pub fn benchmark_heap_extract_min<T: Measurement + 'static>(c: &mut Criterion<T>) {
    let mut group = c.benchmark_group(format!(
        "Heap_ExtractMin/{}",
        std::any::type_name::<T>().split(':').next_back().unwrap()
    ));
    let plot_config = PlotConfiguration::default().summary_scale(AxisScale::Logarithmic);
    group.plot_config(plot_config);

    const TEST_SET: [usize; 3] = [16, 64, 256]; // Adjust sizes as needed

    for &size in &TEST_SET {
        group.bench_with_input(BenchmarkId::new("ObliviousHeap", size), &size, |b, &size| {
            b.iter_batched(
                || {
                    // Setup: Create a new heap and fill it
                    let mut heap = Heap::<u64>::new(size);
                    let mut rng = rand::thread_rng();
                    
                    for i in 0..size/2 {
                        let key = rng.random_range(0..usize::MAX);
                        heap.insert(key, i as u64);
                    }
                    heap
                },
                |mut heap| {
                    black_box(heap.extract_min());
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }
}

pub fn benchmark_heap_delete<T: Measurement + 'static>(c: &mut Criterion<T>) {
    let mut group = c.benchmark_group(format!(
        "Heap_Delete/{}",
        std::any::type_name::<T>().split(':').next_back().unwrap()
    ));
    let plot_config = PlotConfiguration::default().summary_scale(AxisScale::Logarithmic);
    group.plot_config(plot_config);

    const TEST_SET: [usize; 3] = [16, 64, 256]; // Adjust sizes as needed

    for &size in &TEST_SET {
        group.bench_with_input(BenchmarkId::new("ObliviousHeap", size), &size, |b, &size| {
            b.iter_batched(
                || {
                    // Setup: Create a new heap, fill it, and keep track of inserted elements
                    let mut heap = Heap::<u64>::new(size);
                    let mut rng = rand::thread_rng();
                    let mut positions = Vec::new();
                    let mut oram_keys = Vec::new();
                    
                    for i in 0..size/2 {
                        let key = rng.random_range(0..usize::MAX);
                        let pos = heap.insert(key, i as u64);
                        let (_, oram_key, _, _) = heap.find_min();
                        positions.push(pos);
                        oram_keys.push(oram_key);
                    }
                    
                    (heap, positions, oram_keys)
                },
                |(mut heap, positions, oram_keys)| {
                    let idx = positions.len() / 2; // Delete an element from the middle
                    black_box(heap.delete(black_box(positions[idx]), black_box(oram_keys[idx])));
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }
}

pub fn benchmark_heap_operations_mixed<T: Measurement + 'static>(c: &mut Criterion<T>) {
    let mut group = c.benchmark_group(format!(
        "Heap_MixedOperations/{}",
        std::any::type_name::<T>().split(':').next_back().unwrap()
    ));
    let plot_config = PlotConfiguration::default().summary_scale(AxisScale::Logarithmic);
    group.plot_config(plot_config);

    const TEST_SET: [usize; 3] = [16, 64, 256]; // Adjust sizes as needed

    for &size in &TEST_SET {
        group.bench_with_input(BenchmarkId::new("ObliviousHeap", size), &size, |b, &size| {
            let mut heap = Heap::<u64>::new(size);
            let mut rng = rand::thread_rng();
            
            // Pre-insert some elements
            for i in 0..size/4 {
                heap.insert(rng.random_range(0..usize::MAX), i as u64);
            }
            
            // Create operation pattern: 0=insert, 1=find_min, 2=extract_min
            let mut ops = vec![0, 1, 1, 2];
            ops.extend_from_slice(&ops.clone());
            ops.shuffle(&mut rng);
            
            let mut op_idx = 0;
            
            b.iter(|| {
                match ops[op_idx] {
                    0 => {
                        // Insert
                        let key = rng.random_range(0..usize::MAX);
                        let value = rng.gen::<u64>();
                        black_box(heap.insert(black_box(key), black_box(value)));
                    },
                    1 => {
                        // Find min
                        black_box(heap.find_min());
                    },
                    2 => {
                        // Extract min
                        black_box(heap.extract_min());
                    },
                    _ => unreachable!(),
                }
                
                op_idx = (op_idx + 1) % ops.len();
            });
        });
    }
}

criterion_group!(name = benches_time;
    config = Criterion::default()
        .warm_up_time(std::time::Duration::from_millis(500))
        .measurement_time(std::time::Duration::from_secs(3));
    targets = 
        benchmark_heap_initialization,
        benchmark_heap_insert,
        benchmark_heap_find_min,
        benchmark_heap_extract_min,
        benchmark_heap_delete,
        benchmark_heap_operations_mixed);
criterion_main!(benches_time);