#![allow(missing_docs)]

use criterion::{criterion_group, criterion_main, Criterion};
use rand::{RngCore, SeedableRng};
use rand_chacha::ChaCha12Rng;
use std::cell::RefCell;
use std::hint::black_box;
use std::time::Instant;

thread_local! {
  static TL: RefCell<ChaCha12Rng> = RefCell::new(ChaCha12Rng::seed_from_u64(1));
}

#[inline]
fn tls_next_u64() -> u64 {
  TL.with(|r| r.borrow_mut().next_u64())
}

fn bench_rng(c: &mut Criterion) {
  c.bench_function("rng by_ref next_u64", |b| {
    let mut rng = ChaCha12Rng::seed_from_u64(1);
    b.iter(|| black_box(rng.next_u64()));
  });

  c.bench_function("rng tls next_u64", |b| {
    b.iter(|| black_box(tls_next_u64()));
  });

  c.bench_function("rng thread_rng next_u64", |b| {
    b.iter(|| black_box(rand::rng().next_u64()));
  });

  c.bench_function("rng tls precached10 next_u64", |b| {
    b.iter_custom(|iters| {
      let start = Instant::now();

      for _ in 0..iters {
        TL.with_borrow_mut(|r| {
          for _ in 0..10 {
            black_box(r.next_u64());
          }
        });
      }

      start.elapsed() / 10
    });
  });

  c.bench_function("rng thread_rng precached10 next_u64", |b| {
    b.iter_custom(|iters| {
      let start = Instant::now();

      for _ in 0..iters {
        let r = &mut rand::rng();
        for _ in 0..10 {
          black_box(r.next_u64());
        }
      }

      start.elapsed() / 10
    });
  });
}

criterion_group!(benches, bench_rng);
criterion_main!(benches);
