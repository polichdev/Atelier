use base64::Engine;
use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::PathBuf,
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};
const MAX_SIZE: u64 = 150 * 1024 * 1024;
struct Upload {
    id: String,
    file: File,
    path: PathBuf,
    size: u64,
    written: u64,
}
impl Upload {
    fn append(&mut self, offset: u64, bytes: &[u8]) -> Result<(), String> {
        if offset != self.written
            || bytes.is_empty()
            || self.written + bytes.len() as u64 > self.size
        {
            return Err("Invalid image upload position".into());
        }
        self.file.write_all(bytes).map_err(|e| e.to_string())?;
        self.written += bytes.len() as u64;
        Ok(())
    }
    fn finish(&self, root: &std::path::Path) -> Result<String, String> {
        if self.written != self.size {
            return Err("Image upload incomplete".into());
        }
        self.file.sync_all().map_err(|e| e.to_string())?;
        let mut head = [0u8; 6];
        File::open(&self.path)
            .and_then(|mut f| f.read_exact(&mut head))
            .map_err(|e| e.to_string())?;
        if &head != b"GIF87a" && &head != b"GIF89a" {
            return Err("Invalid GIF file".into());
        }
        std::fs::copy(&self.path, root.join(format!("{}.gif", self.id)))
            .map_err(|e| e.to_string())?;
        Ok(format!("stored:{}", self.id))
    }
}
impl Drop for Upload {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}
#[derive(Default)]
pub struct PhotoUploads(Mutex<Option<Upload>>);
fn dir() -> PathBuf {
    super::settings_path()
        .unwrap_or_else(|| std::env::temp_dir().join("Atelier/settings.json"))
        .with_file_name("widget-photos")
}
fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 32 && id.bytes().all(|b| b.is_ascii_digit())
}
fn ensure_main(window: &tauri::Window) -> Result<(), String> {
    if window.label() == "main" {
        Ok(())
    } else {
        Err("Photo upload requires the studio".into())
    }
}
#[tauri::command]
pub async fn photo_upload_begin(
    window: tauri::Window,
    state: tauri::State<'_, PhotoUploads>,
    size: u64,
) -> Result<String, String> {
    ensure_main(&window)?;
    if size < 6 || size > MAX_SIZE {
        return Err("Choose a GIF up to 150 MB".into());
    }
    let mut state = state.0.lock().map_err(|e| e.to_string())?;
    *state = None;
    let root = dir();
    std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    let id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos()
        .to_string();
    let path = root.join(format!("{id}.part"));
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|e| e.to_string())?;
    *state = Some(Upload {
        id: id.clone(),
        file,
        path,
        size,
        written: 0,
    });
    Ok(id)
}
#[tauri::command]
pub async fn photo_upload_chunk(
    window: tauri::Window,
    state: tauri::State<'_, PhotoUploads>,
    id: String,
    offset: u64,
    data: String,
) -> Result<(), String> {
    ensure_main(&window)?;
    if data.len() > 400_000 {
        return Err("Image chunk is too large".into());
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data)
        .map_err(|_| "Invalid image chunk")?;
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    let u = guard
        .as_mut()
        .filter(|u| u.id == id)
        .ok_or("Upload unavailable")?;
    u.append(offset, &bytes)
}
#[tauri::command]
pub async fn photo_upload_finish(
    window: tauri::Window,
    state: tauri::State<'_, PhotoUploads>,
    id: String,
) -> Result<String, String> {
    ensure_main(&window)?;
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    let u = guard
        .as_ref()
        .filter(|u| u.id == id)
        .ok_or("Upload unavailable")?;
    let result = u.finish(&dir())?;
    *guard = None;
    Ok(result)
}
pub fn response(request: tauri::http::Request<Vec<u8>>) -> tauri::http::Response<Vec<u8>> {
    response_in(request, &dir())
}
fn response_in(
    request: tauri::http::Request<Vec<u8>>,
    root: &std::path::Path,
) -> tauri::http::Response<Vec<u8>> {
    let id = request.uri().path().trim_start_matches('/');
    let bytes = if request.method() == tauri::http::Method::GET && valid_id(id) {
        std::fs::read(root.join(format!("{id}.gif"))).ok()
    } else {
        None
    };
    match bytes {
        Some(data) if data.len() as u64 <= MAX_SIZE => tauri::http::Response::builder()
            .header("Content-Type", "image/gif")
            .header("Cache-Control", "public, max-age=31536000, immutable")
            .body(data)
            .unwrap(),
        _ => tauri::http::Response::builder()
            .status(404)
            .body(Vec::new())
            .unwrap(),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn large_gif_upload_and_serve() {
        let root = std::env::temp_dir().join(format!("atelier-photo-test-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("123.part");
        let file = File::create(&path).unwrap();
        let size = 50 * 1024 * 1024;
        let mut upload = Upload {
            id: "123".into(),
            file,
            path,
            size,
            written: 0,
        };
        let mut chunk = vec![0u8; 256 * 1024];
        chunk[..6].copy_from_slice(b"GIF89a");
        assert!(upload.append(2, &chunk).is_err());
        for offset in (0..size).step_by(chunk.len()) {
            upload.append(offset, &chunk).unwrap();
        }
        assert_eq!(upload.finish(&root).unwrap(), "stored:123");
        let response = response_in(
            tauri::http::Request::builder()
                .uri("http://widget-photo.localhost/123")
                .body(Vec::new())
                .unwrap(),
            &root,
        );
        assert_eq!(response.status(), 200);
        assert_eq!(response.body().len(), size as usize);
        assert_eq!(&response.body()[..6], b"GIF89a");
        drop(upload);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn id_is_not_a_path() {
        assert!(valid_id("12345"));
        assert!(!valid_id("../secret"));
        assert!(!valid_id(""));
    }
}
