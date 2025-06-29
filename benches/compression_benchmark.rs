use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rzx::core::compression::{compress, decompress};
use rzx::core::CompressionAlgorithm;

fn get_test_data(size: usize) -> Vec<u8> {
    (0..size).map(|i| (i % 256) as u8).collect()
}

fn benchmark_compression(c: &mut Criterion) {
    let data = get_test_data(1024 * 1024); // 1MB

    let mut group = c.benchmark_group("Compression");

    for &alg in &[CompressionAlgorithm::Deflate, CompressionAlgorithm::Lzma, CompressionAlgorithm::Zstd] {
        group.bench_function(format!("{:?}", alg), |b| {
            b.iter(|| compress(alg, black_box(&data), 6).unwrap());
        });
    }

    group.finish();
}

fn benchmark_decompression(c: &mut Criterion) {
    let data = get_test_data(1024 * 1024); // 1MB

    let mut group = c.benchmark_group("Decompression");

    for &alg in &[CompressionAlgorithm::Deflate, CompressionAlgorithm::Lzma, CompressionAlgorithm::Zstd] {
        let compressed = compress(alg, &data, 6).unwrap();
        group.bench_function(format!("{:?}", alg), |b| {
            b.iter(|| decompress(alg, black_box(&compressed)).unwrap());
        });
    }

    group.finish();
}

criterion_group!(benches, benchmark_compression, benchmark_decompression);
criterion_main!(benches);
