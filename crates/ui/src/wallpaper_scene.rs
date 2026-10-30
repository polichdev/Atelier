use atelier_wallpaper::{package::Package, texture};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    fs::{self, File},
    path::Path,
};

const MAX_ASSET: u64 = 64 * 1024 * 1024;
const MAX_SCENE: usize = 384 * 1024 * 1024;
struct Exporter<'a> {
    package: Package,
    file: File,
    directory: &'a Path,
    assets: HashMap<String, String>,
    total: usize,
}
impl Exporter<'_> {
    fn read(&mut self, name: &str, limit: u64) -> Result<Vec<u8>, String> {
        self.package
            .read_entry(&mut self.file, name, limit)
            .map_err(|e| format!("Scene asset {name}: {e}"))
    }
    fn json(&mut self, name: &str) -> Result<Value, String> {
        serde_json::from_slice(&self.read(name, 4 * 1024 * 1024)?)
            .map_err(|e| format!("Scene JSON {name}: {e}"))
    }
    fn asset(&mut self, name: &str, is_texture: bool) -> Result<String, String> {
        if let Some(file) = self.assets.get(name) {
            return Ok(file.clone());
        }
        if self.assets.len() >= 256 {
            return Err("Scene contains too many assets".into());
        }
        let bytes = self.read(name, MAX_ASSET)?;
        let (ext, bytes) = if is_texture {
            match texture::embedded_image(&bytes).map_err(|e| e.to_string())? {
                Some((ext, bytes)) => (ext, bytes),
                None => {
                    let image =
                        texture::decode(&bytes).map_err(|e| format!("Texture {name}: {e}"))?;
                    if self.total + image.rgba.len() > MAX_SCENE {
                        return Err("Scene texture memory exceeds 384 MB".into());
                    }
                    let mut png = Vec::new();
                    {
                        let mut encoder = png::Encoder::new(&mut png, image.width, image.height);
                        encoder.set_color(png::ColorType::Rgba);
                        encoder.set_depth(png::BitDepth::Eight);
                        encoder
                            .write_header()
                            .map_err(|e| e.to_string())?
                            .write_image_data(&image.rgba)
                            .map_err(|e| e.to_string())?;
                    }
                    self.total += image.rgba.len();
                    ("png", png)
                }
            }
        } else {
            let ext = Path::new(name)
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("");
            if !["mp3", "ogg", "wav"].contains(&ext) {
                return Err("Unsupported scene audio format".into());
            }
            (ext, bytes)
        };
        self.total += bytes.len();
        if self.total > MAX_SCENE {
            return Err("Scene assets exceed 384 MB".into());
        }
        let output = format!("{}.{}", self.assets.len(), ext);
        fs::write(self.directory.join(&output), bytes).map_err(|e| e.to_string())?;
        self.assets.insert(name.to_owned(), output.clone());
        Ok(output)
    }
    fn texture(&mut self, name: &str) -> Result<String, String> {
        let name = if name.ends_with(".tex") {
            name.to_owned()
        } else {
            format!("materials/{name}.tex")
        };
        self.asset(&name, true)
    }
}
fn value(v: &Value) -> &Value {
    v.get("value").unwrap_or(v)
}
fn visible(v: &Value) -> bool {
    value(v).as_bool().unwrap_or(true)
}
fn scalar(v: &Value, default: f64) -> f64 {
    value(v)
        .as_f64()
        .filter(|n| n.is_finite())
        .unwrap_or(default)
}
fn vector(v: &Value, default: &[f64]) -> Vec<f64> {
    let numbers: Vec<f64> = if let Some(s) = value(v).as_str() {
        s.split_whitespace()
            .filter_map(|s| s.parse::<f64>().ok())
            .collect()
    } else {
        value(v)
            .as_array()
            .map(|a| a.iter().filter_map(Value::as_f64).collect())
            .unwrap_or_default()
    };
    default
        .iter()
        .enumerate()
        .map(|(i, d)| {
            numbers
                .get(i)
                .copied()
                .filter(|n| n.is_finite())
                .unwrap_or(*d)
        })
        .collect()
}

pub fn prepare(source: &Path, destination: &Path) -> Result<String, String> {
    fs::create_dir(destination).map_err(|e| e.to_string())?;
    let result = prepare_inner(source, destination);
    if result.is_err() {
        let _ = fs::remove_dir_all(destination);
    }
    result
}
fn prepare_inner(source: &Path, destination: &Path) -> Result<String, String> {
    let mut file = File::open(source).map_err(|e| e.to_string())?;
    let package = Package::read(&mut file).map_err(|e| e.to_string())?;
    let mut e = Exporter {
        package,
        file,
        directory: destination,
        assets: HashMap::new(),
        total: 0,
    };
    let scene = e.json("scene.json")?;
    let width = scalar(&scene["general"]["orthogonalprojection"]["width"], 1920.0);
    let height = scalar(&scene["general"]["orthogonalprojection"]["height"], 1080.0);
    if !(1.0..=16384.0).contains(&width) || !(1.0..=16384.0).contains(&height) {
        return Err("Unsupported scene projection size".into());
    }
    let objects = super::scene_geometry::load(&scene, &e.package, &mut e.file)?;
    if objects.len() > 1024 {
        return Err("Scene contains too many objects".into());
    }
    let mut layers = Vec::new();
    let mut sounds = Vec::new();
    let mut warnings = Vec::<String>::new();
    for object in &objects {
        if object["atelier_visibility"]
            .as_array()
            .is_some_and(|a| a.iter().any(|v| !visible(v) && v.get("user").is_none()))
        {
            continue;
        }
        let mut composite = super::scene_effects::composites(object);
        if !composite.is_empty() {
            if layers.len() >= 128 {
                return Err("Scene contains too many layers".into());
            }
            for effect in &mut composite {
                effect["mask"] = if let Some(mask) = effect["mask_source"].as_str() {
                    json!(e.texture(mask)?)
                } else {
                    Value::Null
                };
                effect["composite"] = json!(true);
            }
            layers.push(json!({"id":object["id"],"kind":"composite","visibility":object["atelier_visibility"],"effects":composite}));
            continue;
        }
        if let Some(model) = object["image"].as_str() {
            if model.starts_with("models/util/") {
                warnings.push("Composite utility layers".into());
                continue;
            }
            if layers.len() >= 128 {
                return Err("Scene contains too many image layers".into());
            }
            let descriptor = e.json(model)?;
            let material = e.json(
                descriptor["material"]
                    .as_str()
                    .ok_or("Image layer has no material")?,
            )?;
            let pass = &material["passes"][0];
            let texture = pass["textures"][0]
                .as_str()
                .ok_or("Image layer has no texture")?;
            if !pass["shader"]
                .as_str()
                .unwrap_or("genericimage")
                .starts_with("genericimage")
            {
                warnings.push("Custom material shader".into());
            }
            let src = e.texture(texture)?;
            let mut effects = Vec::new();
            for effect in object["effects"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|v| visible(&v["visible"]) || v["visible"].get("user").is_some())
            {
                let name = effect["file"]
                    .as_str()
                    .unwrap_or("")
                    .split('/')
                    .nth(1)
                    .unwrap_or("");
                if ![
                    "waterwaves",
                    "waterripple",
                    "shake",
                    "pulse",
                    "waterflow",
                    "tint",
                    "opacity",
                ]
                .contains(&name)
                {
                    warnings.push(format!("Effect: {name}"));
                    continue;
                }
                let pass = &effect["passes"][0];
                let index = if name == "pulse" { 2 } else { 1 };
                let mask = pass["textures"][index]
                    .as_str()
                    .filter(|s| !s.starts_with("util/"))
                    .map(|s| e.texture(s))
                    .transpose()?;
                effects.push(json!({"kind":name,"visible":effect.get("visible").cloned().unwrap_or(json!(true)),"mask":mask,"values":pass["constantshadervalues"],"combos":pass["combos"]}));
            }
            if object.get("animationlayers").is_some() || object.to_string().contains("\"script\"")
            {
                warnings.push("SceneScript or skeletal animation".into());
            }
            layers.push(json!({"id":object["id"],"parent":object["parent"],"matrix":object["atelier_matrix"],"transforms":object["atelier_transforms"],"visibility":object["atelier_visibility"],"alpha_binding":object.get("alpha").cloned().unwrap_or(json!(1)),"color_binding":object.get("color").cloned().unwrap_or(json!("1 1 1")),"mesh":object["atelier_mesh"],"src":src,"origin":vector(&object["origin"], &[width/2.0,height/2.0,0.0]),"size":vector(&object["size"], &[width,height]),"scale":vector(&object["scale"], &[1.0,1.0,1.0]),"angle":vector(&object["angles"], &[0.0,0.0,0.0])[2],"alpha":scalar(&object["alpha"],1.0).clamp(0.0,1.0),"color":vector(&object["color"], &[1.0,1.0,1.0]),"blend":pass["blending"].as_str().unwrap_or("translucent"),"effects":effects}));
        } else if let Some(sound) = object["sound"]
            .as_array()
            .and_then(|v| v.first())
            .and_then(Value::as_str)
        {
            if sounds.len() < 8 {
                sounds.push(json!({"src":e.asset(sound, false)?,"visibility":object["atelier_visibility"],"volume_binding":object.get("volume").cloned().unwrap_or(json!(1)),"volume":scalar(&object["volume"],1.0).clamp(0.0,1.0)}));
            }
        } else if object.get("particle").and_then(Value::as_str).is_some()
            || object.get("model").and_then(Value::as_str).is_some()
        {
            warnings.push("Particles or 3D models".into());
        }
    }
    let mut preview = layers
        .iter()
        .find_map(|l| l["src"].as_str())
        .ok_or("This scene has no supported 2D image layers")?
        .to_owned();
    warnings.sort();
    warnings.dedup();
    let camera = json!({"enabled":scene["general"].get("camerashake").cloned().unwrap_or(json!(false)),"amplitude":scene["general"]["camerashakeamplitude"],"speed":scene["general"]["camerashakespeed"]});
    let properties = super::scene_properties::schema(
        source,
        &json!({"layers":layers,"sounds":sounds,"camera":camera}),
    );
    let output = json!({"version":3,"width":width,"height":height,"background":vector(&scene["general"]["clearcolor"],&[0.0,0.0,0.0]),"properties":properties,"camera":camera,"layers":layers,"sounds":sounds,"warnings":warnings});
    fs::write(
        destination.join("properties.json"),
        serde_json::to_vec(&json!({"version":3,"properties":properties}))
            .map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    fs::write(
        destination.join("scene.json"),
        serde_json::to_vec(&output).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    if let Ok(png) = super::scene_preview::render(source) {
        if fs::write(destination.join("256.png"), png).is_ok() {
            preview = "256.png".into();
        }
    }
    Ok(preview)
}

pub fn response(request: tauri::http::Request<Vec<u8>>) -> tauri::http::Response<Vec<u8>> {
    let not_found = || {
        tauri::http::Response::builder()
            .status(404)
            .body(Vec::new())
            .unwrap()
    };
    let parts: Vec<_> = request
        .uri()
        .path()
        .trim_start_matches('/')
        .split('/')
        .collect();
    if parts.len() != 2
        || parts[0].len() > 32
        || parts[0].is_empty()
        || !parts[0].bytes().all(|b| b.is_ascii_digit())
        || request.method() != tauri::http::Method::GET
    {
        return not_found();
    }
    let name = parts[1];
    let Some((stem, ext)) = name.rsplit_once('.') else {
        return not_found();
    };
    if name != "scene.json"
        && (stem.is_empty()
            || !stem.bytes().all(|b| b.is_ascii_digit())
            || !["png", "jpg", "mp3", "ogg", "wav"].contains(&ext))
    {
        return not_found();
    }
    let directory = super::settings_path()
        .unwrap_or_else(|| std::env::temp_dir().join("Atelier/settings.json"))
        .with_file_name("wallpapers")
        .join(format!("{}.scene", parts[0]));
    let Ok(mut file) = File::open(directory.join(name)) else {
        return not_found();
    };
    let Ok(len) = file.metadata().map(|m| m.len()) else {
        return not_found();
    };
    if len > MAX_ASSET || len == 0 {
        return not_found();
    }
    let range = request.headers().get("range").and_then(|h| h.to_str().ok());
    let Ok((start, end, partial)) = super::desktop_wallpaper::range(range, len) else {
        return tauri::http::Response::builder()
            .status(416)
            .header("Content-Range", format!("bytes */{len}"))
            .body(Vec::new())
            .unwrap();
    };
    use std::io::{Read, Seek, SeekFrom};
    let mut bytes = vec![0; (end - start + 1) as usize];
    if file.seek(SeekFrom::Start(start)).is_err() || file.read_exact(&mut bytes).is_err() {
        return not_found();
    }
    let mime = match ext {
        "json" => "application/json",
        "png" => "image/png",
        "jpg" => "image/jpeg",
        "mp3" => "audio/mpeg",
        "ogg" => "audio/ogg",
        "wav" => "audio/wav",
        _ => return not_found(),
    };
    let mut response = tauri::http::Response::builder()
        .status(if partial { 206 } else { 200 })
        .header("Content-Type", mime)
        .header("Access-Control-Allow-Origin", "*")
        .header("Accept-Ranges", "bytes")
        .header("Content-Length", bytes.len());
    if partial {
        response = response.header("Content-Range", format!("bytes {start}-{end}/{len}"));
    }
    response.body(bytes).unwrap()
}

#[cfg(debug_assertions)]
pub fn test_args() -> bool {
    if std::env::args().nth(1).as_deref() == Some("--scene-preview-test") {
        let args: Vec<_> = std::env::args_os().collect();
        if args.len() != 4 {
            eprintln!("Usage: --scene-preview-test scene.pkg output.png");
            return true;
        }
        match super::scene_preview::render(Path::new(&args[2])) {
            Ok(bytes) => {
                fs::write(&args[3], &bytes).unwrap();
                println!("Scene preview written: {} bytes", bytes.len());
            }
            Err(error) => eprintln!("Scene preview failed: {error}"),
        }
        return true;
    }
    let settings_test = std::env::args().nth(1).as_deref() == Some("--scene-settings-test");
    let desktop_test = std::env::args().nth(1).as_deref() == Some("--wallpaper-desktop-test");
    let runtime_test =
        desktop_test || std::env::args().nth(1).as_deref() == Some("--wallpaper-runtime-test");
    if !settings_test
        && !runtime_test
        && std::env::args().nth(1).as_deref() != Some("--scene-engine-test")
    {
        return false;
    }
    #[derive(Clone)]
    struct ReportPath(std::path::PathBuf, bool);
    #[tauri::command]
    fn scene_test_report(
        window: tauri::Window,
        app: tauri::AppHandle,
        path: tauri::State<ReportPath>,
        mut report: Value,
    ) {
        if !["scene-test", "main", "desktop-wallpaper"].contains(&window.label()) {
            return;
        }
        if let Some(png) = report
            .get("png")
            .and_then(Value::as_str)
            .and_then(|s| s.strip_prefix("data:image/png;base64,"))
        {
            use base64::Engine;
            if let Ok(bytes) = base64::prelude::BASE64_STANDARD.decode(png) {
                let _ = fs::write(path.0.with_extension("png"), bytes);
            }
        }
        report.as_object_mut().map(|r| r.remove("png"));
        if path.1 {
            if let Ok(hwnd) = window.hwnd() {
                unsafe {
                    use windows::Win32::Foundation::HWND;
                    use windows::Win32::UI::WindowsAndMessaging::{
                        GetAncestor, GetClassNameW, IsWindowVisible, GA_PARENT,
                    };
                    let hwnd = HWND(hwnd.0 as *mut _);
                    let parent = GetAncestor(hwnd, GA_PARENT);
                    let mut class = [0u16; 64];
                    let n = GetClassNameW(parent, &mut class);
                    report["parentClass"] =
                        json!(String::from_utf16_lossy(&class[..n.max(0) as usize]));
                    report["nativeVisible"] = json!(IsWindowVisible(hwnd).as_bool());
                }
            }
            let _ = super::desktop_background::shutdown();
        }
        let _ = fs::write(&path.0, serde_json::to_vec_pretty(&report).unwrap());
        println!("{report}");
        app.exit(if report["ok"].as_bool() == Some(true) {
            0
        } else {
            1
        });
    }
    #[tauri::command]
    fn wallpaper_repair() -> Result<(), String> {
        Ok(())
    }
    let args: Vec<_> = std::env::args_os().collect();
    if args.len() != 4 {
        eprintln!("Usage: --scene-engine-test scene.pkg report.json");
        return true;
    }
    let source = Path::new(&args[2]);
    let report = std::path::PathBuf::from(&args[3]);
    let directory = report.parent().unwrap().join("scene-engine-data");
    let assets = directory.join("wallpapers/1.scene");
    fs::create_dir_all(assets.parent().unwrap()).unwrap();
    let prepared = if runtime_test {
        fs::create_dir(&assets)
            .and_then(|_| {
                for entry in fs::read_dir(source)? {
                    let entry = entry?;
                    if entry.file_type()?.is_file() {
                        fs::copy(entry.path(), assets.join(entry.file_name()))?;
                    }
                }
                Ok(())
            })
            .map_err(|e| e.to_string())
    } else {
        prepare(source, &assets).map(|_| ())
    };
    if let Err(error) = prepared {
        eprintln!("{error}");
        let _ = fs::write(&report, json!({"ok":false,"error":error}).to_string());
        return true;
    }
    std::env::set_var("ATELIER_DATA_DIR", &directory);
    if runtime_test {
        let mut config: Value = serde_json::from_slice(
            &fs::read(source.parent().unwrap().join("library.json")).unwrap(),
        )
        .unwrap();
        let id = source.file_stem().unwrap().to_str().unwrap();
        let mut item = config["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|i| i["id"] == id)
            .unwrap()
            .clone();
        item["id"] = json!("1");
        config["items"] = json!([item]);
        config["active"] = json!("1");
        config["settings"] = json!({"1":config["settings"][id]});
        fs::write(
            directory.join("wallpapers/library.json"),
            serde_json::to_vec(&config).unwrap(),
        )
        .unwrap();
    }
    if settings_test {
        let project: Value = serde_json::from_slice(
            &fs::read(source.parent().unwrap().join("project.json")).unwrap(),
        )
        .unwrap();
        let mut manifest = super::scene_properties::metadata(&assets).unwrap();
        manifest["version"] = json!(1);
        manifest.as_object_mut().unwrap().remove("properties");
        fs::write(
            assets.join("scene.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        fs::remove_file(assets.join("properties.json")).unwrap();
        fs::write(directory.join("wallpapers/library.json"),json!({"items":[{"id":"1","title":project["title"],"category":"scene","kind":"scene","description":"Migration fixture","asset":"256.png"}]}).to_string()).unwrap();
    }
    let mut context = tauri::generate_context!();
    context.config_mut().app.windows.clear();
    let _ = tauri::Builder::default()
        .manage(ReportPath(report, desktop_test))
        .manage(super::desktop_wallpaper::Wallpapers::new())
        .register_asynchronous_uri_scheme_protocol("wallpaper-scene", |_ctx, request, responder| {
            std::thread::spawn(move || responder.respond(response(request)));
        })
        .invoke_handler(tauri::generate_handler![
            scene_test_report,
            super::desktop_wallpaper::wallpaper_scene_info,
            super::desktop_wallpaper::wallpaper_snapshot,
            super::desktop_wallpaper::wallpaper_scene_options,
            super::desktop_wallpaper::wallpaper_options,
            super::desktop_wallpaper::wallpaper_performance_options,
            super::desktop_wallpaper::wallpaper_playback_state,
            super::desktop_wallpaper::wallpaper_report,
            wallpaper_repair
        ])
        .setup(move |app| {
            tauri::WebviewWindowBuilder::new(
                app,
                if runtime_test {
                    "desktop-wallpaper"
                } else if settings_test {
                    "main"
                } else {
                    "scene-test"
                },
                tauri::WebviewUrl::App(
                    if runtime_test {
                        "wallpaper.html"
                    } else if settings_test {
                        "scene-settings-test.html"
                    } else {
                        "scene-engine-test.html"
                    }
                    .into(),
                ),
            )
            .visible(false)
            .inner_size(
                if runtime_test { 1920. } else { 960. },
                if runtime_test { 1080. } else { 540. },
            )
            .initialization_script(if runtime_test {
                "window.__ATELIER_RUNTIME_TEST__=true;"
            } else {
                ""
            })
            .data_directory(directory.join("webview"))
            .build()?;
            if desktop_test {
                let handle = app.handle().clone();
                std::thread::spawn(move || {
                    if let Err(error) = super::desktop_wallpaper::start(&handle) {
                        eprintln!("Desktop test failed: {error}");
                        let _ = super::desktop_background::shutdown();
                        handle.exit(1);
                    }
                });
            }
            Ok(())
        })
        .run(context);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_asset_traversal_without_opening_files() {
        for path in [
            "/../scene.json",
            "/123/../scene.json",
            "/123/source.pkg",
            "/abc/0.png",
            "/123/1.png/extra",
        ] {
            let request = tauri::http::Request::builder()
                .uri(format!("http://wallpaper-scene.localhost{path}"))
                .body(Vec::new())
                .unwrap();
            assert_eq!(response(request).status(), 404);
        }
    }
    #[test]
    fn handles_wrapped_scene_values() {
        assert_eq!(
            vector(&json!({"value":"1 2 3"}), &[0.0, 0.0, 0.0]),
            vec![1.0, 2.0, 3.0]
        );
        assert_eq!(scalar(&json!({"value":0.4}), 1.0), 0.4);
        assert!(!visible(&json!({"value":false})));
    }
}
