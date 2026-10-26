fn main() {
    open_theme_studio();
}

fn open_theme_studio() {
    let ui_path = std::env::current_dir()
        .map(|p| {
            p.join("crates")
                .join("atelier-ui")
                .join("ui")
                .join("index.html")
        })
        .unwrap_or_else(|_| std::path::PathBuf::from("crates/atelier-ui/ui/index.html"));

    let url = format!("file:///{}", ui_path.display()).replace('\\', "/");
    let app_arg = format!("--app={url}");

    if let Some(browser_exe) = find_app_browser() {
        let _ = std::process::Command::new(browser_exe)
            .args([
                &app_arg,
                "--window-size=920,660",
                "--app-auto-launched",
                "--disable-extensions",
            ])
            .spawn();
    } else {
        let _ = std::process::Command::new("msedge.exe")
            .args([&app_arg, "--window-size=920,660"])
            .spawn();
    }
}

fn find_app_browser() -> Option<std::path::PathBuf> {
    let candidate_paths = [
        r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
        r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
        r"C:\Program Files\Google\Chrome\Application\chrome.exe",
        r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
    ];

    for path_str in candidate_paths {
        let p = std::path::PathBuf::from(path_str);
        if p.exists() {
            return Some(p);
        }
    }

    if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
        let edge_user = std::path::PathBuf::from(&local_app_data)
            .join("Microsoft")
            .join("Edge")
            .join("Application")
            .join("msedge.exe");
        if edge_user.exists() {
            return Some(edge_user);
        }

        let chrome_user = std::path::PathBuf::from(&local_app_data)
            .join("Google")
            .join("Chrome")
            .join("Application")
            .join("chrome.exe");
        if chrome_user.exists() {
            return Some(chrome_user);
        }
    }

    None
}
