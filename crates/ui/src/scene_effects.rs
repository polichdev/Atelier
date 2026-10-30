use serde_json::{json, Value};

pub fn composites(object: &Value) -> Vec<Value> {
    if let Some(particle) = object["particle"].as_str() {
        if particle.ends_with("/rainrefractive.json") || particle.ends_with("/rain.json") {
            return vec![
                json!({"kind":"rain","visible":true,"values":{"speed":object["instanceoverride"].get("speed").cloned().unwrap_or(json!(1)),"strength":object["instanceoverride"].get("rate").cloned().unwrap_or(json!(0.5))},"mask_source":null}),
            ];
        }
        return Vec::new();
    }
    if !object["image"]
        .as_str()
        .is_some_and(|s| s.starts_with("models/util/"))
    {
        return Vec::new();
    }
    let mut output = Vec::new();
    for effect in object["effects"].as_array().into_iter().flatten().take(8) {
        let file = effect["file"].as_str().unwrap_or("");
        let kind = if file == "effects/godrays/effect.json" {
            "godrays"
        } else if file.ends_with("/bokeh_blur/effect.json") || file == "effects/blur/effect.json" {
            "blur"
        } else if file.ends_with("/bloom/effect.json") {
            "bloom"
        } else {
            continue;
        };
        let passes = effect["passes"]
            .as_array()
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        let mut values = serde_json::Map::new();
        let mut mask = None;
        for pass in passes {
            for (key, value) in pass["constantshadervalues"]
                .as_object()
                .into_iter()
                .flatten()
            {
                values.entry(key.clone()).or_insert_with(|| value.clone());
            }
            if mask.is_none() {
                mask = pass["textures"]
                    .as_array()
                    .and_then(|a| {
                        a.iter()
                            .filter_map(Value::as_str)
                            .find(|s| s.starts_with("masks/"))
                    })
                    .map(str::to_owned);
            }
        }
        output.push(json!({"kind":kind,"visible":effect.get("visible").cloned().unwrap_or(json!(true)),"values":values,"mask_source":mask}));
    }
    output
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preserves_authors_composite_switches_and_numeric_bindings() {
        let o = json!({"image":"models/util/composelayer.json","effects":[{"file":"effects/workshop/2798319181/bokeh_blur/effect.json","visible":{"user":"blurbackground","value":false},"passes":[{"textures":[null,"masks/blur"]},{"constantshadervalues":{"Opacity":{"user":"opacity","value":1}}}]}]});
        let effects = composites(&o);
        assert_eq!(effects.len(), 1);
        assert_eq!(effects[0]["kind"], "blur");
        assert_eq!(effects[0]["values"]["Opacity"]["user"], "opacity");
        assert_eq!(effects[0]["visible"]["user"], "blurbackground");
        assert!(composites(&json!({"particle":"particles/custom/unknown.json"})).is_empty());
    }
}
