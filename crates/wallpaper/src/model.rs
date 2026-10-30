use std::{collections::BTreeMap, io};

pub type Matrix = [f64; 6];
pub fn multiply(p: Matrix, m: Matrix) -> Matrix {
    [
        p[0] * m[0] + p[2] * m[1],
        p[1] * m[0] + p[3] * m[1],
        p[0] * m[2] + p[2] * m[3],
        p[1] * m[2] + p[3] * m[3],
        p[0] * m[4] + p[2] * m[5] + p[4],
        p[1] * m[4] + p[3] * m[5] + p[5],
    ]
}
#[derive(Debug)]
pub struct Model {
    pub triangles: Vec<[f64; 4]>,
    pub attachments: BTreeMap<String, Matrix>,
}
fn invalid() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "Unsupported or invalid 2D model",
    )
}
struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}
impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> io::Result<&'a [u8]> {
        let end = self.at.checked_add(n).ok_or_else(invalid)?;
        let b = self.bytes.get(self.at..end).ok_or_else(invalid)?;
        self.at = end;
        Ok(b)
    }
    fn u8(&mut self) -> io::Result<u8> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> io::Result<u16> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap()))
    }
    fn u32(&mut self) -> io::Result<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn count(&mut self, limit: usize) -> io::Result<usize> {
        let n = self.u32()? as usize;
        if n > limit {
            return Err(invalid());
        }
        Ok(n)
    }
    fn f32(&mut self) -> io::Result<f64> {
        let n = f32::from_le_bytes(self.take(4)?.try_into().unwrap()) as f64;
        if !n.is_finite() || n.abs() > 1e6 {
            return Err(invalid());
        }
        Ok(n)
    }
    fn string(&mut self) -> io::Result<String> {
        let end = self.bytes[self.at..]
            .iter()
            .take(4096)
            .position(|b| *b == 0)
            .ok_or_else(invalid)?;
        let s = std::str::from_utf8(self.take(end)?)
            .map_err(|_| invalid())?
            .to_owned();
        self.take(1)?;
        Ok(s)
    }
    fn matrix(&mut self) -> io::Result<Matrix> {
        let mut a = [0.; 16];
        for n in &mut a {
            *n = self.f32()?
        }
        if [a[2], a[3], a[6], a[7], a[8], a[9], a[11], a[14]]
            .iter()
            .any(|v| v.abs() > 0.001)
            || (a[10] - 1.).abs() > 0.001
            || (a[15] - 1.).abs() > 0.001
        {
            return Err(invalid());
        }
        Ok([a[0], a[1], a[4], a[5], a[12], a[13]])
    }
}
pub fn decode(bytes: &[u8]) -> io::Result<Model> {
    if bytes.len() > 16 * 1024 * 1024 {
        return Err(invalid());
    }
    let mut r = Reader { bytes, at: 0 };
    if r.take(9)? != b"MDLV0023\0" {
        return Err(invalid());
    }
    r.u32()?;
    if r.u32()? != 1 || r.u32()? != 1 {
        return Err(invalid());
    }
    r.string()?;
    if r.u32()? == 2 {
        r.u32()?;
    }
    r.take(24)?;
    let flags = r.u32()?;
    if flags & !0x0181002f != 0 || flags & 8 == 0 {
        return Err(invalid());
    }
    let stride = 12
        + if flags & 2 != 0 { 12 } else { 0 }
        + if flags & 4 != 0 { 16 } else { 0 }
        + if flags & 0x10000 != 0 { 4 } else { 0 }
        + if flags & 0x800000 != 0 { 16 } else { 0 }
        + if flags & 0x1000000 != 0 { 16 } else { 0 }
        + 8
        + if flags & 32 != 0 { 8 } else { 0 };
    let size = r.count(8 * 1024 * 1024)?;
    if size % stride != 0 || size / stride > 100_000 {
        return Err(invalid());
    }
    let mut vertices = Vec::with_capacity(size / stride);
    for _ in 0..size / stride {
        let start = r.at;
        let x = r.f32()?;
        let y = r.f32()?;
        let z = r.f32()?;
        if z.abs() > 0.01 {
            return Err(invalid());
        }
        r.take(stride - 12 - if flags & 32 != 0 { 16 } else { 8 })?;
        let u = r.f32()?;
        let v = r.f32()?;
        if flags & 32 != 0 {
            r.take(8)?;
        }
        debug_assert_eq!(r.at - start, stride);
        vertices.push([x, y, u, v]);
    }
    let index_bytes = r.count(2 * 1024 * 1024)?;
    let index_size = if vertices.len() > u16::MAX as usize {
        4
    } else {
        2
    };
    if index_bytes % (3 * index_size) != 0 || index_bytes / index_size > 300_000 {
        return Err(invalid());
    }
    let mut triangles = Vec::with_capacity(index_bytes / index_size);
    for _ in 0..index_bytes / index_size {
        let i = if index_size == 4 {
            r.u32()? as usize
        } else {
            r.u16()? as usize
        };
        triangles.push(*vertices.get(i).ok_or_else(invalid)?);
    }
    match r.u8()? {
        0 => (),
        1 => {
            if r.u8()? != 0 {
                r.take(3)?;
                let n = r.count(8 * 1024 * 1024)?;
                r.take(n)?;
            }
        }
        _ => return Err(invalid()),
    }
    if r.u8()? != 0 {
        let n = r.count(1024 * 1024)?;
        if n % 16 != 0 {
            return Err(invalid());
        }
        r.take(n)?;
    }
    for _ in 0..r.count(128)? {
        r.take(8)?;
        r.string()?;
        r.u32()?;
        let n = r.count(100_000)?;
        r.take(n * 4)?;
        let n = r.count(100_000)?;
        r.take(n * 4)?;
    }
    let mut attachments = BTreeMap::new();
    if r.bytes.get(r.at..r.at + 4) == Some(b"MDLS") {
        let tag = r.take(9)?;
        if ![b"MDLS0002\0", b"MDLS0003\0", b"MDLS0004\0"]
            .iter()
            .any(|t| t.as_slice() == tag)
        {
            return Err(invalid());
        }
        let end = r.count(bytes.len())?;
        let count = r.u16()? as usize;
        r.u16()?;
        if count > 1024 {
            return Err(invalid());
        }
        let mut bones = Vec::with_capacity(count);
        for i in 0..count {
            r.string()?;
            r.u32()?;
            let parent = r.u32()?;
            if r.u32()? != 64 {
                return Err(invalid());
            }
            let local = r.matrix()?;
            let world = if parent == u32::MAX {
                local
            } else {
                if parent as usize >= i {
                    return Err(invalid());
                }
                multiply(bones[parent as usize], local)
            };
            bones.push(world);
            r.string()?;
            if r.at > end {
                return Err(invalid());
            }
        }
        if end < r.at {
            return Err(invalid());
        }
        r.at = end;
        if r.bytes.get(r.at..r.at + 9) == Some(b"MDAT0001\0") {
            r.take(9)?;
            let end = r.count(bytes.len())?;
            let count = r.u16()? as usize;
            if count > 1024 {
                return Err(invalid());
            }
            for _ in 0..count {
                let bone = r.u16()? as usize;
                let name = r.string()?;
                let local = r.matrix()?;
                attachments.insert(name, multiply(*bones.get(bone).ok_or_else(invalid)?, local));
                if r.at > end {
                    return Err(invalid());
                }
            }
        }
    }
    if triangles.is_empty() {
        return Err(invalid());
    }
    Ok(Model {
        triangles,
        attachments,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Vec<u8> {
        fn u32(b: &mut Vec<u8>, n: u32) {
            b.extend(n.to_le_bytes())
        }
        fn matrix(b: &mut Vec<u8>, x: f32, y: f32) {
            for n in [1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., x, y, 0., 1.] {
                b.extend(n.to_le_bytes());
            }
        }
        let mut b = b"MDLV0023\0".to_vec();
        for n in [8, 1, 1] {
            u32(&mut b, n)
        }
        b.extend(b"materials/test.json\0");
        u32(&mut b, 0);
        b.extend([0; 24]);
        u32(&mut b, 8);
        u32(&mut b, 60);
        for n in [0f32, 0., 0., 0., 0., 1., 0., 0., 1., 0., 0., 1., 0., 0., 1.] {
            b.extend(n.to_le_bytes())
        }
        u32(&mut b, 6);
        for n in [0u16, 1, 2] {
            b.extend(n.to_le_bytes())
        }
        b.extend([0, 0]);
        u32(&mut b, 0);
        b.extend(b"MDLS0004\0");
        let end = b.len();
        u32(&mut b, 0);
        b.extend([1, 0, 0, 0]);
        b.push(0);
        u32(&mut b, 0);
        u32(&mut b, u32::MAX);
        u32(&mut b, 64);
        matrix(&mut b, 3., 4.);
        b.push(0);
        let len = b.len() as u32;
        b[end..end + 4].copy_from_slice(&len.to_le_bytes());
        b.extend(b"MDAT0001\0");
        let end = b.len();
        u32(&mut b, 0);
        b.extend([1, 0, 0, 0]);
        b.extend(b"head\0");
        matrix(&mut b, 5., 6.);
        let len = b.len() as u32;
        b[end..end + 4].copy_from_slice(&len.to_le_bytes());
        b
    }
    #[test]
    fn reads_mesh_and_composes_attachment_bind_pose() {
        let bytes = fixture();
        let model = decode(&bytes).unwrap();
        assert_eq!(
            model.triangles,
            vec![[0., 0., 0., 0.], [1., 0., 1., 0.], [0., 1., 0., 1.]]
        );
        assert_eq!(model.attachments["head"], [1., 0., 0., 1., 8., 10.]);
        assert!(decode(&bytes[..bytes.len() - 1]).is_err());
    }
    #[test]
    fn rejects_truncated_or_unknown_model() {
        assert!(decode(b"MDLV0023\0").is_err());
        assert!(decode(b"MDLV0021\0").is_err());
        assert_eq!(
            multiply([1., 0., 0., 1., 0., 0.], [2., 0., 0., 3., 4., 5.]),
            [2., 0., 0., 3., 4., 5.]
        );
    }
}
