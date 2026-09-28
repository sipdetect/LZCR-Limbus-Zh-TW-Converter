use crate::error::AppError;
use std::path::PathBuf;

#[cfg(target_os = "windows")]
use winreg::enums::*;
#[cfg(target_os = "windows")]
use winreg::RegKey;

const LIMBUS_COMPANY_APP_ID: &str = "1973530";

#[cfg(target_os = "windows")]
pub fn find_steam_path() -> Result<PathBuf, AppError> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);

    let steam_key = hklm
        .open_subkey("SOFTWARE\\WOW6432Node\\Valve\\Steam")
        .or_else(|_| hklm.open_subkey("SOFTWARE\\Valve\\Steam"))
        .map_err(|e| AppError::Other(format!("找不到 Steam 登錄路徑: {}", e)))?;

    let install_path: String = steam_key
        .get_value("InstallPath")
        .map_err(|e| AppError::Other(format!("無法讀取 Steam 安裝路徑: {}", e)))?;

    Ok(PathBuf::from(install_path))
}

#[cfg(target_os = "windows")]
pub fn find_limbus_company_path() -> Result<PathBuf, AppError> {
    let steam_path = find_steam_path()?;
    let steamapps_path = steam_path.join("steamapps");
    let limbus_path = steamapps_path.join("common").join("Limbus Company");

    if limbus_path.exists() {
        return Ok(limbus_path);
    }

    let library_folders_path = steamapps_path.join("libraryfolders.vdf");
    if library_folders_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&library_folders_path) {
            for lib_path in parse_library_paths(&content) {
                for subpath in ["steamapps", "SteamApps"] {
                    let potential_path = lib_path.join(subpath).join("common").join("Limbus Company");
                    if potential_path.exists() {
                        return Ok(potential_path);
                    }
                }
            }
        }
    }

    if let Ok(game_path) = find_game_by_steam_registry() {
        return Ok(game_path);
    }

    for path in [
        PathBuf::from("C:\\Program Files (x86)\\Steam\\steamapps\\common\\Limbus Company"),
        PathBuf::from("C:\\Program Files\\Steam\\steamapps\\common\\Limbus Company"),
        PathBuf::from("D:\\Steam\\steamapps\\common\\Limbus Company"),
        PathBuf::from("E:\\Steam\\steamapps\\common\\Limbus Company"),
    ] {
        if path.exists() {
            return Ok(path);
        }
    }

    Err(AppError::Other(
        "找不到 Limbus Company 遊戲目錄".to_string(),
    ))
}

#[cfg(target_os = "windows")]
fn find_game_by_steam_registry() -> Result<PathBuf, AppError> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);

    for uninstall_path in [
        format!(
            "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\Steam App {}",
            LIMBUS_COMPANY_APP_ID
        ),
        format!(
            "SOFTWARE\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\Steam App {}",
            LIMBUS_COMPANY_APP_ID
        ),
    ] {
        if let Ok(game_key) = hklm.open_subkey(&uninstall_path) {
            if let Ok(install_location) = game_key.get_value::<String, _>("InstallLocation") {
                let game_path = PathBuf::from(install_location);
                if game_path.exists() {
                    return Ok(game_path);
                }
            }
        }
    }

    Err(AppError::Other(
        "登錄中找不到遊戲安裝路徑".to_string(),
    ))
}

pub fn parse_library_paths(vdf_content: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let mut inside_library_folders = false;
    let mut brace_count = 0;

    for line in vdf_content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("\"libraryfolders\"") {
            inside_library_folders = true;
            continue;
        }

        if inside_library_folders {
            brace_count += trimmed.chars().filter(|&c| c == '{').count() as i32;
            brace_count -= trimmed.chars().filter(|&c| c == '}').count() as i32;

            if trimmed.starts_with("\"path\"") {
                if let Some(path_str) = extract_quoted_value(trimmed) {
                    let normalized_path = path_str.replace("\\\\", "\\");
                    let path = PathBuf::from(normalized_path);
                    if path.exists() {
                        paths.push(path);
                    }
                }
            }

            if brace_count <= 0 && inside_library_folders {
                break;
            }
        }
    }

    paths
}

fn extract_quoted_value(line: &str) -> Option<String> {
    let parts: Vec<&str> = line.split('"').collect();
    if parts.len() >= 4 {
        return Some(parts[3].to_string());
    }
    None
}

#[cfg(not(target_os = "windows"))]
pub fn find_steam_path() -> Result<PathBuf, AppError> {
    let home_dir = std::env::var("HOME")
        .map_err(|_| AppError::Other("無法取得 HOME 目錄".to_string()))?;

    #[cfg(target_os = "linux")]
    let steam_paths = vec![
        PathBuf::from(format!("{}/.steam/steam", home_dir)),
        PathBuf::from(format!("{}/.local/share/Steam", home_dir)),
        PathBuf::from("/usr/share/steam"),
    ];

    #[cfg(target_os = "macos")]
    let steam_paths = vec![PathBuf::from(format!(
        "{}/Library/Application Support/Steam",
        home_dir
    ))];

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    let steam_paths: Vec<PathBuf> = vec![];

    for path in steam_paths {
        if path.exists() {
            return Ok(path);
        }
    }

    Err(AppError::Other(
        "找不到 Steam 安裝目錄".to_string(),
    ))
}

#[cfg(not(target_os = "windows"))]
pub fn find_limbus_company_path() -> Result<PathBuf, AppError> {
    let steam_path = find_steam_path()?;
    let limbus_path = steam_path
        .join("steamapps")
        .join("common")
        .join("Limbus Company");

    if limbus_path.exists() {
        return Ok(limbus_path);
    }

    let library_folders_path = steam_path.join("steamapps").join("libraryfolders.vdf");
    if library_folders_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&library_folders_path) {
            for lib_path in parse_library_paths(&content) {
                let potential_path = lib_path
                    .join("steamapps")
                    .join("common")
                    .join("Limbus Company");
                if potential_path.exists() {
                    return Ok(potential_path);
                }
            }
        }
    }

    Err(AppError::Other(
        "找不到 Limbus Company 遊戲目錄".to_string(),
    ))
}

pub fn get_lang_folder_path() -> Result<PathBuf, AppError> {
    let game_path = find_limbus_company_path()?;
    let lang_path = game_path.join("LimbusCompany_Data").join("Lang");

    if !lang_path.exists() {
        std::fs::create_dir_all(&lang_path).map_err(|e| {
            AppError::Other(format!("無法建立 Lang 資料夾: {}", e))
        })?;
    }

    Ok(lang_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_library_paths_extracts_paths() {
        let existing = std::env::temp_dir();
        let escaped = existing.display().to_string().replace('\\', "\\\\");
        let vdf = format!(
            r#"
"libraryfolders"
{{
    "0"
    {{
        "path"        "{escaped}"
    }}
}}
"#
        );

        let paths = parse_library_paths(&vdf);
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0], existing);
    }
}