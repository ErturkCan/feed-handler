/// Decode throughput and latency benchmarks

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use feed_handler::Decoder;
use byteorder::LittleEndian;

fn create_message_buffer(msg_count: usize) -> Vec<u8> {
    let mut buffer = Vec::new();

    for seq in 0..msg_count {
        let mut msg = vec![0u8; 46];
        msg[0] = 1; // AddOrder
        LittleEndian::write_u16(&mut msg[1..3], 46);
        LittleEndian::write_u32(&mut msg[3..7], seq as u32);

        // Fill with some data
        LittleEndian::write_u64(&mut msg[8..16], seq as u64);
        LittleEndian::write_u64(&mut msg[16..24], 100_00000000);
        LittleEndian::write_u32(&mut msg[24..28], 100);
        msg[28] = 0; // side = bid

        buffer.extend_from_slice(&msg);
    }

    buffer
}

fn bench_decode_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("decode_throughput");

    for msg_count in [1000, 10000, 100000].iter() {
        let buffer = black_box(create_message_buffer(*msg_count));

        group.bench_with_input(
            BenchmarkId::from_parameter(msg_count),
            msg_count,
            |b, _| {
                b.iter(|| {
                    let mut count = 0;
                    let mut offset = 0;
                    while offset < buffer.len() {
                        if let Ok((_, consumed)) = Decoder::decode(&buffer[offset..]) {
                            offset += consumed;
                            count += 1;
                        } else {
                            break;
                        }
                    }
                    count
                });
            },
        );
    }
    group.finish();
}

fn bench_decode_latency(c: &mut Criterion) {
    let mut group = c.benchmark_group("decode_latency");

    // Single message decode latency
    let msg = {
        let mut msg = vec![0u8; 46];
        msg[0] = 1;
        LittleEndian::write_u16(&mut msg[1..3], 46);
        LittleEndian::write_u32(&mut msg[3..7], 42);
        msg
    };

    group.bench_function("single_message", |b| {
        b.iter(|| Decoder::decode(black_box(&msg)))
    });

    group.finish();
}

fn bench_decode_message_types(c: &mut Criterion) {
    let mut group = c.benchmark_group("message_types");

    // AddOrder
    let add_order = {
        let mut msg = vec![0u8; 46];
        msg[0] = 1;
        LittleEndian::write_u16(&mut msg[1..3], 46);
        msg
    };

    // ModifyOrder
    let modify_order = {
        let mut msg = vec![0u8; 26];
        msg[0] = 2;
        LittleEndian::write_u16(&mut msg[1..3], 26);
        msg
    };

    // DeleteOrder
    let delete_order = {
        let mut msg = vec![0u8; 16];
        msg[0] = 3;
        LittleEndian::write_u16(&mut msg[1..3], 16);
        msg
    };

    // Trade
    let trade = {
        let mut msg = vec![0u8; 30];
        msg[0] = 4;
        LittleEndian::write_u16(&mut msg[1..3], 30);
        msg
    };

    group.bench_function("add_order", |b| {
        b.iter(|| Decoder::decode(black_box(&add_order)))
    });

    group.bench_function("modify_order", |b| {
        b.iter(|| Decoder::decode(black_box(&modify_order)))
    });

    group.bench_function("delete_order", |b| {
        b.iter(|| Decoder::decode(black_box(&delete_order)))
    });

    group.bench_function("trade", |b| {
        b.iter(|| Decoder::decode(black_box(&trade)))
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_decode_throughput,
    bench_decode_latency,
    bench_decode_message_types
);
criterion_main!(benches);
