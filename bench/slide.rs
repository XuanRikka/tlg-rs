use std::hint::black_box;
use criterion::{
    BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main,
};
use tlg::tlg5::slide::{SlideCompressor, SLIDE_N, SLIDE_M};

// 生成可压缩的测试数据（重复模式）
fn prepare_input(size: usize) -> Vec<u8> {
    let seed = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_-";
    let mut data = Vec::with_capacity(size);
    while data.len() < size {
        data.extend_from_slice(seed);
    }
    data.truncate(size);
    data
}

// ---------- 原始压缩/解压基准 ----------
fn bench_slide_compress(c: &mut Criterion) {
    let mut group = c.benchmark_group("slide_compress");
    let sizes = [256 * 1024, 1024 * 1024, 4 * 1024 * 1024];

    for &size in &sizes {
        let data = prepare_input(size);
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &data, |b, input| {
            b.iter_batched(
                || SlideCompressor::new(),
                |mut comp| {
                    let out = comp.encode(black_box(input));
                    black_box(out);
                },
                BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

fn bench_slide_decompress(c: &mut Criterion) {
    let mut group = c.benchmark_group("slide_decompress");
    let sizes = [256 * 1024, 1024 * 1024, 4 * 1024 * 1024];

    for &size in &sizes {
        let data = prepare_input(size);
        let mut compressor = SlideCompressor::new();
        let compressed = compressor.encode(&data);

        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &compressed, |b, input| {
            b.iter_batched(
                || SlideCompressor::new(),
                |mut decomp| {
                    let out = decomp.decode(black_box(input));
                    black_box(out);
                },
                BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

// ---------- 回溯功能基准 ----------
fn bench_store_restore(c: &mut Criterion) {
    let mut group = c.benchmark_group("store_restore");
    let sizes = [256 * 1024, 1024 * 1024, 4 * 1024 * 1024];

    for &size in &sizes {
        let data = prepare_input(size);
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &data, |b, input| {
            b.iter_batched(
                || {
                    let mut comp = SlideCompressor::new();
                    // 先压缩到一半，产生非初始状态
                    let half = input.len() / 2;
                    comp.encode(&input[..half]);
                    comp
                },
                |mut comp| {
                    comp.store();
                    comp.restore();
                    black_box(());
                },
                BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

fn bench_compress_with_backup(c: &mut Criterion) {
    let mut group = c.benchmark_group("compress_with_backup");
    let sizes = [256 * 1024, 1024 * 1024, 4 * 1024 * 1024];

    for &size in &sizes {
        let data = prepare_input(size);
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &data, |b, input| {
            b.iter_batched(
                || SlideCompressor::new(),
                |mut comp| {
                    // 每 64KB 回溯一次（模拟搜索最优匹配）
                    let chunk = 64 * 1024;
                    for chunk_data in input.chunks(chunk) {
                        comp.encode(chunk_data);
                        comp.store();
                        comp.restore(); // 立即恢复，模拟回溯
                    }
                    // 最终完成压缩，确保所有数据被处理
                    comp.encode(input);
                    black_box(());
                },
                BatchSize::LargeInput,
            );
        });
    }
    group.finish();
}

// 测试：先压缩一次，保存状态，然后继续压缩不同数据后恢复
fn bench_save_resume(c: &mut Criterion) {
    let mut group = c.benchmark_group("save_resume");
    let sizes = [256 * 1024, 1024 * 1024];

    for &size in &sizes {
        let data1 = prepare_input(size);
        let data2 = prepare_input(size / 2); // 不同的数据

        group.bench_with_input(BenchmarkId::from_parameter(size), &(data1, data2), |b, (d1, d2)| {
            b.iter_batched(
                || SlideCompressor::new(),
                |mut comp| {
                    // 压缩第一段数据
                    comp.encode(d1);
                    comp.store(); // 保存状态

                    // 压缩第二段数据（不同内容，模拟切换）
                    comp.encode(d2);

                    // 恢复到第一段压缩结束时的状态
                    comp.restore();

                    // 验证：可以继续压缩更多数据（这里省略）
                    black_box(());
                },
                BatchSize::LargeInput,
            );
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_slide_compress,
    bench_slide_decompress,
    bench_store_restore,
    bench_compress_with_backup,
    bench_save_resume
);
criterion_main!(benches);