use std::sync::Arc;

use arrow::array::{PrimitiveArray, UInt16Type};
use arrow::buffer::Buffer;
use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};

fn make_frame_u16(width: u32, height: u32) -> Vec<u16> {
    let pixels = (width * height) as usize;
    let mut data = Vec::with_capacity(pixels);
    for i in 0..pixels {
        data.push((i as u16).wrapping_add(100));
    }
    data
}

fn bench_arrow_tap(c: &mut Criterion) {
    let mut group = c.benchmark_group("pvcam_arrow_tap");
    let width = 2048;
    let height = 2048;
    group.throughput(criterion::Throughput::Elements((width * height) as u64));

    group.bench_function(BenchmarkId::new("zero_copy_buffer", "u16->arrow"), |b| {
        b.iter_batched(
            || make_frame_u16(width, height),
            |pixels| {
                // No extra copy: buffer takes ownership of Vec<u16>
                let buffer = Buffer::from_vec(pixels);
                let array = PrimitiveArray::<UInt16Type>::new(Arc::new(buffer), None);
                // Minimal work to keep optimizer from discarding
                criterion::black_box(array.len())
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function(BenchmarkId::new("copy_into_buffer", "u16->arrow"), |b| {
        b.iter_batched(
            || make_frame_u16(width, height),
            |pixels| {
                // Simulate copy path: convert to bytes then to buffer
                let mut bytes = Vec::with_capacity(pixels.len() * 2);
                for p in pixels {
                    bytes.extend_from_slice(&p.to_le_bytes());
                }
                let buffer = Buffer::from(bytes);
                let array = PrimitiveArray::<UInt16Type>::new(Arc::new(buffer), None);
                criterion::black_box(array.len())
            },
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

criterion_group!(benches, bench_arrow_tap);
criterion_main!(benches);
