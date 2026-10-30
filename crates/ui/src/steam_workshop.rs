use serde::Serialize;
use std::{
    ffi::c_void,
    fs,
    path::{Path, PathBuf},
};
use windows::core::{w, PCWSTR};

fn atelier_workshop_root() -> PathBuf {
    super::settings_path()
        .unwrap_or_else(|| std::env::temp_dir().join("Atelier/settings.json"))
        .with_file_name("workshop")
        .join("content/431960")
}

#[derive(Serialize, Debug, Clone)]
pub struct LocalWorkshopEntry {
    pub added_at: u64,
    pub id: String,
    pub title: String,
    pub description: String,
    pub kind: String,
    pub preview: Option<String>,
    pub video_available: bool,
    pub web_available: bool,
    pub scene_available: bool,
}
fn workshop_id(input: &str) -> Result<String, String> {
    let input = input.trim();
    let id = if input.bytes().all(|b| b.is_ascii_digit()) {
        input
    } else {
        let query = input
            .strip_prefix("https://steamcommunity.com/sharedfiles/filedetails/?")
            .ok_or("Enter a Workshop ID or Steam Workshop link")?;
        query
            .split('&')
            .find_map(|pair| pair.strip_prefix("id="))
            .ok_or("The link has no Workshop ID")?
    };
    if id.is_empty()
        || id.len() > 20
        || !id.bytes().all(|b| b.is_ascii_digit())
        || id.parse::<u64>().is_err()
    {
        return Err("Invalid Workshop ID".into());
    }
    Ok(id.into())
}
#[link(name = "advapi32")]
extern "system" {
    fn RegGetValueW(
        key: isize,
        subkey: *const u16,
        value: *const u16,
        flags: u32,
        kind: *mut u32,
        data: *mut c_void,
        size: *mut u32,
    ) -> i32;
}
fn steam_root() -> Option<PathBuf> {
    let key: Vec<u16> = "Software\\Valve\\Steam"
        .encode_utf16()
        .chain(Some(0))
        .collect();
    let name: Vec<u16> = "SteamPath".encode_utf16().chain(Some(0)).collect();
    let mut buffer = [0u16; 1024];
    let mut size = std::mem::size_of_val(&buffer) as u32;
    let status = unsafe {
        RegGetValueW(
            0x80000001u32 as i32 as isize,
            key.as_ptr(),
            name.as_ptr(),
            2,
            std::ptr::null_mut(),
            buffer.as_mut_ptr().cast(),
            &mut size,
        )
    };
    if status == 0 {
        let len = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
        let p = PathBuf::from(String::from_utf16_lossy(&buffer[..len]));
        if p.is_dir() {
            return Some(p);
        }
    }
    let fallback = std::env::var_os("ProgramFiles(x86)")
        .map(PathBuf::from)?
        .join("Steam");
    fallback.is_dir().then_some(fallback)
}
fn vdf_strings(data: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut it = data.chars().peekable();
    while let Some(c) = it.next() {
        if c != '"' {
            continue;
        }
        let mut val = String::new();
        while let Some(c) = it.next() {
            if c == '"' {
                break;
            }
            if c == '\\' {
                match it.next() {
                    Some('\\') => val.push('\\'),
                    Some('"') => val.push('"'),
                    Some(x) => {
                        val.push('\\');
                        val.push(x)
                    }
                    None => break,
                }
            } else {
                val.push(c)
            }
        }
        result.push(val)
    }
    result
}
fn libraries(base: &Path) -> Vec<PathBuf> {
    let mut roots = vec![base.to_path_buf()];
    let p = base.join("steamapps/libraryfolders.vdf");
    if let Ok(data) = fs::read_to_string(p) {
        if data.len() <= 4 * 1024 * 1024 {
            let tokens = vdf_strings(&data);
            for pair in tokens.windows(2) {
                if pair[0].eq_ignore_ascii_case("path") {
                    let p = PathBuf::from(&pair[1]);
                    if p.is_dir() && !roots.contains(&p) {
                        roots.push(p)
                    }
                }
            }
        }
    }
    roots
}
fn load_preview_as_data_url(path: &Path) -> Option<String> {
    let meta = fs::metadata(path).ok()?;
    if meta.len() == 0 || meta.len() > 4 * 1024 * 1024 {
        return None;
    }
    let data = fs::read(path).ok()?;
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    let mime = match ext.as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        _ => return None,
    };
    use base64::Engine;
    let b64 = base64::prelude::BASE64_STANDARD.encode(&data);
    Some(format!("data:{mime};base64,{b64}"))
}

struct Installed {
    title: String,
    description: String,
    video: Option<PathBuf>,
    scene: Option<PathBuf>,
}

fn inspect_project(
    dir: &Path,
) -> Option<(
    String,
    String,
    String,
    Option<String>,
    Option<PathBuf>,
    Option<PathBuf>,
    Option<PathBuf>,
)> {
    let canonical_dir = dir.canonicalize().ok()?;
    let dir = canonical_dir.as_path();
    let project = dir.join("project.json");
    let meta = fs::metadata(&project).ok()?;
    if meta.len() > 1024 * 1024 {
        return None;
    }
    let file = project.canonicalize().ok()?;
    if !file.starts_with(dir) {
        return None;
    }
    let data = fs::read(&file).ok()?;
    let value: serde_json::Value = serde_json::from_slice(&data).ok()?;

    let kind = value["type"].as_str().unwrap_or("unknown").to_string();
    let title = value["title"]
        .as_str()
        .filter(|t| !t.trim().is_empty())
        .unwrap_or("")
        .chars()
        .take(200)
        .collect();
    let description = value["description"]
        .as_str()
        .unwrap_or("")
        .chars()
        .take(1000)
        .collect();

    let preview = value["preview"]
        .as_str()
        .and_then(|name| {
            let p = dir.join(name);
            p.is_file().then(|| load_preview_as_data_url(&p)).flatten()
        })
        .or_else(|| {
            for fallback in &["preview.jpg", "preview.png", "preview.gif", "preview.jpeg"] {
                let fp = dir.join(fallback);
                if fp.is_file() {
                    if let Some(data_url) = load_preview_as_data_url(&fp) {
                        return Some(data_url);
                    }
                }
            }
            None
        });

    let video = if kind == "video" {
        value["file"].as_str().and_then(|name| {
            let path = Path::new(name);
            if path.is_absolute()
                || path
                    .components()
                    .any(|c| !matches!(c, std::path::Component::Normal(_)))
            {
                return None;
            }
            let ext = path.extension()?.to_str()?.to_ascii_lowercase();
            if !["mp4", "webm"].contains(&ext.as_str()) {
                return None;
            }
            let resolved = dir.join(path).canonicalize().ok()?;
            (resolved.starts_with(dir) && resolved.is_file()).then_some(resolved)
        })
    } else {
        None
    };

    let scene = if kind == "scene" {
        dir.join("scene.pkg")
            .canonicalize()
            .ok()
            .filter(|path| path.starts_with(dir) && path.is_file())
    } else {
        None
    };

    let web = if kind == "web" {
        let name = value["file"].as_str().unwrap_or("index.html");
        let path = Path::new(name);
        if !path.is_absolute()
            && path
                .components()
                .all(|c| matches!(c, std::path::Component::Normal(_)))
        {
            let resolved = dir.join(path).canonicalize().ok()?;
            (resolved.starts_with(dir) && resolved.is_file()).then_some(resolved)
        } else {
            None
        }
    } else {
        None
    };

    Some((kind, title, description, preview, video, scene, web))
}

fn local_item(id: &str, base: &Path) -> Option<Installed> {
    let atelier_folder = atelier_workshop_root().join(id);
    if let Ok(dir) = atelier_folder.canonicalize() {
        if let Some((_, title, description, _, video, scene, _)) = inspect_project(&dir) {
            return Some(Installed {
                title: if title.is_empty() {
                    format!("Steam Workshop {id}")
                } else {
                    title
                },
                description,
                video,
                scene,
            });
        }
    }
    let roots = libraries(base);
    for r in roots {
        let root = r
            .join("steamapps/workshop/content/431960")
            .canonicalize()
            .ok();
        if let Some(root) = root {
            let folder = root.join(id);
            if let Ok(dir) = folder.canonicalize() {
                if dir.starts_with(&root) {
                    if let Some((_, title, description, _, video, scene, _)) = inspect_project(&dir)
                    {
                        return Some(Installed {
                            title: if title.is_empty() {
                                format!("Steam Workshop {id}")
                            } else {
                                title
                            },
                            description,
                            video,
                            scene,
                        });
                    }
                }
            }
        }
    }
    None
}

fn installed_at(path: &Path) -> u64 {
    fs::metadata(path)
        .ok()
        .and_then(|m| m.created().or_else(|_| m.modified()).ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis().min(u64::MAX as u128) as u64)
        .unwrap_or(0)
}

pub fn workshop_media_response(
    request: tauri::http::Request<Vec<u8>>,
) -> tauri::http::Response<Vec<u8>> {
    let missing = || {
        tauri::http::Response::builder()
            .status(404)
            .header("Access-Control-Allow-Origin", "*")
            .body(Vec::new())
            .unwrap()
    };
    let parts: Vec<_> = request
        .uri()
        .path()
        .trim_start_matches('/')
        .split('/')
        .collect();
    if request.method() != tauri::http::Method::GET
        || parts.len() != 2
        || parts[0].is_empty()
        || !parts[0].bytes().all(|b| b.is_ascii_digit())
        || workshop_id(parts[0]).is_err()
    {
        return missing();
    }
    let root = steam_root().unwrap_or_else(atelier_workshop_root);
    let Some(item) = local_item(parts[0], &root) else {
        return missing();
    };
    let (path, mime) = match parts[1] {
        "video" => {
            let Some(path) = item.video else {
                return missing();
            };
            let mime = if path
                .extension()
                .and_then(|s| s.to_str())
                .is_some_and(|s| s.eq_ignore_ascii_case("webm"))
            {
                "video/webm"
            } else {
                "video/mp4"
            };
            (path, mime)
        }
        "cover" => {
            let Some(source) = item.scene else {
                return missing();
            };
            let Ok(meta) = fs::metadata(&source) else {
                return missing();
            };
            let stamp = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            let cache = super::settings_path()
                .unwrap_or_else(|| std::env::temp_dir().join("Atelier/settings.json"))
                .with_file_name("cache")
                .join("workshop-previews");
            let path = cache.join(format!("v2-{}-{}-{stamp}.png", parts[0], meta.len()));
            let unavailable = path.with_extension("unavailable");
            if unavailable.is_file() {
                return missing();
            }
            if !path.is_file() {
                static PREVIEWS: std::sync::Mutex<()> = std::sync::Mutex::new(());
                let Ok(_guard) = PREVIEWS.lock() else {
                    return missing();
                };
                if !path.is_file() {
                    if unavailable.is_file() {
                        return missing();
                    }
                    if fs::create_dir_all(&cache).is_err() {
                        return missing();
                    }
                    let Ok(png) = super::scene_preview::render(&source) else {
                        let _ = fs::write(&unavailable, []);
                        return missing();
                    };
                    let temporary = path.with_extension("tmp");
                    if fs::write(&temporary, png)
                        .and_then(|_| fs::rename(&temporary, &path))
                        .is_err()
                    {
                        let _ = fs::remove_file(temporary);
                        return missing();
                    }
                }
            }
            (path, "image/png")
        }
        _ => return missing(),
    };
    let Ok(mut file) = fs::File::open(path) else {
        return missing();
    };
    let Ok(size) = file.metadata().map(|m| m.len()) else {
        return missing();
    };
    let range = request.headers().get("range").and_then(|h| h.to_str().ok());
    let Ok((start, end, partial)) = super::desktop_wallpaper::range(range, size) else {
        return tauri::http::Response::builder()
            .status(416)
            .header("Content-Range", format!("bytes */{size}"))
            .body(Vec::new())
            .unwrap();
    };
    use std::io::{Read, Seek, SeekFrom};
    let mut bytes = vec![0; (end - start + 1) as usize];
    if file.seek(SeekFrom::Start(start)).is_err() || file.read_exact(&mut bytes).is_err() {
        return missing();
    }
    let mut response = tauri::http::Response::builder()
        .status(if partial { 206 } else { 200 })
        .header("Content-Type", mime)
        .header("Access-Control-Allow-Origin", "*")
        .header("Accept-Ranges", "bytes")
        .header("Content-Length", bytes.len());
    if partial {
        response = response.header("Content-Range", format!("bytes {start}-{end}/{size}"));
    }
    response.body(bytes).unwrap()
}

pub fn scan_local_workshop_items() -> Vec<LocalWorkshopEntry> {
    let mut results = Vec::new();
    let mut seen_ids = std::collections::HashSet::new();

    let atelier_root = atelier_workshop_root();
    if let Ok(entries) = fs::read_dir(&atelier_root) {
        for entry in entries.flatten() {
            if let Ok(file_type) = entry.file_type() {
                if file_type.is_dir() {
                    let name = entry.file_name();
                    let id_str = name.to_string_lossy();
                    if id_str.bytes().all(|b| b.is_ascii_digit())
                        && seen_ids.insert(id_str.to_string())
                    {
                        if let Ok(dir) = entry.path().canonicalize() {
                            if let Some((kind, title, description, preview, video, scene, web)) =
                                inspect_project(&dir)
                            {
                                results.push(LocalWorkshopEntry {
                                    added_at: installed_at(&entry.path()),
                                    id: id_str.to_string(),
                                    title: if title.is_empty() {
                                        format!("Workshop {id_str}")
                                    } else {
                                        title
                                    },
                                    description,
                                    kind,
                                    preview,
                                    video_available: video.is_some(),
                                    web_available: web.is_some(),
                                    scene_available: scene.is_some(),
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    if let Some(base) = steam_root() {
        let roots = libraries(&base);
        for r in roots {
            let workshop_dir = r.join("steamapps/workshop/content/431960");
            if let Ok(entries) = fs::read_dir(&workshop_dir) {
                for entry in entries.flatten() {
                    if let Ok(file_type) = entry.file_type() {
                        if file_type.is_dir() {
                            let name = entry.file_name();
                            let id_str = name.to_string_lossy();
                            if id_str.bytes().all(|b| b.is_ascii_digit())
                                && seen_ids.insert(id_str.to_string())
                            {
                                if let Ok(dir) = entry.path().canonicalize() {
                                    if let Some((
                                        kind,
                                        title,
                                        description,
                                        preview,
                                        video,
                                        scene,
                                        web,
                                    )) = inspect_project(&dir)
                                    {
                                        results.push(LocalWorkshopEntry {
                                            added_at: installed_at(&entry.path()),
                                            id: id_str.to_string(),
                                            title: if title.is_empty() {
                                                format!("Workshop {id_str}")
                                            } else {
                                                title
                                            },
                                            description,
                                            kind,
                                            preview,
                                            video_available: video.is_some(),
                                            web_available: web.is_some(),
                                            scene_available: scene.is_some(),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
            for sub in &["myprojects", "defaultprojects"] {
                let projects_dir = r
                    .join("steamapps/common/wallpaper_engine/projects")
                    .join(sub);
                if let Ok(entries) = fs::read_dir(&projects_dir) {
                    for entry in entries.flatten() {
                        if let Ok(file_type) = entry.file_type() {
                            if file_type.is_dir() {
                                let name = entry.file_name();
                                let id_str = name.to_string_lossy();
                                let unique_key = format!("{sub}:{id_str}");
                                if seen_ids.insert(unique_key) {
                                    if let Ok(dir) = entry.path().canonicalize() {
                                        if let Some((
                                            kind,
                                            title,
                                            description,
                                            preview,
                                            video,
                                            scene,
                                            web,
                                        )) = inspect_project(&dir)
                                        {
                                            results.push(LocalWorkshopEntry {
                                                added_at: installed_at(&entry.path()),
                                                id: id_str.to_string(),
                                                title: if title.is_empty() {
                                                    format!("{sub} / {id_str}")
                                                } else {
                                                    title
                                                },
                                                description,
                                                kind,
                                                preview,
                                                video_available: video.is_some(),
                                                web_available: web.is_some(),
                                                scene_available: scene.is_some(),
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    results.sort_by(|a, b| b.added_at.cmp(&a.added_at).then_with(|| b.id.cmp(&a.id)));
    results
}

#[tauri::command]
pub async fn workshop_list_local() -> Result<Vec<LocalWorkshopEntry>, String> {
    tauri::async_runtime::spawn_blocking(scan_local_workshop_items)
        .await
        .map_err(|e| e.to_string())
}

pub fn scene_source_for_title(title: &str) -> Option<PathBuf> {
    let items = scan_local_workshop_items()
        .into_iter()
        .filter(|i| i.scene_available && i.title == title)
        .collect::<Vec<_>>();
    if items.len() != 1 {
        return None;
    }
    let root = steam_root().unwrap_or_else(atelier_workshop_root);
    local_item(&items[0].id, &root)?.scene
}

#[tauri::command]
pub async fn workshop_import_video(
    window: tauri::Window,
    app: tauri::AppHandle,
    id: String,
) -> Result<super::desktop_wallpaper::WallpaperItem, String> {
    if window.label() != "main" {
        return Err("Only the studio can import a wallpaper".into());
    }
    let id = workshop_id(&id)?;
    tauri::async_runtime::spawn_blocking(move || {
        let root = steam_root().unwrap_or_else(atelier_workshop_root);
        let item = local_item(&id, &root).ok_or("Workshop item is not downloaded")?;
        let video = item
            .video
            .ok_or("This item does not contain a supported MP4 or WebM video")?;
        super::desktop_wallpaper::import_workshop_video(
            &app,
            &video,
            &id,
            Some(&item.title),
            Some(&item.description),
        )
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn workshop_import_scene(
    window: tauri::Window,
    app: tauri::AppHandle,
    id: String,
) -> Result<super::desktop_wallpaper::WallpaperItem, String> {
    if window.label() != "main" {
        return Err("Only the studio can import scenes".into());
    }
    let id = workshop_id(&id)?;
    tauri::async_runtime::spawn_blocking(move || {
        let root = steam_root().unwrap_or_else(atelier_workshop_root);
        let item = local_item(&id, &root).ok_or("Scene is not downloaded")?;
        let source = item.scene.ok_or("This item does not contain scene.pkg")?;
        super::desktop_wallpaper::import_workshop_scene(
            &app,
            &source,
            &item.title,
            &item.description,
        )
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn workshop_open_browse(window: tauri::Window, query: Option<String>) -> Result<(), String> {
    if window.label() != "main" {
        return Err("Only the studio can open Steam".into());
    }
    let query_param = query
        .as_deref()
        .map(str::trim)
        .filter(|q| !q.is_empty())
        .map(|q| {
            let mut encoded = String::new();
            for b in q.bytes() {
                if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.' || b == b'~' {
                    encoded.push(b as char);
                } else if b == b' ' {
                    encoded.push('+');
                } else {
                    encoded.push_str(&format!("%{:02X}", b));
                }
            }
            format!("&searchtext={encoded}")
        })
        .unwrap_or_default();

    let web_url = format!("https://steamcommunity.com/workshop/browse/?appid=431960{query_param}");
    let client_url = format!("steam://openurl/{web_url}");

    let client_wide: Vec<u16> = client_url.encode_utf16().chain(Some(0)).collect();
    let result = unsafe {
        windows::Win32::UI::Shell::ShellExecuteW(
            None,
            w!("open"),
            PCWSTR(client_wide.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL,
        )
    };
    if (result.0 as isize) <= 32 {
        let web_wide: Vec<u16> = web_url.encode_utf16().chain(Some(0)).collect();
        let fallback = unsafe {
            windows::Win32::UI::Shell::ShellExecuteW(
                None,
                w!("open"),
                PCWSTR(web_wide.as_ptr()),
                PCWSTR::null(),
                PCWSTR::null(),
                windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL,
            )
        };
        if (fallback.0 as isize) <= 32 {
            return Err("Could not open Steam Workshop".into());
        }
    }
    Ok(())
}

#[tauri::command]
pub fn workshop_open_client(window: tauri::Window, id: String) -> Result<(), String> {
    if window.label() != "main" {
        return Err("Only the studio can open Steam".into());
    }
    let id = workshop_id(&id)?;
    let url = format!("steam://url/CommunityFilePage/{id}");
    let wide: Vec<u16> = url.encode_utf16().chain(Some(0)).collect();
    let result = unsafe {
        windows::Win32::UI::Shell::ShellExecuteW(
            None,
            w!("open"),
            PCWSTR(wide.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL,
        )
    };
    if result.0 as isize <= 32 {
        Err("Could not open Steam client".into())
    } else {
        Ok(())
    }
}

#[tauri::command]
pub fn workshop_open_page(window: tauri::Window, id: String) -> Result<(), String> {
    if window.label() != "main" {
        return Err("Only the studio can open Steam".into());
    }
    let id = workshop_id(&id)?;
    let url = format!("https://steamcommunity.com/sharedfiles/filedetails/?id={id}");
    let wide: Vec<u16> = url.encode_utf16().chain(Some(0)).collect();
    let result = unsafe {
        windows::Win32::UI::Shell::ShellExecuteW(
            None,
            w!("open"),
            PCWSTR(wide.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL,
        )
    };
    if result.0 as isize <= 32 {
        Err("Could not open Steam Workshop".into())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ids_require_a_steam_link_or_decimal_id() {
        assert_eq!(workshop_id("123").unwrap(), "123");
        assert_eq!(
            workshop_id(
                "https://steamcommunity.com/sharedfiles/filedetails/?id=123&searchtext=foo"
            )
            .unwrap(),
            "123"
        );
        assert!(workshop_id("https://evil.example/?id=123").is_err());
        assert!(workshop_id("../123").is_err());
    }
    #[test]
    fn vdf_paths_handle_backslashes() {
        let data = r#""libraryfolders" { "0" { "path" "C:\\Steam" } "1" { "path" "D:\\Games" } }"#;
        let t = vdf_strings(data);
        assert!(t.contains(&"C:\\Steam".into()));
        assert!(t.contains(&"D:\\Games".into()));
    }
    #[test]
    fn detects_video_and_rejects_paths_outside_the_item() {
        let temp =
            std::env::temp_dir().join(format!("atelier-workshop-test-{}", std::process::id()));
        let item = temp.join("steamapps/workshop/content/431960/123");
        fs::create_dir_all(&item).unwrap();
        fs::write(item.join("movie.mp4"), b"sample").unwrap();
        fs::write(
            item.join("project.json"),
            br#"{"type":"video","file":"movie.mp4"}"#,
        )
        .unwrap();
        assert!(local_item("123", &temp).unwrap().video.is_some());
        assert!(inspect_project(&item).unwrap().4.is_some());
        fs::write(
            item.join("project.json"),
            br#"{"type":"video","file":"../movie.mp4"}"#,
        )
        .unwrap();
        assert!(local_item("123", &temp).unwrap().video.is_none());
        fs::write(
            item.join("project.json"),
            br#"{"type":"scene","file":"scene.pkg"}"#,
        )
        .unwrap();
        assert!(local_item("123", &temp).unwrap().video.is_none());
        fs::remove_dir_all(temp).unwrap();
    }
}
