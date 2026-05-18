use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use std::hint::black_box;
use trisult::{trisult, Acc, Contextual, Contextuals, Diagnosed, Diagnosis, Most, NoLoc, Trisult};

#[cfg(feature = "alloc")]
use trisult::All;

// --- SHARED TYPES ---
type ErrorTy = &'static str;
type WarnTy = &'static str;

#[allow(type_alias_bounds)]
type Tri<T, A: Acc> = Trisult<T, WarnTy, ErrorTy, A::Acc<Diagnosis<WarnTy, ErrorTy>, NoLoc>>;

// ============================================================================
// IMPLEMENTATIONS
// ============================================================================

mod result {
    use super::*;

    pub fn fast_fail(inputs: &[i32]) -> Result<i32, ErrorTy> {
        let mut ret = 0;
        for &input in inputs {
            if input < 0 {
                return Err("error!");
            }

            ret += 1;
        }
        Ok(ret)
    }
}

mod manual {
    use super::*;

    pub fn fast_fail<T: Acc>(inputs: &[i32]) -> Tri<i32, T> {
        let mut ret = 0;
        for &input in inputs {
            if input < 0 {
                let mut errs = Contextuals::new(T::create_state());
                errs.push_naive(Contextual::new(NoLoc, Diagnosis::Error("error!")));
                return Trisult::Err(errs);
            }

            ret += 1;
        }

        Trisult::Ok(Diagnosed(ret, Contextuals::new(T::create_state())))
    }

    pub fn accumulate<T: Acc>(inputs: &[i32]) -> Tri<i32, T> {
        let mut errs = Contextuals::new(T::create_state());

        let mut ret = 0;
        for &input in inputs {
            if input < 0 {
                errs.push_naive(Contextual::new(NoLoc, Diagnosis::Error("error!")));
            }

            ret += 1;
        }

        if errs.is_empty() {
            Trisult::Ok(Diagnosed(ret, Contextuals::new(T::create_state())))
        } else {
            Trisult::Err(errs)
        }
    }
}

mod macros {
    use super::*;

    #[trisult]
    pub fn fast_fail<#[kind] T: Acc>(inputs: &[i32]) -> Tri<i32, T> {
        let mut ret = 0;
        for &input in inputs {
            if input < 0 {
                error!("error!", NoLoc);
                return None;
            }

            ret += 1;
        }

        Some(ret)
    }

    #[trisult]
    pub fn accumulate<#[kind] T: Acc>(inputs: &[i32]) -> Tri<i32, T> {
        let mut ret = 0;
        for &input in inputs {
            if input < 0 {
                error!("error!", NoLoc);
            }

            ret += 1;
        }

        // if failed, it'll be ignored by macro implementation
        Some(ret)
    }
}

// ============================================================================
// BENCHMARK GROUPS
// ============================================================================

macro_rules! decl_bench {
    () => {
        false
    };
    (std) => {
        true
    };
    ($fn_name:ident, $name:literal, $input:ident, $callee:ident, $std:literal) => {
        fn $fn_name(c: &mut Criterion) {
            let mut group = c.benchmark_group($name);
            for size in [100, 1000, 10000] {
                let input: Vec<i32> = $input(size);
                #[cfg($std)]
                group.bench_with_input(BenchmarkId::new("Result", size), &input, |b, input| {
                    b.iter(|| black_box(result::$callee(black_box(input))))
                });
                group.bench_with_input(
                    BenchmarkId::new("Manual/Most", size),
                    &input,
                    |b, input| b.iter(|| black_box(manual::$callee::<Most>(black_box(input)))),
                );
                #[cfg(feature = "alloc")]
                group.bench_with_input(BenchmarkId::new("Manual/All", size), &input, |b, input| {
                    b.iter(|| black_box(manual::$callee::<All>(black_box(input))))
                });
                group.bench_with_input(BenchmarkId::new("Macro/Most", size), &input, |b, input| {
                    b.iter(|| black_box(macros::$callee::<Most>(black_box(input))))
                });
                #[cfg(feature = "alloc")]
                group.bench_with_input(BenchmarkId::new("Macro/All", size), &input, |b, input| {
                    b.iter(|| black_box(macros::$callee::<All>(black_box(input))))
                });
            }
        }
    };
}

fn ok(size: usize) -> Vec<i32> {
    let mut x: i32 = 42; // use fixed seed for reproducibility
    let mut vec = vec![0; size];
    for i in &mut vec {
        // do xorshift32
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        *i = x.abs();
    }
    vec
}

fn early_err(size: usize) -> Vec<i32> {
    let mut vec = ok(size);
    vec[0] = -1;
    vec
}

fn late_err(size: usize) -> Vec<i32> {
    let mut vec = ok(size);
    vec[size - 1] = -1;
    vec
}

fn all_err(size: usize) -> Vec<i32> {
    ok(size).into_iter().map(|i| -i).collect()
}

decl_bench!(success, "Success", ok, fast_fail, true);
decl_bench!(early_fail, "Early Fail", early_err, fast_fail, true);
decl_bench!(late_fail, "Late Fail", late_err, fast_fail, true);
decl_bench!(multi_fail, "Multi Fail", all_err, accumulate, false);

criterion_group!(
    name = benches;
    config = Criterion::default();
    targets = success, early_fail, late_fail, multi_fail
);
criterion_main!(benches);
