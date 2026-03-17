pub enum Error
{
    output_overrun,
    input_overrun,
    lzo_e_error,
    input_not_consumed,
}

const MAX_255_COUNT:usize =(usize::MAX / 255) - 2;
const M2_MAX_OFFSET: usize = 0x0800;
const MIN_ZERO_RUN_LENGTH: usize = 4;

fn get_unaligned_le16(input: &[u8], ip: usize) -> usize
{
    (input[ip] as usize) | ((input[ip+1] as usize) << 8)
}

macro_rules! NEED
{
    ($input: expr,$p:expr,$n:expr) =>
    {
        if $p+$n>$input.len()
        {
            return Err(Error::input_overrun);
        }
    };
    ($out: expr,$p:expr ,$n:expr) => 
    {
        if $p + $n>$out.len()
        {
            return Err(Error::output_overrun);
        }
    };
}
macro_rules! HAVE
{
    ($ar:expr,$p:expr,$n:expr) =>
    {
        $p+$n <= $ar.len()
    };
}
macro_rules! TEST_LB
{
    ($m_pos:expr,$op:expr) =>
    {
        if $m_pos > $op
        {
            return Err(Error::lzo_e_error);
        }
    };
}

pub fn lzo1x_decompress_safe(input: &[u8], in_len: usize, out: &mut[u8], out_len: &mut usize) -> Result<(), Error>
{
    let mut op: usize;
    let mut ip: usize;
    let mut t: usize = 0;
    let mut next: usize = 0;
    let mut state: usize = 0;
    let mut m_pos: usize = 0;
    let ip_end: usize = in_len;
    let bitstream_version: u8;

    op = 0;
    ip = 0;
    if in_len < 3
    {
        *out_len = op;
        return Err(Error::input_overrun);
    }
    if in_len >= 5 && input[ip] == 17
    {
        bitstream_version = input[ip+1];
        ip += 2;
    }
    else
    { bitstream_version = 0; }

    'outer: loop
    {
        let mut skip_decode = false;
        if input[ip] > 17
        {
            t = (input[ip] as usize) - 17;
            ip += 1;
            if t < 4
            {
                next = t;
                // match_next inline
                state = next;
                t = next;
                if HAVE!(input,ip,6) && HAVE!(out,op,4)
                {
                    out[op..op+4].copy_from_slice(&input[ip..ip+4]);
                    op += t;
                    ip += t;
                }
                else
                {
                    NEED!(input,ip,t+3);
                    NEED!(out,op,t);
                    while t > 0
                    {
                        out[op] = input[ip];
                        op += 1; ip += 1;
                        t -= 1;
                    }
                }
                continue 'outer;
            }
            t += 3;
            skip_decode = true;
        }

        'inner: loop
        {
        if !skip_decode
        {
            t = input[ip] as usize;
            ip += 1;
            if t < 16
            {
                if state == 0
                {
                    if t == 0
                    {
                        let mut offset: usize;
                        let ip_last = ip;
                        while input[ip] == 0
                        {
                            ip += 1;
                            NEED!(input,ip,1);
                        }
                        offset = ip - ip_last;
                        if offset > MAX_255_COUNT
                        {
                            return Err(Error::lzo_e_error);
                        }
                        offset = (offset << 8) - offset;
                        t += offset + 15 + (input[ip] as usize);
                        ip += 1;
                    }
                    t += 3;

                    // literal copy from input to out
                    if HAVE!(input,ip,t+15) && HAVE!(out,op,t+15)
                    {
                        let ie = ip + t;
                        let oe = op + t;
                        loop
                        {
                            out[op..op+16].copy_from_slice(&input[ip..ip+16]);
                            op += 16;
                            ip += 16;
                            if ip >= ie
                            { break; }
                        }
                        ip = ie;
                        op = oe;
                    }
                    else
                    {
                        NEED!(out,op,t);
                        NEED!(input,ip,t+3);
                        loop
                        {
                            out[op] = input[ip];
                            op += 1; ip += 1;
                            t -= 1;
                            if t <= 0
                            { break; }
                        }
                    }
                    state = 4;
                    continue 'inner;
                }
                else if state != 4
                {
                    next = t & 3;
                    m_pos = op - 1;
                    m_pos -= t >> 2;
                    m_pos -= (input[ip] as usize) << 2;
                    ip += 1;
                    TEST_LB!(m_pos,op);
                    NEED!(out,op,2);
                    out[op+0] = out[m_pos+0];
                    out[op+1] = out[m_pos+1];
                    op += 2;
                    // match_next inline
                    state = next;
                    t = next;
                    if HAVE!(input,ip,6) && HAVE!(out,op,4)
                    {
                        out[op..op+4].copy_from_slice(&input[ip..ip+4]);
                        op += t;
                        ip += t;
                    }
                    else
                    {
                        NEED!(input,ip,t+3);
                        NEED!(out,op,t);
                        while t > 0
                        {
                            out[op] = input[ip];
                            op += 1; ip += 1;
                            t -= 1;
                        }
                    }
                    continue 'inner;
                }
                else
                {
                    next = t & 3;
                    m_pos = op - (1 + M2_MAX_OFFSET);
                    m_pos -= t >> 2;
                    m_pos -= (input[ip] as usize) << 2;
                    ip += 1;
                    t = 3;
                }
            }
            else if t >= 64
            {
                next = t & 3;
                m_pos = op - 1;
                m_pos -= (t >> 2) & 7;
                m_pos -= (input[ip] as usize) << 3;
                ip += 1;
                t = (t >> 5) - 1 + (3 - 1);
            }
            else if t >= 32
            {
                t = (t & 31) + (3 - 1);
                if t == 2
                {
                    let mut offset: usize;
                    let ip_last = ip;
                    while input[ip] == 0
                    {
                        ip += 1;
                        NEED!(input,ip,1);
                    }
                    offset = ip - ip_last;
                    if offset > MAX_255_COUNT
                    {
                        return Err(Error::lzo_e_error);
                    }
                    offset = (offset << 8) - offset;
                    t += offset + 31 + (input[ip] as usize);
                    ip += 1;
                    NEED!(input,ip,2);
                }
                m_pos = op - 1;
                next = get_unaligned_le16(input, ip);
                ip += 2;
                m_pos -= next >> 2;
                next &= 3;
            }
            else
            {
                NEED!(input,ip,2);
                next = get_unaligned_le16(input, ip);
                if ((next & 0xfffc) == 0xfffc) && ((t & 0xf8) == 0x18) && bitstream_version != 0
                {
                    NEED!(input,ip,3);
                    t &= 7;
                    t |= (input[ip+2] as usize) << 3;
                    t += MIN_ZERO_RUN_LENGTH;
                    NEED!(out,op,t);
                    out[op..op+t].fill(0);
                    op += t;
                    next &= 3;
                    ip += 3;
                    // match_next inline
                    state = next;
                    t = next;
                    if HAVE!(input,ip,6) && HAVE!(out,op,4)
                    {
                        out[op..op+4].copy_from_slice(&input[ip..ip+4]);
                        op += t;
                        ip += t;
                    }
                    else
                    {
                        NEED!(input,ip,t+3);
                        NEED!(out,op,t);
                        while t > 0
                        {
                            out[op] = input[ip];
                            op += 1; ip += 1;
                            t -= 1;
                        }
                    }
                    continue 'inner;
                }
                else
                {
                    m_pos = op;
                    m_pos -= (t & 8) << 11;
                    t = (t & 7) + (3 - 1);
                    if t == 2
                    {
                        let mut offset: usize;
                        let ip_last = ip;
                        while input[ip] == 0
                        {
                            ip += 1;
                            NEED!(input,ip,1);
                        }
                        offset = ip - ip_last;
                        if offset > MAX_255_COUNT
                        {
                            return Err(Error::lzo_e_error);
                        }
                        offset = (offset << 8) - offset;
                        t += offset + 7 + (input[ip] as usize);
                        ip += 1;
                        NEED!(input,ip,2);
                        next = get_unaligned_le16(input, ip);
                    }
                    ip += 2;
                    m_pos -= next >> 2;
                    next &= 3;
                    if m_pos == op
                    {
                        *out_len = op;
                        return if t != 3
                        {
                            Err(Error::lzo_e_error)
                        }
                        else if ip == ip_end
                        {
                            Ok(())
                        }
                        else if ip < ip_end
                        {
                            Err(Error::input_not_consumed)
                        }
                        else
                        {
                            Err(Error::input_overrun)
                        };
                    }
                    m_pos -= 0x4000;
                }
            }
        } // end if !skip_decode

        // TEST_LB(m_pos)
        TEST_LB!(m_pos,op);

        // match copy
        if op - m_pos >= 8
        {
            let oe = op + t;
            if HAVE!(out,op,t+15)
            {
                loop
                {
                    out[op..op+8].copy_from_slice(&input[m_pos..m_pos+8]);
                    op += 8;
                    m_pos += 8;
                    out[op..op+8].copy_from_slice(&input[m_pos..m_pos+8]);
                    op += 8;
                    m_pos += 8;
                    if op >= oe
                    { break; }
                }
                op = oe;
                if HAVE!(input,ip,6)
                {
                    state = next;
                    out[op..op+4].copy_from_slice(&input[ip..ip+4]);
                    op += next;
                    ip += next;
                    continue 'outer;
                }
            }
            else
            {
                NEED!(out,op,t);
                loop
                {
                    out[op] = out[m_pos];
                    op += 1; m_pos += 1;
                    if op >= oe
                    { break; }
                }
            }
        }
        else
        {
            let oe = op + t;
            NEED!(out,op,t);
            out[op+0] = out[m_pos+0];
            out[op+1] = out[m_pos+1];
            op += 2;
            m_pos += 2;
            loop
            {
                out[op] = out[m_pos];
                op += 1; m_pos += 1;
                if op >= oe
                { break; }
            }
        }

        // match_next
        state = next;
        t = next;
        if HAVE!(input,ip,6) && HAVE!(out,op,4)
        {
            out[op..op+4].copy_from_slice(&input[ip..ip+4]);
            op += t;
            ip += t;
        }
        else
        {
            NEED!(input,ip,t+3);
            NEED!(out,op,t);
            while t > 0
            {
                out[op] = input[ip];
                op += 1; ip += 1;
                t -= 1;
            }
        }

        continue 'inner;
    }
    } // end 'outer loop

    *out_len = op;
    return Ok(());
}