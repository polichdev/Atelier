use std::io::{self, Cursor, Read};

const MAX_PIXELS: u64 = 16 * 1024 * 1024;
const MAX_BYTES: usize = 64 * 1024 * 1024;

#[derive(Debug, PartialEq, Eq)]
pub struct Texture {
    pub width: u32,
    pub height: u32,
    pub flags: u32,
    pub rgba: Vec<u8>,
}
fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
fn num(c: &mut Cursor<&[u8]>) -> io::Result<u32> {
    let mut b = [0; 4];
    c.read_exact(&mut b)?;
    Ok(u32::from_le_bytes(b))
}
fn tag(c: &mut Cursor<&[u8]>) -> io::Result<[u8; 8]> {
    let mut b = [0; 8];
    c.read_exact(&mut b)?;
    let mut zero = [0];
    c.read_exact(&mut zero)?;
    if zero[0] != 0 {
        return Err(invalid("Invalid texture tag"));
    }
    Ok(b)
}
fn decompress(input: &[u8], expected: usize) -> io::Result<Vec<u8>> {
    if expected > MAX_BYTES {
        return Err(invalid("Texture is too large"));
    }
    let mut out = Vec::with_capacity(expected);
    let mut i = 0usize;
    while i < input.len() {
        let token = input[i];
        i += 1;
        let mut literals = (token >> 4) as usize;
        if literals == 15 {
            loop {
                let b = *input.get(i).ok_or_else(|| invalid("Truncated LZ4"))? as usize;
                i += 1;
                literals = literals
                    .checked_add(b)
                    .ok_or_else(|| invalid("LZ4 overflow"))?;
                if b != 255 {
                    break;
                }
            }
        }
        let end = i
            .checked_add(literals)
            .filter(|&v| v <= input.len())
            .ok_or_else(|| invalid("Truncated LZ4 literals"))?;
        if out
            .len()
            .checked_add(literals)
            .filter(|&n| n <= expected)
            .is_none()
        {
            return Err(invalid("LZ4 output exceeds limit"));
        }
        out.extend_from_slice(&input[i..end]);
        i = end;
        if i == input.len() {
            break;
        }
        let pair = input
            .get(i..i + 2)
            .ok_or_else(|| invalid("Truncated LZ4 offset"))?;
        i += 2;
        let offset = u16::from_le_bytes([pair[0], pair[1]]) as usize;
        if offset == 0 || offset > out.len() {
            return Err(invalid("Invalid LZ4 offset"));
        }
        let mut length = (token & 15) as usize + 4;
        if token & 15 == 15 {
            loop {
                let b = *input.get(i).ok_or_else(|| invalid("Truncated LZ4"))? as usize;
                i += 1;
                length = length
                    .checked_add(b)
                    .ok_or_else(|| invalid("LZ4 overflow"))?;
                if b != 255 {
                    break;
                }
            }
        }
        if out
            .len()
            .checked_add(length)
            .filter(|&n| n <= expected)
            .is_none()
        {
            return Err(invalid("LZ4 output exceeds limit"));
        }
        for _ in 0..length {
            let v = out[out.len() - offset];
            out.push(v);
        }
    }
    if out.len() != expected {
        return Err(invalid("Incorrect LZ4 output size"));
    }
    Ok(out)
}
fn rgb565(n: u16) -> [u8; 4] {
    let r = ((n >> 11) & 31) as u8;
    let g = ((n >> 5) & 63) as u8;
    let b = (n & 31) as u8;
    [
        (r << 3) | (r >> 2),
        (g << 2) | (g >> 4),
        (b << 3) | (b >> 2),
        255,
    ]
}
fn blocks(format: u32, width: u32, height: u32, data: &[u8]) -> io::Result<Vec<u8>> {
    let block_size = if format == 7 { 8 } else { 16 };
    let cols = width.div_ceil(4) as usize;
    let rows = height.div_ceil(4) as usize;
    if cols
        .checked_mul(rows)
        .and_then(|n| n.checked_mul(block_size))
        != Some(data.len())
    {
        return Err(invalid("Invalid block texture size"));
    }
    let mut out = vec![0; width as usize * height as usize * 4];
    for by in 0..rows {
        for bx in 0..cols {
            let block = &data[(by * cols + bx) * block_size..][..block_size];
            let color = if format == 7 { &block[..] } else { &block[8..] };
            let c0 = u16::from_le_bytes([color[0], color[1]]);
            let c1 = u16::from_le_bytes([color[2], color[3]]);
            let a = rgb565(c0);
            let b = rgb565(c1);
            let mut palette = [a, b, [0; 4], [0; 4]];
            for k in 0..3 {
                if c0 > c1 || format != 7 {
                    palette[2][k] = ((2 * a[k] as u16 + b[k] as u16) / 3) as u8;
                    palette[3][k] = ((a[k] as u16 + 2 * b[k] as u16) / 3) as u8;
                } else {
                    palette[2][k] = ((a[k] as u16 + b[k] as u16) / 2) as u8;
                }
            }
            palette[2][3] = 255;
            palette[3][3] = if c0 > c1 || format != 7 { 255 } else { 0 };
            let index = u32::from_le_bytes(color[4..8].try_into().unwrap());
            let mut alphas = [255u8; 16];
            if format == 6 {
                let bits = u64::from_le_bytes(block[..8].try_into().unwrap());
                for (i, v) in alphas.iter_mut().enumerate() {
                    *v = (((bits >> (4 * i)) & 15) * 17) as u8;
                }
            }
            if format == 4 {
                let a0 = block[0];
                let a1 = block[1];
                let mut ap = [0u8; 8];
                ap[0] = a0;
                ap[1] = a1;
                if a0 > a1 {
                    for n in 1..=6 {
                        ap[n + 1] = (((7 - n) as u16 * a0 as u16 + n as u16 * a1 as u16) / 7) as u8;
                    }
                } else {
                    for n in 1..=4 {
                        ap[n + 1] = (((5 - n) as u16 * a0 as u16 + n as u16 * a1 as u16) / 5) as u8;
                    }
                    ap[6] = 0;
                    ap[7] = 255;
                }
                let mut bits = 0u64;
                for (n, v) in block[2..8].iter().enumerate() {
                    bits |= (*v as u64) << (8 * n);
                }
                for (n, v) in alphas.iter_mut().enumerate() {
                    *v = ap[((bits >> (3 * n)) & 7) as usize];
                }
            }
            for py in 0..4 {
                for px in 0..4 {
                    let x = bx * 4 + px;
                    let y = by * 4 + py;
                    if x >= width as usize || y >= height as usize {
                        continue;
                    }
                    let n = py * 4 + px;
                    let mut rgba = palette[((index >> (2 * n)) & 3) as usize];
                    if format != 7 {
                        rgba[3] = alphas[n];
                    }
                    let p = (y * width as usize + x) * 4;
                    out[p..p + 4].copy_from_slice(&rgba);
                }
            }
        }
    }
    Ok(out)
}
pub fn decode(data: &[u8]) -> io::Result<Texture> {
    let mut c = Cursor::new(data);
    if &tag(&mut c)? != b"TEXV0005" || &tag(&mut c)? != b"TEXI0001" {
        return Err(invalid("Unsupported texture version"));
    }
    let format = num(&mut c)?;
    let flags = num(&mut c)?;
    let tw = num(&mut c)?;
    let th = num(&mut c)?;
    let width = num(&mut c)?;
    let height = num(&mut c)?;
    let _ = num(&mut c)?;
    if tw == 0
        || th == 0
        || width == 0
        || height == 0
        || width > tw
        || height > th
        || u64::from(tw) * u64::from(th) > MAX_PIXELS
    {
        return Err(invalid("Invalid texture dimensions"));
    }
    let version = tag(&mut c)?;
    let images = num(&mut c)?;
    if images == 0 || images > 4096 {
        return Err(invalid("Invalid texture image count"));
    }
    let v = match &version {
        b"TEXB0001" => 1,
        b"TEXB0002" => 2,
        b"TEXB0003" => 3,
        b"TEXB0004" => 4,
        _ => return Err(invalid("Unsupported texture container")),
    };
    if v >= 3 && num(&mut c)? != u32::MAX {
        return Err(invalid("Embedded image textures are not supported"));
    }
    if v == 4 && num(&mut c)? != 0 {
        return Err(invalid("Embedded video textures are not supported"));
    }
    let mipmaps = num(&mut c)?;
    if mipmaps == 0 || mipmaps > 32 {
        return Err(invalid("Invalid texture mipmap count"));
    }
    let mw = num(&mut c)?;
    let mh = num(&mut c)?;
    if mw != tw || mh != th {
        return Err(invalid("Unexpected texture mipmap dimensions"));
    }
    let (compressed, unpacked) = if v >= 2 {
        (num(&mut c)?, num(&mut c)? as usize)
    } else {
        (0, 0)
    };
    if compressed > 1 {
        return Err(invalid("Unknown texture compression"));
    }
    let size = num(&mut c)? as usize;
    if size > MAX_BYTES {
        return Err(invalid("Texture data is too large"));
    }
    let start = c.position() as usize;
    let end = start
        .checked_add(size)
        .filter(|&v| v <= data.len())
        .ok_or_else(|| invalid("Truncated texture data"))?;
    let raw = if compressed == 1 {
        decompress(&data[start..end], unpacked)?
    } else {
        data[start..end].to_vec()
    };
    let pixels = tw as usize * th as usize;
    let rgba = match format {
        0 if raw.len() == pixels * 4 => raw,
        8 if raw.len() == pixels * 2 => raw
            .chunks_exact(2)
            .flat_map(|p| [p[0], p[1], 0, 255])
            .collect(),
        9 if raw.len() == pixels => raw.iter().flat_map(|&p| [p, p, p, 255]).collect(),
        4 | 6 | 7 => blocks(format, tw, th, &raw)?,
        _ => return Err(invalid("Unsupported or malformed texture format")),
    };
    let mut cropped = Vec::with_capacity(width as usize * height as usize * 4);
    for row in 0..height as usize {
        let start = row * tw as usize * 4;
        cropped.extend_from_slice(&rgba[start..start + width as usize * 4]);
    }
    Ok(Texture {
        width,
        height,
        flags,
        rgba: cropped,
    })
}

pub fn embedded_image(data: &[u8]) -> io::Result<Option<(&'static str, Vec<u8>)>> {
    let mut c = Cursor::new(data);
    if &tag(&mut c)? != b"TEXV0005" || &tag(&mut c)? != b"TEXI0001" {
        return Err(invalid("Unsupported texture version"));
    }
    for _ in 0..7 {
        num(&mut c)?;
    }
    let version = tag(&mut c)?;
    if &version != b"TEXB0003" && &version != b"TEXB0004" {
        return Ok(None);
    }
    if num(&mut c)? != 1 {
        return Err(invalid("Embedded texture arrays are not supported"));
    }
    let format = num(&mut c)?;
    if &version == b"TEXB0004" && num(&mut c)? != 0 {
        return Err(invalid("Embedded video textures are not supported"));
    }
    if format == u32::MAX {
        return Ok(None);
    }
    let mips = num(&mut c)?;
    if !(1..=32).contains(&mips) {
        return Err(invalid("Invalid embedded mip count"));
    }
    let width = num(&mut c)?;
    let height = num(&mut c)?;
    if width == 0 || height == 0 || u64::from(width) * u64::from(height) > MAX_PIXELS {
        return Err(invalid("Invalid embedded image dimensions"));
    }
    let compressed = num(&mut c)?;
    let unpacked = num(&mut c)? as usize;
    let size = num(&mut c)? as usize;
    if size > MAX_BYTES || compressed > 1 {
        return Err(invalid("Invalid embedded image size"));
    }
    let start = c.position() as usize;
    let end = start
        .checked_add(size)
        .filter(|n| *n <= data.len())
        .ok_or_else(|| invalid("Truncated embedded image"))?;
    let bytes = if compressed == 1 {
        decompress(&data[start..end], unpacked)?
    } else {
        data[start..end].to_vec()
    };
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Ok(Some(("png", bytes)))
    } else if bytes.starts_with(b"\xff\xd8\xff") {
        Ok(Some(("jpg", bytes)))
    } else {
        Err(invalid("Unsupported embedded image format"))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(format: u32, compressed: bool, bytes: &[u8], expected: u32) -> Vec<u8> {
        let mut d = Vec::new();
        d.extend_from_slice(b"TEXV0005\0TEXI0001\0");
        for n in [format, 0, 2, 2, 1, 2, 0] {
            d.extend_from_slice(&n.to_le_bytes());
        }
        d.extend_from_slice(b"TEXB0002\0");
        for n in [
            1u32,
            1,
            2,
            2,
            compressed as u32,
            expected,
            bytes.len() as u32,
        ] {
            d.extend_from_slice(&n.to_le_bytes());
        }
        d.extend_from_slice(bytes);
        d
    }
    #[test]
    fn decodes_and_crops_raw_rgba() {
        let raw = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
        let t = decode(&fixture(0, false, &raw, 16)).unwrap();
        assert_eq!((t.width, t.height), (1, 2));
        assert_eq!(t.rgba, [1, 2, 3, 4, 9, 10, 11, 12]);
    }
    #[test]
    fn decodes_v4_extra_header_and_rejects_truncation() {
        let raw = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
        let mut v4 = fixture(0, false, &raw, 16);
        v4[53] = b'4';
        v4.splice(
            59..59,
            [u32::MAX.to_le_bytes(), 0u32.to_le_bytes()].concat(),
        );
        assert!(embedded_image(&v4).unwrap().is_none());
        assert_eq!(decode(&v4).unwrap().rgba, [1, 2, 3, 4, 9, 10, 11, 12]);
        assert!(decode(&v4[..65]).is_err());
    }
    #[test]
    fn decodes_lz4_and_rejects_truncation() {
        let raw = [0x48, 8, 9, 10, 11, 4, 0];
        let t = decode(&fixture(0, true, &raw, 16)).unwrap();
        assert_eq!(t.rgba, [8, 9, 10, 11, 8, 9, 10, 11]);
        assert_eq!(
            decompress(&[0x40, 1, 2, 3, 4, 4, 0], 8).unwrap(),
            [1, 2, 3, 4, 1, 2, 3, 4]
        );
        assert!(decompress(&[0, 0, 0], 4).is_err());
    }
    #[test]
    fn decodes_dxt1() {
        let block = [0, 0xf8, 0, 0, 0, 0, 0, 0];
        let image = blocks(7, 2, 2, &block).unwrap();
        assert_eq!(&image[..4], &[255, 0, 0, 255]);
        assert!(blocks(7, 2, 2, &block[..7]).is_err());
    }
    #[test]
    fn decodes_dxt3_and_dxt5_alpha() {
        let mut dxt3 = [0u8; 16];
        dxt3[0] = 0xf8;
        dxt3[9] = 0xf8;
        let result = blocks(6, 4, 4, &dxt3).unwrap();
        assert_eq!(&result[..8], &[255, 0, 0, 136, 255, 0, 0, 255]);

        let mut dxt5 = [0u8; 16];
        dxt5[0] = 200;
        dxt5[1] = 100;
        dxt5[2] = 0b0000_1010;
        dxt5[9] = 0xf8;
        let result = blocks(4, 4, 4, &dxt5).unwrap();
        assert_eq!(&result[..8], &[255, 0, 0, 185, 255, 0, 0, 100]);
    }
}
