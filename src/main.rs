use lzo::{lzo1x_decompress_safe, lzogeneric1x_1_compress};
use std::fs;
use std::io::{self, Read};

fn main() {
    loop {
        println!(">lzo>");
        let mut cl_input = String::new();
        io::stdin().read_line(&mut cl_input).unwrap();
        let mut iter = cl_input.split_whitespace();
        let com = match iter.next() {
            Some(c) => c,
            None => continue,
        };
        if com == "exit" {
            return;
        } else {
            let input_path = match iter.next() {
                Some(p) => p,
                None => {
                    println!("Missing file path");
                    continue;
                }
            };
            if com == "compress" {
                let input = fs::read(input_path).expect("failed to read file");

                println!("Input size: {} bytes", input.len());

                let mut compressed = vec![0u8; input.len() + input.len() / 16 + 64 + 3];
                compressed.fill(0);
                let mut comp_len = 0;
                let mut wrkmem = vec![0usize; 1 << 16];

                // compress
                match lzogeneric1x_1_compress(
                    &input,
                    input.len(),
                    &mut compressed,
                    &mut comp_len,
                    &mut wrkmem,
                    1,
                ) {
                    Ok(_) => println!("Compression OK: {} -> {} bytes", input.len(), comp_len),
                    Err(e) => {
                        println!("Compression FAILED: {:?}", e);
                        continue;
                    }
                }
                let file_name = input_path.rsplit('/').next().unwrap();
                let name = "compressed_".to_string() + file_name;

                fs::write(&name, &compressed[..comp_len]).expect("failed to write compressed file");
                println!("Compressed file written to {}", name);
            } else if (com == "decompress") {
                let input = fs::read(input_path).expect("failed to read file");
                println!("Input size: {} bytes", input.len());

                let mut decompressed = vec![0u8; 20 * input.len()];
                let mut decomp_len = 0;

                // decompress
                match lzo1x_decompress_safe(
                    &input[..input.len()],
                    input.len(),
                    &mut decompressed,
                    &mut decomp_len,
                ) {
                    Ok(_) => println!("Decompression OK: {} bytes", decomp_len),
                    Err(e) => {
                        println!("Decompression FAILED: {:?}", e);
                    }
                }
                let file_name = input_path.rsplit('/').next().unwrap();
                let name = "decompressed_".to_string() + file_name;
                fs::write(&name, &decompressed[..decomp_len])
                    .expect("failed to write decompressed file");

                println!("Decompressed file written to {}", name);
            }
        }
    }
}
