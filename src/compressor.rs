use std::cmp::min;
use crate::helpers::{get_unaligned_32le, put_unaligned_le32,get_unaligned_64le};

// simple constants
const D_BITS: usize = 13;
const D_SIZE: usize = 1 << D_BITS;
const D_MASK: u32 = (D_SIZE - 1) as u32;

const M2_MAX_LEN: usize = 8;
const M2_MAX_OFFSET: usize = 0x0800;

const M3_MARKER: u8 = 32;
const M3_MAX_LEN: usize = 33;
const M3_MAX_OFFSET: usize = 0x4000;

const M4_MARKER: usize = 16;
const M4_MAX_LEN: usize = 9;
const M4_MAX_OFFSET_V0: usize = 0xbfff;
const M4_MAX_OFFSET_V1: usize = 0xbffe;

const MIN_ZERO_RUN_LENGTH: usize = 4;
const MAX_ZERO_RUN_LENGTH: usize = 2047 + MIN_ZERO_RUN_LENGTH;



#[derive(Debug)]
pub enum Error{
	OutputOverrun
}

macro_rules! need_op {
	($op:expr,$out:expr,$n:expr) => {
		if $op +$n > $out.len()
		{
            return Err(Error::OutputOverrun);
        }
		
	};
}

pub fn lzo1x_do_compress(input :&[u8],out :&mut [u8],out_ind :&mut usize,tp :&mut usize,wrkmem :&mut[usize],state_offset :&mut isize,bitstream_version :u8) -> Result<(),Error>
{
    let ip_end=input.len()-20;
    let in_end=input.len();
    let mut ip :usize=0;
    let mut ii=ip;
    let mut op=*out_ind;
    let mut ti=*tp;
    let mut dict=wrkmem;

    if ti<4
    {ip+=4-ti;}
    'm: loop
    {

        let mut m_pos:usize=0;
        let mut t:usize;
        let mut m_len:usize;
        let mut m_off:usize;
        let mut dv:u32;
        let mut run_length:usize=0;


       'literal :loop
       {
            ip+=1+((ip-ii)>>5);
       'next: loop{
        if ip>=ip_end
        {break 'm;}
        dv= get_unaligned_32le(input,ip);
        //Zero run check
        if dv==0 && bitstream_version!=0
        {
            let mut ir=ip+4;
            let  limit=min(ip_end,ip+MAX_ZERO_RUN_LENGTH+1);

            let mut dv64:u64=0;
            while ir+32<=limit
            {
                dv64=u64::from_le_bytes(input[ir..ir+8].try_into().unwrap());  
                dv64|=u64::from_le_bytes(input[ir+8..ir+16].try_into().unwrap());
                dv64|=u64::from_le_bytes(input[ir+16..ir+24].try_into().unwrap());
                dv64|=u64::from_le_bytes(input[ir+24..ir+32].try_into().unwrap());   
                if dv64!=0
                {break;}
                ir+=32           
            }

			while ir+8<=limit
			{
				dv64=get_unaligned_64le(input,ir) as u64;
				if dv64!=0
				{
					ir += (dv64.trailing_zeros()/8 )as usize;
					break;
				}
				ir+=8;
			}

            while ir<limit && input[ir]==0
            {ir+=1;}
            run_length= ir-ip;
            if run_length>MAX_ZERO_RUN_LENGTH
            {run_length=MAX_ZERO_RUN_LENGTH}


             
        }
        //Match check
        else
        {
            t =(((0x1824429d_u32.wrapping_mul(dv)) >> (32-D_BITS)) & D_MASK) as usize;
            m_pos = dict[t];
            dict[t as usize]=ip;
            // if value in dictionary dont match 
            if dv!=get_unaligned_32le(input, m_pos)
            {continue 'literal ;}
        } 
        //literal encoding after match or zero run found
        ii-=ti;
        ti=0;
        t=ip-ii;
        if t!=0
        {
            if t<=3
            {
                out[(op as isize +*state_offset) as usize]|=t as u8;
                need_op!(op,out,4);
                out[op..op+4].copy_from_slice(&input[ii..ii+4]);
                op+=t; 
            }
            else if t<=16
            {
                need_op!(op,out,17) ;    
                out[op]= (t-3) as u8;
                op+=1;
                out[op..op+16].copy_from_slice(&input[ii..ii+16]);
                op+=t;
            }
            else {
                if t<=18{
                    need_op!(op,out,1);
                    out[op]=(t-3) as u8;
					op+=1;
                }
                else {
                    let mut tt = t - 18;
                    out[op]=0;
                    op+=1;
                    while tt>255
                    {
                        tt-=255;
                        out[op]=0;
                        op+=1;
                    }
                    out[op]=tt as u8;
                    op+=1
                }

                loop
                {
                    out[op..op+16].copy_from_slice(&input[ii..ii+16]);
                    op+=16;
                    ii+=16;
                    t -= 16;  
					if t<16
					{break;}
                }
                while t>0
                {               
                    out[op]=input[ii];
                    op+=1;ii+=1;
					t-=1;
                }
            }
        }
        //if its a zero run goto finished_writing instruction
        if run_length!=0
        {
            ip+=run_length;
            run_length -= MIN_ZERO_RUN_LENGTH;
            need_op!(op,out,4);
            let val=(run_length << 21) | 0xfffc18| (run_length & 0x7);
            put_unaligned_le32(out, op, val as u32 );
            op+=4;
			run_length=0;
			*state_offset=-3;
            ii=ip;
			continue 'next;
        }

		m_len=4;
        'outer: loop{
#[cfg(all(feature = "efficient_unaligned_access", feature = "ctz64"))]
   {        
		    let mut v: u64= get_unaligned_64le(input,ip + m_len)^get_unaligned_64le(input,m_pos + m_len);
            if v==0{
				loop
                {
					m_len += 8;
                    v = get_unaligned_64le(input,ip + m_len)^get_unaligned_64le(input,m_pos + m_len);
				    if ip + m_len >= ip_end
					{break 'outer;}
					if(v!=0)
					{break;}
                }      
		    }
			m_len+=(v.trailing_zeros()/8) as usize;
	}
#[cfg(all(not(all(feature = "efficient_unaligned_access", feature = "ctz64")),feature = "efficient_unaligned_access",feature = "ctz32"))]
   {
        let mut v = get_unaligned_32le(input,ip + m_len) ^
				    get_unaligned_32le(input,m_pos + m_len);
        if v==0
        {
            while(v==0)
            {
                m_len += 4;
				v = get_unaligned_32le(input,ip + m_len) ^
				    get_unaligned_32le(input,m_pos + m_len);
				if v != 0
					{break;}
				m_len += 4;
				v = get_unaligned_32le(input,ip + m_len) ^
				    get_unaligned_32le(input,m_pos + m_len);
				if ip + m_len >= ip_end
 				{break 'outer;}
            }
        }
        m_len+=(v.trailing_zeros()/8) as usize;
	}
#[cfg(not(feature = "efficient_unaligned_access"))]
{

    if input[ip+m_len]==input[m_pos+m_len] 
    {
        loop
        {
            m_len += 1;
			if input[ip+m_len] != input[m_pos+m_len]
				{break;}
			m_len += 1;
			if input[ip+m_len] != input[m_pos+m_len]
				{break;}
			m_len += 1;
			if input[ip+m_len] != input[m_pos+m_len]
				{break;}
			m_len += 1;
			if input[ip+m_len] != input[m_pos+m_len]
			{break;}
			m_len += 1;
			if input[ip+m_len] != input[m_pos+m_len]
				{break;}
			m_len += 1;
			if input[ip+m_len] != input[m_pos+m_len]
	            {break;}
			m_len += 1;
			if input[ip+m_len] != input[m_pos+m_len]
				{break;}
			m_len += 1;
			if ip + m_len >= ip_end
				{break 'outer;}
        }

    }
  
}
break 'outer;
	}
'm_len_done: loop
    {       
		m_off = ip - m_pos;
		ip += m_len;
		if m_len <= M2_MAX_LEN && m_off <= M2_MAX_OFFSET {
			m_off -= 1;
			need_op!(op,out,2);
			out[op] = (((m_len - 1) << 5) | ((m_off & 7) << 2)) as u8;
            op+=1;
			out[op] = (m_off >> 3) as u8;
            op+=1;
		} else if m_off <= M3_MAX_OFFSET {
			m_off -= 1;
			need_op!(op,out,1);
			if m_len <= M3_MAX_LEN
				{
                    out[op] = M3_MARKER | (m_len - 2) as u8;
                    op+=1;
                }
			else {
				m_len -= M3_MAX_LEN;
				out[op] = M3_MARKER | 0;
                op+=1;
				while m_len > 255{
					m_len -= 255;
					need_op!(op,out,1);
					out[op]=0;
					op+=1;
				}
				need_op!(op,out,1);
				out[op] = m_len as u8;
                op+=1;
			}
			need_op!(op,out,2);
			out[op] = (m_off << 2) as u8;
            op+=1;
			out[op] = (m_off >> 6) as u8;
            op+=1;
		} else {
			m_off -= 0x4000;
			need_op!(op,out,1);
			if m_len <= M4_MAX_LEN
				{
                    out[op]= (M4_MARKER | ((m_off >> 11) & 8) | (m_len - 2)) as u8;
                    op+=1;
                }
			else {
				if (m_off & 0x403f) == 0x403f
						&& (m_len >= 261)
						&& (m_len <= 264)
						&& (bitstream_version!=0) {

					ip -= m_len - 260;
					m_len = 260;
				}
				m_len -= M4_MAX_LEN;
				out[op] = (M4_MARKER | ((m_off >> 11) & 8)) as u8;
                op+=1;
				while m_len > 255 {
					need_op!(op,out,1);
					m_len -= 255;
					out[op] = 0;
                    op+=1;
				}
				need_op!(op,out,1);
				out[op] = (m_len) as u8;
                op+=1;
			}
			need_op!(op,out,2);
			out[op] = (m_off << 2) as u8;
            op+=1;
			out[op] = (m_off >> 6) as u8;
            op+=1;
		}
		*state_offset = -2;

		break 'm_len_done;
	}
	ii=ip;
	continue 'next;

}

}//'literal
	}//'m
*out_ind = op;
*tp = in_end - (ii - ti);
return Ok(());

}

pub fn  lzogeneric1x_1_compress(input :&[u8],in_len:usize,out :&mut [u8],out_len :&mut usize,wrkmem :&mut[usize],bitstream_version :u8) -> Result<(),Error>
{
	let mut ip = 0;
	let mut op = 0;
	let data_start: usize;
	let mut l = in_len;
	let mut t = 0;
	let mut state_offset:isize = -2;
	let m4_max_offset;
	// LZO v0 will never write 17 as first byte (except for zero-length
	// input), so this is used to version the bitstream
	if bitstream_version > 0 {
		out[op] = 17;
		op+=1;
		out[op] = bitstream_version;
		op+=1;
		m4_max_offset = M4_MAX_OFFSET_V1;
	} else {
		m4_max_offset = M4_MAX_OFFSET_V0;
	}
	data_start = op;
	while l > 20 {
		let ll = l.min(m4_max_offset + 1);
		let ll_end = ip + ll;
		// if ll_end + ((t + ll) >> 5) <= ll_end
		// 	{break;}
		wrkmem[..D_SIZE].fill(0);
		lzo1x_do_compress(&input[ip..ip+ll], out, &mut op, &mut t, wrkmem, &mut state_offset, bitstream_version)?;
		ip += ll;
		l  -= ll;
	}
	t += l;
	if t > 0 {
		let mut ii = in_len - t;
		if op == data_start && t <= 238 {
			need_op!(op,out,1);
			out[op] = (17 + t) as u8;
			op+=1;
		} else if t <= 3 {
			out [(op as isize + state_offset) as usize] |= t as u8;
		} else if t <= 18 {
			need_op!(op,out,1);
			out[op] = (t - 3) as u8;
			op+=1;
		} else {
			let mut tt = t - 18;
			need_op!(op,out,1);
			out[op] = 0;
			op+=1;
			while tt > 255 {
				tt -= 255;
				need_op!(op,out,1);
				out[op] = 0;
				op+=1;
			}
			need_op!(op,out,1);
			out[op] = tt as u8;
			op+=1;
		}
		need_op!(op,out,t);
		if t >= 16
		{
			loop {
				out[op..op+16].copy_from_slice(&input[ii..ii+16]);
				op += 16;
				ii += 16;
				t -= 16;
				if t<16
				{break;}
			}
		}
		if t > 0 {
			loop {
			out[op] = input[ii];
			ii+=1;
			op+=1;
			t-=1;
			if t<1
			{break;}
		    }
		}
	}
	need_op!(op,out,3);
	out[op] = (M4_MARKER | 1) as u8;
	op+=1;
	out[op] = 0;
	op+=1;
	out[op] = 0;
	op+=1;
	*out_len = op;
	return Ok(());

}


         
