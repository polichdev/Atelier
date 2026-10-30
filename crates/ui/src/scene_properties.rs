use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, HashSet},
    fs,
    path::Path,
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SceneOptions {
    pub speed: f64,
    pub hue: f64,
    pub saturation: f64,
    pub brightness: f64,
    pub contrast: f64,
    pub tint: String,
    pub properties: BTreeMap<String, Value>,
}
impl Default for SceneOptions {
    fn default() -> Self {
        Self {
            speed: 1.,
            hue: 0.,
            saturation: 1.,
            brightness: 1.,
            contrast: 1.,
            tint: "1 1 1".into(),
            properties: BTreeMap::new(),
        }
    }
}
fn bounded(n: f64, default: f64, min: f64, max: f64) -> f64 {
    if n.is_finite() {
        n.clamp(min, max)
    } else {
        default
    }
}
pub fn color(v: &Value) -> Option<String> {
    let s = v.as_str()?;
    let c: Vec<f64> = s
        .split_whitespace()
        .map(str::parse)
        .collect::<Result<_, _>>()
        .ok()?;
    (c.len() == 3 && c.iter().all(|n| n.is_finite())).then(|| {
        c.iter()
            .map(|n| n.clamp(0., 1.).to_string())
            .collect::<Vec<_>>()
            .join(" ")
    })
}
impl SceneOptions {
    pub fn normalize(&mut self) {
        self.speed = bounded(self.speed, 1., 0., 3.);
        self.hue = bounded(self.hue, 0., -180., 180.);
        self.saturation = bounded(self.saturation, 1., 0., 2.);
        self.brightness = bounded(self.brightness, 1., 0., 2.);
        self.contrast = bounded(self.contrast, 1., 0., 2.);
        self.tint = color(&json!(self.tint)).unwrap_or_else(|| "1 1 1".into());
        self.properties
            .retain(|key, value| valid_key(key) && value.to_string().len() <= 1024);
        if self.properties.len() > 128 {
            self.properties.clear()
        }
    }
    pub fn validate(&mut self, schema: &[Value]) {
        self.normalize();
        self.properties.retain(|key, value| {
            let Some(property) = schema
                .iter()
                .find(|p| p["key"] == key.as_str() && p["supported"] == true)
            else {
                return false;
            };
            match property["type"].as_str().unwrap_or("") {
                "bool" => value.is_boolean(),
                "slider" => {
                    if let Some(n) = value.as_f64().filter(|v| v.is_finite()) {
                        *value = json!(n.clamp(
                            property["min"].as_f64().unwrap_or(0.),
                            property["max"].as_f64().unwrap_or(100.)
                        ));
                        true
                    } else {
                        false
                    }
                }
                "color" => {
                    if let Some(c) = color(value) {
                        *value = json!(c);
                        true
                    } else {
                        false
                    }
                }
                "combo" => property["options"]
                    .as_array()
                    .is_some_and(|a| a.iter().any(|p| p["value"] == *value)),
                _ => false,
            }
        });
    }
}
fn valid_key(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= 80
        && !["__proto__", "constructor", "prototype"].contains(&key)
        && key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}
pub fn binding_keys(value: &Value, keys: &mut HashSet<String>) {
    match value {
        Value::Object(o) => {
            if let Some(key) = o
                .get("user")
                .and_then(|v| v.as_str().or_else(|| v["name"].as_str()))
            {
                keys.insert(key.to_owned());
            }
            for (key, value) in o {
                if key != "user" {
                    binding_keys(value, keys)
                }
            }
        }
        Value::Array(a) => {
            for v in a {
                binding_keys(v, keys)
            }
        }
        _ => (),
    }
}
pub fn schema(source: &Path, supported: &Value) -> Vec<Value> {
    let Some(path) = source.parent().map(|p| p.join("project.json")) else {
        return Vec::new();
    };
    if fs::metadata(&path).map_or(true, |m| m.len() > 1024 * 1024) {
        return Vec::new();
    }
    let project: Value = fs::read(&path)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or(Value::Null);
    schema_from(&project, supported)
}
fn schema_from(project: &Value, supported: &Value) -> Vec<Value> {
    let mut used = HashSet::new();
    binding_keys(supported, &mut used);
    let mut output = Vec::new();
    for (key, p) in project["general"]["properties"]
        .as_object()
        .into_iter()
        .flatten()
        .take(128)
    {
        if !valid_key(key) {
            continue;
        }
        let kind = p["type"].as_str().unwrap_or("");
        let text = p["text"]
            .as_str()
            .unwrap_or(key)
            .chars()
            .take(200)
            .collect::<String>();
        let label = if text == "ui_browse_properties_scheme_color" {
            "Theme color".into()
        } else {
            text
        };
        let enabled = used.contains(key) && ["bool", "slider", "color", "combo"].contains(&kind);
        let min = p["min"].as_f64().unwrap_or(0.);
        let max = p["max"].as_f64().unwrap_or(100.).max(min + 0.000001);
        let options=p["options"].as_array().map(|a|a.iter().take(128).filter_map(|o|Some(json!({"label":o["label"].as_str()?.chars().take(200).collect::<String>(),"value":o["value"].as_str()?.chars().take(100).collect::<String>()}))).collect::<Vec<_>>()).unwrap_or_default();
        output.push(json!({"key":key,"type":kind,"label":label,"default":p["value"],"min":min,"max":max,"step":p["step"].as_f64().filter(|v|*v>0.).unwrap_or(if p["fraction"]==true {0.01}else{1.}),"order":p["order"].as_f64().or_else(||p["index"].as_f64()).unwrap_or(0.),"options":options,"condition":p["condition"].as_str().unwrap_or(""),"supported":enabled,"reason":if key=="schemecolor" {"Theme color is used by Wallpaper Engine's interface."}else{"This option depends on a feature not supported by Atelier yet."}}));
    }
    output.sort_by(|a, b| {
        a["order"]
            .as_f64()
            .unwrap_or(0.)
            .total_cmp(&b["order"].as_f64().unwrap_or(0.))
    });
    output
}
pub fn metadata(directory: &Path) -> Result<Value, String> {
    let small = directory.join("properties.json");
    let path = if small.is_file() {
        small
    } else {
        directory.join("scene.json")
    };
    if fs::metadata(&path).map_err(|e| e.to_string())?.len() > 32 * 1024 * 1024 {
        return Err("Scene manifest exceeds memory limit".into());
    }
    serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exposes_only_properties_with_supported_bindings_and_validates_values() {
        let project = json!({"general":{"properties":{"speed":{"type":"slider","min":0,"max":2,"value":1},"color":{"type":"color","value":"1 1 1"},"particles":{"type":"bool","value":true}}}});
        let schema = schema_from(&project, &json!([{"user":"speed"},{"user":"color"}]));
        assert_eq!(
            schema.iter().find(|p| p["key"] == "particles").unwrap()["supported"],
            false
        );
        let mut settings = SceneOptions {
            speed: 99.,
            properties: BTreeMap::from([
                ("speed".into(), json!(9)),
                ("particles".into(), json!(false)),
                ("color".into(), json!("1 0.5 0")),
            ]),
            ..Default::default()
        };
        settings.validate(&schema);
        assert_eq!(settings.speed, 3.);
        assert_eq!(settings.properties["speed"], 2.);
        assert!(!settings.properties.contains_key("particles"));
        let restored: SceneOptions =
            serde_json::from_slice(&serde_json::to_vec(&settings).unwrap()).unwrap();
        assert_eq!(restored, settings);
    }
}
