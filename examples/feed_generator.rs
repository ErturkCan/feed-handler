/// Synthetic market data feed generator
///
/// Creates realistic order flow and writes binary feed to stdout or file.
/// Useful for testing and benchmarking.

use std::env;
use std::fs::File;
use std::io::Write;
use byteorder::{LittleEndian, ByteOrder};
use rand::Rng;

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();

    let output_path = if args.len() > 1 {
        args[1].clone()
    } else {
        "/tmp/feed_generator.bin".to_string()
    };

    let message_count: usize = if args.len() > 2 {
        args[2].parse().unwrap_or(10000)
    } else {
        10000
    };

    let mut output: Box<dyn Write> = if output_path == "stdout" {
        Box::new(std::io::stdout())
    } else {
        Box::new(File::create(&output_path)?)
    };

    let mut rng = rand::thread_rng();
    let mut order_id_counter = 1000u64;
    let mut sequence_number = 1u32;

    println!("Generating {} messages to {}", message_count, output_path);

    for i in 0..message_count {
        let msg_type = rng.gen_range(1u8..=4); // 1-4: Add, Modify, Delete, Trade

        match msg_type {
            1 => {
                // AddOrder
                let mut msg = [0u8; 46];
                msg[0] = 1;
                LittleEndian::write_u16(&mut msg[1..3], 46);
                LittleEndian::write_u32(&mut msg[3..7], sequence_number);

                let order_id = order_id_counter;
                order_id_counter += 1;

                let offset = rng.gen_range(-500_000000i64..500_000000i64);
                let price = if offset < 0 {
                    100_00000000u64.saturating_sub((-offset) as u64)
                } else {
                    100_00000000u64 + offset as u64
                };
                let qty = rng.gen_range(1u32..1000);
                let side = rng.gen_range(0u8..2);

                LittleEndian::write_u64(&mut msg[8..16], order_id);
                LittleEndian::write_u64(&mut msg[16..24], price);
                LittleEndian::write_u32(&mut msg[24..28], qty);
                msg[28] = side;

                output.write_all(&msg)?;
            }

            2 => {
                // ModifyOrder
                let mut msg = [0u8; 26];
                msg[0] = 2;
                LittleEndian::write_u16(&mut msg[1..3], 26);
                LittleEndian::write_u32(&mut msg[3..7], sequence_number);

                let order_id = if order_id_counter > 1000 {
                    rng.gen_range(1000u64..order_id_counter)
                } else {
                    1000
                };

                let new_qty = rng.gen_range(1u32..1000);

                LittleEndian::write_u64(&mut msg[8..16], order_id);
                LittleEndian::write_u32(&mut msg[16..20], new_qty);

                output.write_all(&msg)?;
            }

            3 => {
                // DeleteOrder
                let mut msg = [0u8; 16];
                msg[0] = 3;
                LittleEndian::write_u16(&mut msg[1..3], 16);
                LittleEndian::write_u32(&mut msg[3..7], sequence_number);

                let order_id = if order_id_counter > 1000 {
                    rng.gen_range(1000u64..order_id_counter)
                } else {
                    1000
                };

                LittleEndian::write_u64(&mut msg[8..16], order_id);

                output.write_all(&msg)?;
            }

            4 => {
                // Trade
                let mut msg = [0u8; 38];
                msg[0] = 4;
                LittleEndian::write_u16(&mut msg[1..3], 38);
                LittleEndian::write_u32(&mut msg[3..7], sequence_number);

                let buyer_id = if order_id_counter > 1000 {
                    rng.gen_range(1000u64..order_id_counter)
                } else {
                    1000
                };

                let seller_id = if order_id_counter > 1001 {
                    rng.gen_range(1000u64..order_id_counter)
                } else {
                    1001
                };

                let offset = rng.gen_range(-500_000000i64..500_000000i64);
                let price = if offset < 0 {
                    100_00000000u64.saturating_sub((-offset) as u64)
                } else {
                    100_00000000u64 + offset as u64
                };
                let qty = rng.gen_range(1u32..1000);

                LittleEndian::write_u64(&mut msg[8..16], buyer_id);
                LittleEndian::write_u64(&mut msg[16..24], seller_id);
                LittleEndian::write_u64(&mut msg[24..32], price);
                LittleEndian::write_u32(&mut msg[32..36], qty);

                output.write_all(&msg)?;
            }

            _ => {}
        }

        sequence_number += 1;

        if i % 1000 == 0 && i > 0 {
            println!("Generated {} messages", i);
        }
    }

    println!("Feed generation complete: {} messages", message_count);
    println!("File size: {} bytes", message_count * 40); // approximate

    Ok(())
}
