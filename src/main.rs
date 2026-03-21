use std::fs;
use lzo::{lzogeneric1x_1_compress, lzo1x_decompress_safe};

fn main() {
    let input = fs::read("input.txt").expect("failed to read file");
    println!("Input size: {} bytes", input.len());
    println!("First 8 bytes of input: {:?}", &input[..8.min(input.len())]);

let mut compressed = vec![0u8; 65536];
    let mut comp_len = 0;
    let mut wrkmem = vec![0usize; 1 << 13];

    // compress
    match lzogeneric1x_1_compress(&input, input.len(), &mut compressed, &mut comp_len, &mut wrkmem, 1) {
        Ok(_) => println!("Compression OK: {} -> {} bytes", input.len(), comp_len),
        Err(e) => { println!("Compression FAILED: {:?}", e); return; }
    }

    // print first 8 bytes of compressed — should start with [17, 1, ...]
    println!("First 8 bytes of compressed: {:?}", &compressed[..8]);

    let mut decompressed = vec![0u8; input.len() * 2 + 1024];
    let mut decomp_len = 0;

    // decompress
    match lzo1x_decompress_safe(&compressed[..comp_len], comp_len, &mut decompressed, &mut decomp_len) {
        Ok(_) => println!("Decompression OK: {} bytes", decomp_len),
        Err(e) => { 
            println!("Decompression FAILED: {:?}", e);
            // print what the decompressor actually saw
            println!("compressed bytes: {:?}", &compressed[..comp_len.min(32)]);
            return; 
        }
    }
fs::write("output.txt", &decompressed[..decomp_len])
    .expect("failed to write output file");

println!("Decompressed file written to output.txt");
    // verify byte by byte
    if input.len() != decomp_len {
        println!("❌ SIZE MISMATCH: input={} decomp={}", input.len(), decomp_len);
        return;
    }
    for (i, (a, b)) in input.iter().zip(decompressed[..decomp_len].iter()).enumerate() {
        if a != b {
            println!("❌ MISMATCH at byte {}: original={} decompressed={}", i, a, b);
            return;
        }
    }
    println!("✅ SUCCESS: all {} bytes match", input.len());
}


