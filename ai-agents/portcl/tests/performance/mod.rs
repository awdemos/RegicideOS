//! Performance benchmarks for PortCL
//!
//! These benchmarks measure the performance of critical operations
//! to ensure they meet performance targets and detect regressions.

use criterion::{black_box, criterion_group, criterion_main, Criterion};

#[cfg(test)]
mod benchmarks {
    use super::*;
    use portcl::utils::{format_duration, format_bytes};

    fn benchmark_format_duration(c: &mut Criterion) {
        c.bench_function("format_duration", |b| {
            b.iter(|| format_duration(black_box(3600)))
        });
    }

    fn benchmark_format_bytes(c: &mut Criterion) {
        c.bench_function("format_bytes", |b| {
            b.iter(|| format_bytes(black_box(1024 * 1024)))
        });
    }

    fn benchmark_error_creation(c: &mut Criterion) {
        use portcl::error::PortCLError;
        c.bench_function("error_creation", |b| {
            b.iter(|| PortCLError::Portage(black_box("test error".to_string())))
        });
    }

    criterion_group!(
        benches,
        benchmark_format_duration,
        benchmark_format_bytes,
        benchmark_error_creation
    );
    criterion_main!(benches);
}