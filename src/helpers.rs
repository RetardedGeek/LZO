pub fn get_unaligned_32le(input:&[u8],i:usize) -> u32
{
    return u32::from_le_bytes(input[i..i+4].try_into().unwrap());
}
pub fn get_unaligned_64le(input:& [u8],i:usize) -> u64
{
    return u64::from_le_bytes(input[i..i+8].try_into().unwrap());
}



pub fn put_unaligned_le32(out: &mut[u8],pos: usize,val: u32)
{
    let bytes=val.to_le_bytes();
    out[pos..pos+4].copy_from_slice(&bytes);
}
