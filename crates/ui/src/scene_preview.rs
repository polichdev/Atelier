use atelier_wallpaper::{
    package::Package,
    texture::{self, Texture},
};
use serde_json::Value;
use std::{fs::File, io::Cursor, path::Path};

fn json(package: &Package, file: &mut File, name: &str) -> Result<Value, String> {
    serde_json::from_slice(
        &package
            .read_entry(file, name, 4 * 1024 * 1024)
            .map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}
fn vector(value: &Value, default: &[f64]) -> Vec<f64> {
    let value = value.get("value").unwrap_or(value);
    let numbers: Vec<f64> = value
        .as_str()
        .map(|s| {
            s.split_whitespace()
                .filter_map(|n| n.parse().ok())
                .collect()
        })
        .unwrap_or_default();
    default
        .iter()
        .enumerate()
        .map(|(i, n)| {
            numbers
                .get(i)
                .copied()
                .filter(|v| v.is_finite())
                .unwrap_or(*n)
        })
        .collect()
}
fn image(bytes: &[u8]) -> Result<Texture, String> {
    if let Some((ext, bytes)) = texture::embedded_image(bytes).map_err(|e| e.to_string())? {
        if ext != "png" {
            return Err("Preview uses the author's image for this image format".into());
        }
        let mut decoder = png::Decoder::new(Cursor::new(bytes));
        decoder.set_transformations(png::Transformations::normalize_to_color8());
        decoder.set_limits(png::Limits {
            bytes: 64 * 1024 * 1024,
        });
        let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
        let size = reader
            .output_buffer_size()
            .filter(|n| *n <= 64 * 1024 * 1024)
            .ok_or("Preview texture exceeds memory limit")?;
        let mut bytes = vec![0; size];
        let frame = reader.next_frame(&mut bytes).map_err(|e| e.to_string())?;
        let data = &bytes[..frame.buffer_size()];
        let rgba = match frame.color_type {
            png::ColorType::Rgba => data.to_vec(),
            png::ColorType::Rgb => data
                .chunks_exact(3)
                .flat_map(|p| [p[0], p[1], p[2], 255])
                .collect(),
            png::ColorType::Grayscale => data.iter().flat_map(|p| [*p, *p, *p, 255]).collect(),
            png::ColorType::GrayscaleAlpha => data
                .chunks_exact(2)
                .flat_map(|p| [p[0], p[0], p[0], p[1]])
                .collect(),
            _ => return Err("Unsupported preview color format".into()),
        };
        Ok(Texture {
            width: frame.width,
            height: frame.height,
            flags: 0,
            rgba,
        })
    } else {
        texture::decode(bytes).map_err(|e| e.to_string())
    }
}

fn blend(
    output: &mut [u8],
    output_index: usize,
    texture: &Texture,
    u: f64,
    v: f64,
    alpha: f64,
    color: &[f64],
    additive: bool,
) {
    let tx = (u * texture.width as f64 - 0.5).clamp(0., (texture.width - 1) as f64);
    let ty = (v * texture.height as f64 - 0.5).clamp(0., (texture.height - 1) as f64);
    let (x0, y0) = (tx.floor() as u32, ty.floor() as u32);
    let (x1, y1) = (
        (x0 + 1).min(texture.width - 1),
        (y0 + 1).min(texture.height - 1),
    );
    let (fx, fy) = (tx - x0 as f64, ty - y0 as f64);
    let mut pixel = [0.; 4];
    for (xx, yy, weight) in [
        (x0, y0, (1. - fx) * (1. - fy)),
        (x1, y0, fx * (1. - fy)),
        (x0, y1, (1. - fx) * fy),
        (x1, y1, fx * fy),
    ] {
        let index = ((yy * texture.width + xx) * 4) as usize;
        for c in 0..4 {
            pixel[c] += texture.rgba[index + c] as f64 * weight
        }
    }
    let a = pixel[3] / 255. * alpha;
    let index = output_index;
    for c in 0..3 {
        output[index + c] = (pixel[c] * color[c] * a
            + output[index + c] as f64 * if additive { 1. } else { 1. - a })
        .clamp(0., 255.) as u8
    }
}

fn draw(
    output: &mut [u8],
    width: u32,
    height: u32,
    scene_width: f64,
    scene_height: f64,
    texture: &Texture,
    object: &Value,
    additive: bool,
) -> Result<(), String> {
    let size = vector(
        &object["size"],
        &[texture.width as f64, texture.height as f64],
    );
    let scale = vector(&object["scale"], &[1., 1.]);
    let origin = vector(&object["origin"], &[scene_width / 2., scene_height / 2.]);
    let angle = vector(&object["angles"], &[0., 0., 0.])[2];
    let (cos, sin) = (angle.cos(), angle.sin());
    let alpha = object["alpha"]
        .get("value")
        .unwrap_or(&object["alpha"])
        .as_f64()
        .unwrap_or(1.)
        .clamp(0., 1.);
    let color = vector(&object["color"], &[1., 1., 1.]);
    let fallback = [
        cos * scale[0],
        sin * scale[0],
        -sin * scale[1],
        cos * scale[1],
        origin[0],
        origin[1],
    ];
    let matrix: Vec<f64> = object["atelier_matrix"]
        .as_array()
        .filter(|a| a.len() == 6)
        .map(|a| a.iter().map(|n| n.as_f64().unwrap_or(0.)).collect())
        .unwrap_or_else(|| fallback.to_vec());
    let determinant = matrix[0] * matrix[3] - matrix[1] * matrix[2];
    if determinant.abs() < 0.0001 || size[0].abs() < 0.0001 || size[1].abs() < 0.0001 {
        return Ok(());
    }
    if let Some(mesh) = object["atelier_mesh"].as_array() {
        return draw_mesh(
            output,
            width,
            height,
            scene_width,
            scene_height,
            texture,
            mesh,
            &matrix,
            alpha,
            &color,
            additive,
        );
    }
    let sx = width as f64 / scene_width;
    let sy = height as f64 / scene_height;
    let bound_x = (size[0] * matrix[0]).abs() / 2. + (size[1] * matrix[2]).abs() / 2.;
    let bound_y = (size[0] * matrix[1]).abs() / 2. + (size[1] * matrix[3]).abs() / 2.;
    let left = ((matrix[4] - bound_x) * sx).floor().max(0.) as u32;
    let right = ((matrix[4] + bound_x) * sx).ceil().clamp(0., width as f64) as u32;
    let top = ((scene_height - matrix[5] - bound_y) * sy).floor().max(0.) as u32;
    let bottom = ((scene_height - matrix[5] + bound_y) * sy)
        .ceil()
        .clamp(0., height as f64) as u32;
    for y in top..bottom {
        for x in left..right {
            let dx = (x as f64 + 0.5) / sx - matrix[4];
            let dy = scene_height - (y as f64 + 0.5) / sy - matrix[5];
            let u = (matrix[3] * dx - matrix[2] * dy) / determinant / size[0] + 0.5;
            let v = 0.5 - (-matrix[1] * dx + matrix[0] * dy) / determinant / size[1];
            if !(0.0..=1.).contains(&u) || !(0.0..=1.).contains(&v) {
                continue;
            }
            blend(
                output,
                ((y * width + x) * 4) as usize,
                texture,
                u,
                v,
                alpha,
                &color,
                additive,
            );
        }
    }
    Ok(())
}

fn draw_mesh(
    output: &mut [u8],
    width: u32,
    height: u32,
    scene_width: f64,
    scene_height: f64,
    texture: &Texture,
    mesh: &[Value],
    matrix: &[f64],
    alpha: f64,
    color: &[f64],
    additive: bool,
) -> Result<(), String> {
    let sx = width as f64 / scene_width;
    let sy = height as f64 / scene_height;
    let mut work = 0u64;
    for triangle in mesh.chunks_exact(3) {
        let points: Vec<[f64; 4]> = triangle
            .iter()
            .map(|v| {
                let x = v[0].as_f64().unwrap_or(0.);
                let y = v[1].as_f64().unwrap_or(0.);
                [
                    (matrix[0] * x + matrix[2] * y + matrix[4]) * sx,
                    (scene_height - matrix[1] * x - matrix[3] * y - matrix[5]) * sy,
                    v[2].as_f64().unwrap_or(0.),
                    v[3].as_f64().unwrap_or(0.),
                ]
            })
            .collect();
        let [mut a, mut b, c] = [points[0], points[1], points[2]];
        let edge = |a: [f64; 4], b: [f64; 4], x: f64, y: f64| {
            (b[0] - a[0]) * (y - a[1]) - (b[1] - a[1]) * (x - a[0])
        };
        let mut area = edge(a, b, c[0], c[1]);
        if area.abs() < 0.000001 {
            continue;
        }
        if area < 0. {
            std::mem::swap(&mut a, &mut b);
            area = -area;
        }
        let top_left = |a: [f64; 4], b: [f64; 4]| b[1] < a[1] || (b[1] == a[1] && b[0] > a[0]);
        let edges = [top_left(b, c), top_left(c, a), top_left(a, b)];
        let minx = a[0].min(b[0]).min(c[0]).floor().clamp(0., width as f64) as u32;
        let maxx = a[0].max(b[0]).max(c[0]).ceil().clamp(0., width as f64) as u32;
        let miny = a[1].min(b[1]).min(c[1]).floor().clamp(0., height as f64) as u32;
        let maxy = a[1].max(b[1]).max(c[1]).ceil().clamp(0., height as f64) as u32;
        work += (maxx - minx) as u64 * (maxy - miny) as u64;
        if work > 32 * 1024 * 1024 {
            return Err("Preview mesh exceeds rendering budget".into());
        }
        for y in miny..maxy {
            for x in minx..maxx {
                let px = x as f64 + 0.5;
                let py = y as f64 + 0.5;
                let weights = [
                    edge(b, c, px, py) / area,
                    edge(c, a, px, py) / area,
                    edge(a, b, px, py) / area,
                ];
                if weights
                    .iter()
                    .enumerate()
                    .any(|(i, w)| *w < 0. || (*w == 0. && !edges[i]))
                {
                    continue;
                }
                let u = weights[0] * a[2] + weights[1] * b[2] + weights[2] * c[2];
                let v = weights[0] * a[3] + weights[1] * b[3] + weights[2] * c[3];
                blend(
                    output,
                    ((y * width + x) * 4) as usize,
                    texture,
                    u,
                    v,
                    alpha,
                    color,
                    additive,
                );
            }
        }
    }
    Ok(())
}

pub fn render(source: &Path) -> Result<Vec<u8>, String> {
    let mut file = File::open(source).map_err(|e| e.to_string())?;
    let package = Package::read(&mut file).map_err(|e| e.to_string())?;
    let scene = json(&package, &mut file, "scene.json")?;
    let width = scene["general"]["orthogonalprojection"]["width"]
        .as_f64()
        .unwrap_or(1920.);
    let height = scene["general"]["orthogonalprojection"]["height"]
        .as_f64()
        .unwrap_or(1080.);
    if !(1.0..=16384.).contains(&width) || !(1.0..=16384.).contains(&height) {
        return Err("Invalid scene dimensions".into());
    }
    let ratio = 1f64.min(1920. / width).min(1080. / height);
    let (w, h) = (
        (width * ratio).round().max(1.) as u32,
        (height * ratio).round().max(1.) as u32,
    );
    let background = vector(&scene["general"]["clearcolor"], &[0., 0., 0.]);
    let mut output = vec![0; (w * h * 4) as usize];
    for pixel in output.chunks_exact_mut(4) {
        for c in 0..3 {
            pixel[c] = (background[c] * 255.).clamp(0., 255.) as u8
        }
        pixel[3] = 255
    }
    let mut count = 0;
    let mut decoded = 0usize;
    for object in &super::scene_geometry::load(&scene, &package, &mut file)? {
        if object["visible"]
            .get("value")
            .unwrap_or(&object["visible"])
            .as_bool()
            == Some(false)
        {
            continue;
        }
        let Some(model) = object["image"]
            .as_str()
            .filter(|s| !s.starts_with("models/util/"))
        else {
            continue;
        };
        count += 1;
        if count > 128 {
            return Err("Preview layer limit exceeded".into());
        }
        let model = json(&package, &mut file, model)?;
        if model["puppet"].is_string() && !object["atelier_mesh"].is_array() {
            return Err("Preview uses the author's image for unsupported models".into());
        }
        let material = json(
            &package,
            &mut file,
            model["material"]
                .as_str()
                .ok_or("Missing preview material")?,
        )?;
        let pass = &material["passes"][0];
        let name = pass["textures"][0]
            .as_str()
            .ok_or("Missing preview texture")?;
        let name = if name.ends_with(".tex") {
            name.to_string()
        } else {
            format!("materials/{name}.tex")
        };
        let data = package
            .read_entry(&mut file, &name, 64 * 1024 * 1024)
            .map_err(|e| e.to_string())?;
        let texture = image(&data)?;
        decoded += texture.rgba.len();
        if decoded > 512 * 1024 * 1024 {
            return Err("Preview decode budget exceeded".into());
        }
        draw(
            &mut output,
            w,
            h,
            width,
            height,
            &texture,
            object,
            pass["blending"].as_str() == Some("additive"),
        )?;
    }
    if count == 0 {
        return Err("No image layers available for preview".into());
    }
    let mut png = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png, w, h);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .map_err(|e| e.to_string())?
            .write_image_data(&output)
            .map_err(|e| e.to_string())?;
    }
    Ok(png)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn composes_full_size_layers_with_alpha() {
        let texture = Texture {
            width: 2,
            height: 2,
            flags: 0,
            rgba: [255, 0, 0, 255].repeat(4),
        };
        let mut output = [0, 0, 255, 255].repeat(4);
        draw(
            &mut output,
            2,
            2,
            2.,
            2.,
            &texture,
            &json!({"size":"2 2","origin":"1 1 0","alpha":0.5}),
            false,
        )
        .unwrap();
        for p in output.chunks_exact(4) {
            assert_eq!(p, &[127, 0, 127, 255]);
        }
    }
    #[test]
    fn mesh_shares_edges_without_double_blending_or_gaps() {
        let texture = Texture {
            width: 2,
            height: 2,
            flags: 0,
            rgba: [255, 0, 0, 255].repeat(4),
        };
        let mut output = [0, 0, 255, 255].repeat(4);
        let mesh = json!([
            [0, 2, 0, 0],
            [2, 2, 1, 0],
            [0, 0, 0, 1],
            [0, 0, 0, 1],
            [2, 2, 1, 0],
            [2, 0, 1, 1]
        ]);
        draw_mesh(
            &mut output,
            2,
            2,
            2.,
            2.,
            &texture,
            mesh.as_array().unwrap(),
            &[1., 0., 0., 1., 0., 0.],
            0.5,
            &[1., 1., 1.],
            false,
        )
        .unwrap();
        for pixel in output.chunks_exact(4) {
            assert_eq!(pixel, &[127, 0, 127, 255]);
        }
    }
}
