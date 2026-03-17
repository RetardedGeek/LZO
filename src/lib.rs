pub mod decompressor; 
pub mod compressor;  // your decompressor file
pub mod helpers;
pub use decompressor::lzo1x_decompress_safe;
pub use compressor::lzo1x_do_compress; 