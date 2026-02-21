/// Order book update latency benchmarks

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use feed_handler::{OrderBook, Order, Side};

fn bench_add_order(c: &mut Criterion) {
    c.bench_function("book_add_order", |b| {
        let mut book = OrderBook::new();
        let mut order_id = 0u64;

        b.iter(|| {
            book.bids.insert(100_00000000 + order_id, 100);
            book.orders.insert(order_id, Order {
                order_id,
                price: 100_00000000 + order_id,
                quantity: 100,
                side: Side::Bid,
            });
            order_id += 1;
        });
    });
}

fn bench_delete_order(c: &mut Criterion) {
    c.bench_function("book_delete_order", |b| {
        let mut book = OrderBook::new();

        // Pre-populate with orders
        for i in 0..1000 {
            book.bids.insert(100_00000000 + i, 100);
            book.orders.insert(i as u64, Order {
                order_id: i as u64,
                price: 100_00000000 + i,
                quantity: 100,
                side: Side::Bid,
            });
        }

        let mut id_to_delete = 0u64;
        b.iter(|| {
            if let Some(order) = book.orders.remove(&id_to_delete) {
                if let Some(qty) = book.bids.get_mut(&order.price) {
                    *qty = qty.saturating_sub(order.quantity);
                    if *qty == 0 {
                        book.bids.remove(&order.price);
                    }
                }
            }
            id_to_delete = (id_to_delete + 1) % 1000;
        });
    });
}

fn bench_best_bid(c: &mut Criterion) {
    let mut book = OrderBook::new();

    // Populate with levels
    for i in 0..100 {
        book.bids.insert(100_00000000 - (i * 1_00000000), 100 + i as u32);
    }

    c.bench_function("book_best_bid", |b| {
        b.iter(|| {
            black_box(book.best_bid())
        });
    });
}

fn bench_best_ask(c: &mut Criterion) {
    let mut book = OrderBook::new();

    // Populate with levels
    for i in 0..100 {
        book.asks.insert(100_00000000 + (i * 1_00000000), 100 + i as u32);
    }

    c.bench_function("book_best_ask", |b| {
        b.iter(|| {
            black_box(book.best_ask())
        });
    });
}

fn bench_spread(c: &mut Criterion) {
    let mut book = OrderBook::new();

    // Populate with levels
    for i in 0..100 {
        book.bids.insert(100_00000000 - (i * 1_00000000), 100 + i as u32);
        book.asks.insert(100_00000000 + (i * 1_00000000), 100 + i as u32);
    }

    c.bench_function("book_spread", |b| {
        b.iter(|| {
            black_box(book.spread())
        });
    });
}

fn bench_depth(c: &mut Criterion) {
    let mut book = OrderBook::new();

    // Populate with levels
    for i in 0..100 {
        book.bids.insert(100_00000000 - (i * 1_00000000), 100 + i as u32);
        book.asks.insert(100_00000000 + (i * 1_00000000), 100 + i as u32);
    }

    c.bench_function("book_depth_10", |b| {
        b.iter(|| {
            black_box(book.depth(10))
        });
    });
}

criterion_group!(
    benches,
    bench_add_order,
    bench_delete_order,
    bench_best_bid,
    bench_best_ask,
    bench_spread,
    bench_depth
);
criterion_main!(benches);
