use atelier_wallpaper::{model, package::Package};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs::File;
type Matrix = [f64; 6];
fn vector(v: &Value, default: &[f64]) -> Vec<f64> {
    let v = v.get("value").unwrap_or(v);
    let n: Vec<f64> = v
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
        .map(|(i, d)| n.get(i).copied().filter(|n| n.is_finite()).unwrap_or(*d))
        .collect()
}
fn multiply(p: Matrix, m: Matrix) -> Matrix {
    [
        p[0] * m[0] + p[2] * m[1],
        p[1] * m[0] + p[3] * m[1],
        p[0] * m[2] + p[2] * m[3],
        p[1] * m[2] + p[3] * m[3],
        p[0] * m[4] + p[2] * m[5] + p[4],
        p[1] * m[4] + p[3] * m[5] + p[5],
    ]
}
pub fn load(scene: &Value, package: &Package, file: &mut File) -> Result<Vec<Value>, String> {
    let mut scene = scene.clone();
    let objects = scene["objects"]
        .as_array_mut()
        .ok_or("Missing scene objects")?;
    if objects.len() > 1024 {
        return Err("Too many scene objects".into());
    }
    let mut models = HashMap::<String, Value>::new();
    let mut vertices = 0;
    for object in objects {
        let Some(name) = object["image"]
            .as_str()
            .filter(|s| !s.starts_with("models/util/"))
        else {
            continue;
        };
        let descriptor: Value = serde_json::from_slice(
            &package
                .read_entry(file, name, 4 * 1024 * 1024)
                .map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        let Some(puppet) = descriptor["puppet"].as_str() else {
            continue;
        };
        if !models.contains_key(puppet) {
            let decoded = package
                .read_entry(file, puppet, 16 * 1024 * 1024)
                .ok()
                .and_then(|bytes| model::decode(&bytes).ok());
            let value = match decoded {
                Some(model) => json!({"triangles":model.triangles,"attachments":model.attachments}),
                None => Value::Null,
            };
            models.insert(puppet.to_owned(), value);
        }
        let model = &models[puppet];
        vertices += model["triangles"].as_array().map_or(0, Vec::len);
        if vertices > 300_000 {
            return Err("Scene model geometry exceeds memory limit".into());
        }
        object["atelier_puppet"] = json!(true);
        object["atelier_mesh"] = model["triangles"].clone();
        object["atelier_attachments"] = model["attachments"].clone();
    }
    flatten(&scene)
}
pub fn flatten(scene: &Value) -> Result<Vec<Value>, String> {
    let objects = scene["objects"].as_array().ok_or("Missing scene objects")?;
    if objects.len() > 1024 {
        return Err("Too many scene objects".into());
    }
    let index: HashMap<i64, usize> = objects
        .iter()
        .enumerate()
        .map(|(i, o)| (o["id"].as_i64().unwrap_or(-(i as i64) - 1), i))
        .collect();
    fn resolve(
        i: usize,
        objects: &[Value],
        index: &HashMap<i64, usize>,
        cache: &mut HashMap<usize, (Matrix, bool)>,
        visiting: &mut Vec<usize>,
        center: [f64; 2],
    ) -> Result<(Matrix, bool), String> {
        if let Some(result) = cache.get(&i) {
            return Ok(*result);
        }
        if visiting.len() >= 64 || visiting.contains(&i) {
            return Err("Invalid scene parent hierarchy".into());
        }
        visiting.push(i);
        let o = &objects[i];
        let parent = o["parent"].as_i64();
        let origin = vector(
            &o["origin"],
            if parent.is_some() { &[0., 0.] } else { &center },
        );
        let scale = vector(&o["scale"], &[1., 1.]);
        let angle = vector(&o["angles"], &[0., 0., 0.])[2];
        let (s, c) = angle.sin_cos();
        let local = [
            c * scale[0],
            s * scale[0],
            -s * scale[1],
            c * scale[1],
            origin[0],
            origin[1],
        ];
        let visible = o["visible"].get("value").unwrap_or(&o["visible"]).as_bool() != Some(false);
        let result = if let Some(parent) = parent {
            let p = *index.get(&parent).ok_or("Scene parent is missing")?;
            let (matrix, parent_visible) = resolve(p, objects, index, cache, visiting, center)?;
            let attachment = o["attachment"]
                .as_str()
                .and_then(|name| objects[p]["atelier_attachments"][name].as_array())
                .filter(|v| v.len() == 6);
            let matrix = if let Some(attachment) = attachment {
                multiply(
                    matrix,
                    std::array::from_fn(|i| attachment[i].as_f64().unwrap_or(0.)),
                )
            } else {
                matrix
            };
            (multiply(matrix, local), visible && parent_visible)
        } else {
            (local, visible)
        };
        visiting.pop();
        cache.insert(i, result);
        Ok(result)
    }
    let center = [
        scene["general"]["orthogonalprojection"]["width"]
            .as_f64()
            .unwrap_or(1920.)
            / 2.,
        scene["general"]["orthogonalprojection"]["height"]
            .as_f64()
            .unwrap_or(1080.)
            / 2.,
    ];
    let mut output = Vec::new();
    let mut cache = HashMap::new();
    for (i, object) in objects.iter().enumerate() {
        let (matrix, visible) = resolve(i, objects, &index, &mut cache, &mut Vec::new(), center)?;
        let mut object = object.clone();
        let mut visibility = Vec::new();
        let mut transforms = Vec::new();
        let mut node = i;
        loop {
            let raw = &objects[node];
            visibility.push(raw.get("visible").cloned().unwrap_or(json!(true)));
            let parent = raw["parent"]
                .as_i64()
                .and_then(|id| index.get(&id).copied());
            let attachment = parent
                .and_then(|p| {
                    raw["attachment"]
                        .as_str()
                        .and_then(|name| objects[p]["atelier_attachments"].get(name))
                })
                .cloned()
                .unwrap_or(json!([1, 0, 0, 1, 0, 0]));
            transforms.push(json!({"origin":raw.get("origin").cloned().unwrap_or(if parent.is_some(){json!("0 0")}else{json!(format!("{} {}",center[0],center[1]))}),"scale":raw.get("scale").cloned().unwrap_or(json!("1 1")),"angles":raw.get("angles").cloned().unwrap_or(json!("0 0 0")),"attachment":attachment}));
            if let Some(parent) = parent {
                node = parent
            } else {
                break;
            }
        }
        transforms.reverse();
        object["atelier_visibility"] = json!(visibility);
        object["atelier_transforms"] = json!(transforms);
        object["atelier_matrix"] = json!(matrix);
        object["visible"] = json!(visible);
        output.push(object)
    }
    Ok(output)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inherits_parent_position_scale_and_visibility() {
        let scene = json!({"objects":[{"id":1,"origin":"100 50 0","scale":"2 3 1"},{"id":2,"parent":1,"origin":"10 20 0"},{"id":3,"visible":false},{"id":4,"parent":3}]});
        let objects = flatten(&scene).unwrap();
        assert_eq!(
            objects[1]["atelier_matrix"],
            json!([2., 0., 0., 3., 120., 110.])
        );
        assert_eq!(objects[3]["visible"], false);
    }
    #[test]
    fn rejects_cycles() {
        assert!(flatten(&json!({"objects":[{"id":1,"parent":2},{"id":2,"parent":1}]})).is_err());
    }
    #[test]
    fn resolves_child_from_attachment_without_moving_parent() {
        let scene = json!({"objects":[{"id":1,"origin":"100 50","scale":"2 2","atelier_attachments":{"head":[1,0,0,1,5,10]}},{"id":2,"parent":1,"attachment":"head","origin":"3 4"}]});
        let objects = flatten(&scene).unwrap();
        assert_eq!(
            objects[0]["atelier_matrix"],
            json!([2., 0., 0., 2., 100., 50.])
        );
        assert_eq!(
            objects[1]["atelier_matrix"],
            json!([2., 0., 0., 2., 116., 78.])
        );
    }
}
