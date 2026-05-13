use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use trisult::trisult;
use trisult::{AccumulatorKind, Contextual, Contextuals, Diagnosed, Diagnosis, NoLoc, Trisult};

// --- SHARED TYPES ---
type ErrorTy = &'static str;
type WarnTy = &'static str;
type Tri<T> = Trisult<T, WarnTy, ErrorTy, NoLoc>;

// ============================================================================
// PIPELINE 1: STANDARD RESULT
// ============================================================================

fn std_parse(val: u64) -> Result<u64, ErrorTy> {
    Ok(val + 1)
}

fn std_validate(val: u64) -> Result<u64, ErrorTy> {
    if val > 10 {
        Ok(val)
    } else {
        Err("validation failed")
    }
}

fn std_process(val: u64) -> Result<u64, ErrorTy> {
    Ok(val * 2)
}

fn run_std_pipeline(val: u64) -> Result<u64, ErrorTy> {
    std_parse(val).and_then(std_validate).and_then(std_process)
}

// ============================================================================
// PIPELINE 2: TRISULT MANUAL ACCUMULATION
// ============================================================================

fn tri_parse(val: u64) -> Tri<u64> {
    Trisult::Ok(Diagnosed(val + 1, Contextuals::new(AccumulatorKind::All)))
}

fn tri_validate(val: u64) -> Tri<u64> {
    if val > 10 {
        Trisult::Ok(Diagnosed(val, Contextuals::new(AccumulatorKind::All)))
    } else {
        let mut errs = Contextuals::new(AccumulatorKind::All);
        errs.push_naive(Contextual::new(
            NoLoc,
            Diagnosis::Error("validation failed"),
        ));
        Trisult::Err(errs)
    }
}

fn tri_process(val: u64) -> Tri<u64> {
    Trisult::Ok(Diagnosed(val * 2, Contextuals::new(AccumulatorKind::All)))
}

fn run_tri_pipeline(val: u64) -> Tri<u64> {
    tri_parse(val).and_then(tri_validate).and_then(tri_process)
}

// ============================================================================
// PIPELINE 3: TRISULT MACRO
// ============================================================================

#[trisult]
fn mac_parse(val: u64) -> Tri<u64> {
    Some(val + 1)
}

#[trisult]
fn mac_validate(val: u64) -> Tri<u64> {
    if val > 10 {
        Some(val)
    } else {
        error!("validation failed", NoLoc);
        None
    }
}

#[trisult]
fn mac_process(val: u64) -> Tri<u64> {
    Some(val * 2)
}

// Short-circuits using standard `?` on the Option returned by `tri!`
#[trisult]
fn run_mac_pipeline_short(val: u64) -> Tri<u64> {
    let v1 = tri!(mac_parse(val))?;
    let v2 = tri!(mac_validate(v1))?;
    let v3 = tri!(mac_process(v2))?;
    Some(v3)
}

// Accumulates failures (if there were non-fatal ones) using Option::and_then
#[trisult]
fn run_mac_pipeline_accumulate(val: u64) -> Tri<u64> {
    let v1 = tri!(mac_parse(val));
    let v2 = v1.and_then(|v| tri!(mac_validate(v)));
    let v3 = v2.and_then(|v| tri!(mac_process(v)));
    v3
}

// ============================================================================
// BENCHMARK GROUPS
// ============================================================================

fn bench_pipeline_happy(c: &mut Criterion) {
    let mut group = c.benchmark_group("Pipeline Happy Path");

    group.bench_function("std_result", |b| {
        b.iter(|| black_box(run_std_pipeline(black_box(42))))
    });

    group.bench_function("trisult_manual", |b| {
        b.iter(|| black_box(run_tri_pipeline(black_box(42))))
    });

    group.bench_function("trisult_macro_short", |b| {
        b.iter(|| black_box(run_mac_pipeline_short(black_box(42))))
    });

    group.bench_function("trisult_macro_accumulate", |b| {
        b.iter(|| black_box(run_mac_pipeline_accumulate(black_box(42))))
    });

    group.finish();
}

fn bench_pipeline_error(c: &mut Criterion) {
    let mut group = c.benchmark_group("Pipeline Error Path");

    // Passing 5 will fail the validate step
    group.bench_function("std_result", |b| {
        b.iter(|| black_box(run_std_pipeline(black_box(5))))
    });

    group.bench_function("trisult_manual", |b| {
        b.iter(|| black_box(run_tri_pipeline(black_box(5))))
    });

    group.bench_function("trisult_macro_short", |b| {
        b.iter(|| black_box(run_mac_pipeline_short(black_box(5))))
    });

    group.bench_function("trisult_macro_accumulate", |b| {
        b.iter(|| black_box(run_mac_pipeline_accumulate(black_box(5))))
    });

    group.finish();
}

criterion_group!(benches, bench_pipeline_happy, bench_pipeline_error);
criterion_main!(benches);
