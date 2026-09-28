use crate::error::AppError;
use crate::steam_registry;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

const APP_INFO_FILE: &str = "lzcr-info.json";
const LEGACY_APP_INFO_FILE: &str = "llc-info.json";
/// 舊版無此欄位；小於此版本會在首次載入時強制覆寫成新格式。
pub const CONFIG_FORMAT_VERSION: u32 = 2;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// 設定檔格式版本（用於舊用戶自動遷移）
    #[serde(default = "default_format_version_zero")]
    pub format_version: u32,
    pub repo_owner: String,
    pub repo_name: String,
    pub source_folder: String,
    pub output_base: String,
    /// 舊檔可能寫成 `last_commit_hash`；見 `parse_config_json` 正規化。
    #[serde(default)]
    pub last_release_tag: Option<String>,
    #[serde(default)]
    pub last_release_date: Option<String>,
}

fn default_format_version_zero() -> u32 {
    0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationConfig {
    pub lang: String,
    #[serde(rename = "titleFont")]
    pub title_font: String,
    #[serde(rename = "contextFont")]
    pub context_font: String,
    #[serde(rename = "samplingPointSize")]
    pub sampling_point_size: i32,
    pub padding: i32,
}

impl Default for Config {
    fn default() -> Self {
        let output_base = steam_registry::get_lang_folder_path()
            .map(|path| path.join("LLC_zh-Hant").to_string_lossy().to_string())
            .unwrap_or_else(|_| "LLC_zh-Hant".to_string());

        Config {
            format_version: CONFIG_FORMAT_VERSION,
            repo_owner: "LocalizeLimbusCompany".to_string(),
            repo_name: "LocalizeLimbusCompany".to_string(),
            source_folder: "Lang/LLC_zh-CN".to_string(),
            output_base,
            last_release_tag: None,
            last_release_date: None,
        }
    }
}

impl Default for TranslationConfig {
    fn default() -> Self {
        TranslationConfig {
            lang: "LLC_zh-Hant".to_string(),
            title_font: String::new(),
            context_font: String::new(),
            sampling_point_size: 78,
            padding: 5,
        }
    }
}

/// 載入設定。舊版／損毀的 `lzcr-info.json` 會自動遷移並**強制覆寫**成新格式，
/// 盡量不讓舊用戶開機就失敗。
pub fn load_config() -> Result<Config, AppError> {
    let config_path = get_lzcr_info_path()?;
    let legacy_path = get_legacy_llc_info_path()?;

    let (raw, from_path) = if config_path.exists() {
        (fs::read_to_string(&config_path)?, Some(config_path.clone()))
    } else if legacy_path.exists() {
        (fs::read_to_string(&legacy_path)?, Some(legacy_path.clone()))
    } else {
        let config = Config::default();
        save_config(&config)?;
        return Ok(config);
    };

    let (config, migrated) = match migrate_config_from_raw(&raw) {
        Ok(pair) => pair,
        Err(_err) => {
            // 完全無法解析：備份舊檔後寫入全新預設（保留可偵測的 output 路徑）
            if let Some(path) = &from_path {
                let _ = backup_corrupt_config(path);
            }
            let mut config = Config::default();
            if let Some(base) = salvage_output_base_from_text(&raw) {
                config.output_base = base;
            }
            config = normalize_config_paths(config);
            config.format_version = CONFIG_FORMAT_VERSION;
            save_config(&config)?;
            return Ok(config);
        }
    };

    let mut config = normalize_config_paths(config);
    let needs_rewrite = migrated || config.format_version < CONFIG_FORMAT_VERSION;
    if needs_rewrite {
        config.format_version = CONFIG_FORMAT_VERSION;
        // 強制覆寫成乾淨新格式（去掉 last_commit_hash、last_voice_update_date 等）
        save_config(&config)?;
    } else {
        // 已是新版：仍正規化寫回一次以去掉多餘鍵（冪等）
        let _ = save_config(&config);
    }

    Ok(config)
}

/// 回傳 (config, 是否做了結構遷移／欄位修正)。
fn migrate_config_from_raw(content: &str) -> Result<(Config, bool), AppError> {
    let mut migrated = false;
    let mut value: Value = serde_json::from_str(content).map_err(AppError::Json)?;

    let obj = value
        .as_object_mut()
        .ok_or_else(|| AppError::Other("設定檔根節點必須是 JSON 物件".to_string()))?;

    // 舊版重複鍵／別名
    if obj.contains_key("last_release_tag") {
        if obj.remove("last_commit_hash").is_some() {
            migrated = true;
        }
    } else if let Some(legacy) = obj.remove("last_commit_hash") {
        obj.insert("last_release_tag".to_string(), legacy);
        migrated = true;
    }

    // 已移除功能的欄位
    for obsolete in [
        "last_voice_update_date",
        "voice_bubble_enabled",
        "last_bubble_update",
        "bubble_version",
    ] {
        if obj.remove(obsolete).is_some() {
            migrated = true;
        }
    }

    // 缺 format_version 或過舊 → 標記遷移
    let fmt = obj
        .get("format_version")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;
    if fmt < CONFIG_FORMAT_VERSION {
        migrated = true;
        obj.insert(
            "format_version".to_string(),
            Value::from(CONFIG_FORMAT_VERSION),
        );
    }

    // 缺必要字串欄位時補預設（避免舊檔缺鍵直接炸）
    let defaults = Config::default();
    for (key, default_val) in [
        ("repo_owner", Value::String(defaults.repo_owner.clone())),
        ("repo_name", Value::String(defaults.repo_name.clone())),
        (
            "source_folder",
            Value::String(defaults.source_folder.clone()),
        ),
        ("output_base", Value::String(defaults.output_base.clone())),
    ] {
        match obj.get(key) {
            None => {
                obj.insert(key.to_string(), default_val);
                migrated = true;
            }
            Some(Value::Null) => {
                obj.insert(key.to_string(), default_val);
                migrated = true;
            }
            Some(Value::String(s)) if s.trim().is_empty() && key != "output_base" => {
                obj.insert(key.to_string(), default_val);
                migrated = true;
            }
            _ => {}
        }
    }

    // 只取 Config 認識的鍵再反序列化（丟掉未知鍵）
    let clean = json_pick_config_fields(obj);
    let mut config: Config = serde_json::from_value(clean).map_err(AppError::Json)?;
    config.format_version = CONFIG_FORMAT_VERSION;
    Ok((config, migrated))
}

fn json_pick_config_fields(obj: &serde_json::Map<String, Value>) -> Value {
    let keys = [
        "format_version",
        "repo_owner",
        "repo_name",
        "source_folder",
        "output_base",
        "last_release_tag",
        "last_release_date",
    ];
    let mut out = serde_json::Map::new();
    for k in keys {
        if let Some(v) = obj.get(k) {
            out.insert(k.to_string(), v.clone());
        }
    }
    Value::Object(out)
}

fn backup_corrupt_config(path: &Path) -> Result<(), AppError> {
    let backup = path.with_extension("json.bak");
    let _ = fs::copy(path, &backup);
    Ok(())
}

fn salvage_output_base_from_text(raw: &str) -> Option<String> {
    // 粗略從損毀 JSON 撈 "output_base": "..."
    let key = "\"output_base\"";
    let idx = raw.find(key)?;
    let after = &raw[idx + key.len()..];
    let colon = after.find(':')?;
    let rest = after[colon + 1..].trim_start();
    if !rest.starts_with('"') {
        return None;
    }
    let rest = &rest[1..];
    let mut out = String::new();
    let mut chars = rest.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(n) = chars.next() {
                out.push(match n {
                    'n' => '\n',
                    't' => '\t',
                    'r' => '\r',
                    '"' => '"',
                    '\\' => '\\',
                    other => other,
                });
            }
        } else if c == '"' {
            break;
        } else {
            out.push(c);
        }
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

fn normalize_config_paths(mut config: Config) -> Config {
    if !Path::new(&config.output_base).is_absolute() {
        if let Ok(lang_path) = steam_registry::get_lang_folder_path() {
            config.output_base = lang_path
                .join("LLC_zh-Hant")
                .to_string_lossy()
                .to_string();
        }
    }
    config
}

pub fn save_config(config: &Config) -> Result<(), AppError> {
    let mut to_save = config.clone();
    to_save.format_version = CONFIG_FORMAT_VERSION;
    let content = serde_json::to_string_pretty(&to_save)?;
    let config_path = get_lzcr_info_path()?;

    if let Some(parent) = config_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }

    // 原子一點：先寫 temp 再 rename（Windows 上 replace）
    let tmp = config_path.with_extension("json.tmp");
    fs::write(&tmp, &content)?;
    fs::rename(&tmp, &config_path).or_else(|_| {
        // rename 跨碟失敗時改直接覆寫
        fs::write(&config_path, &content)
    })?;
    Ok(())
}

pub fn save_translation_config(config: &TranslationConfig) -> Result<(), AppError> {
    let content = serde_json::to_string_pretty(config)?;
    let translation_config_path = get_translation_config_path()?;

    if let Some(parent) = translation_config_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }

    fs::write(translation_config_path, content)?;
    Ok(())
}

pub fn get_lzcr_info_path() -> Result<PathBuf, AppError> {
    if let Ok(lang_path) = steam_registry::get_lang_folder_path() {
        Ok(lang_path.join(APP_INFO_FILE))
    } else {
        Ok(PathBuf::from(APP_INFO_FILE))
    }
}

fn get_legacy_llc_info_path() -> Result<PathBuf, AppError> {
    if let Ok(lang_path) = steam_registry::get_lang_folder_path() {
        Ok(lang_path.join(LEGACY_APP_INFO_FILE))
    } else {
        Ok(PathBuf::from(LEGACY_APP_INFO_FILE))
    }
}

pub fn get_translation_config_path() -> Result<PathBuf, AppError> {
    if let Ok(lang_path) = steam_registry::get_lang_folder_path() {
        Ok(lang_path.join("config.json"))
    } else {
        Ok(PathBuf::from("config.json"))
    }
}

pub fn ensure_translation_config() -> Result<(), AppError> {
    let translation_config_path = get_translation_config_path()?;

    if !translation_config_path.exists() {
        let config = TranslationConfig::default();
        save_translation_config(&config)?;
    }

    Ok(())
}

pub fn update_version_info(release_tag: &str, release_date: Option<&str>) -> Result<(), AppError> {
    let mut config = load_config()?;
    config.last_release_tag = Some(release_tag.to_string());
    config.last_release_date = release_date.map(|d| d.to_string());
    config.format_version = CONFIG_FORMAT_VERSION;
    save_config(&config)?;
    Ok(())
}

pub fn should_update(current_release_tag: &str) -> Result<bool, AppError> {
    let config = load_config()?;

    match config.last_release_tag.as_ref() {
        Some(last_tag) => Ok(last_tag != current_release_tag),
        None => Ok(true),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_legacy_last_commit_hash_field() {
        let json = r#"{
            "repo_owner": "LocalizeLimbusCompany",
            "repo_name": "LocalizeLimbusCompany",
            "source_folder": "Lang/LLC_zh-CN",
            "output_base": "LLC_zh-Hant",
            "last_commit_hash": "v20240101"
        }"#;
        let (config, migrated) = migrate_config_from_raw(json).unwrap();
        assert!(migrated);
        assert_eq!(config.last_release_tag.as_deref(), Some("v20240101"));
        assert_eq!(config.format_version, CONFIG_FORMAT_VERSION);
    }

    #[test]
    fn deserializes_when_both_tag_and_legacy_hash_present() {
        let json = r#"{
            "repo_owner": "LocalizeLimbusCompany",
            "repo_name": "LocalizeLimbusCompany",
            "source_folder": "Lang/LLC_zh-CN",
            "output_base": "D:\\game\\Lang\\LLC_zh-Hant",
            "last_commit_hash": "2026072401",
            "last_release_tag": "2026072401",
            "last_release_date": "2026-07-24",
            "last_voice_update_date": "2026-05-30"
        }"#;
        let (config, migrated) = migrate_config_from_raw(json).unwrap();
        assert!(migrated);
        assert_eq!(config.last_release_tag.as_deref(), Some("2026072401"));
        assert_eq!(config.last_release_date.as_deref(), Some("2026-07-24"));
        assert_eq!(config.format_version, CONFIG_FORMAT_VERSION);
    }

    #[test]
    fn migrates_missing_fields_and_sets_format_version() {
        let json = r#"{
            "output_base": "D:\\game\\Lang\\LLC_zh-Hant",
            "last_release_tag": "x"
        }"#;
        let (config, migrated) = migrate_config_from_raw(json).unwrap();
        assert!(migrated);
        assert_eq!(config.repo_owner, "LocalizeLimbusCompany");
        assert_eq!(config.source_folder, "Lang/LLC_zh-CN");
        assert_eq!(config.format_version, CONFIG_FORMAT_VERSION);
    }

    #[test]
    fn salvage_output_base_from_broken_json() {
        let raw = r#"{ "output_base": "D:\\Steam\\Lang\\LLC_zh-Hant", "broken": "#;
        assert_eq!(
            salvage_output_base_from_text(raw).as_deref(),
            Some(r"D:\Steam\Lang\LLC_zh-Hant")
        );
    }

    #[test]
    fn should_update_returns_true_without_previous_tag() {
        let dir = std::env::temp_dir().join(format!("lzcr-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let config = Config {
            output_base: dir.join("LLC_zh-Hant").to_string_lossy().to_string(),
            ..Config::default()
        };
        assert!(config.last_release_tag.is_none());
        assert!(config.last_release_tag.as_deref() != Some("v999"));
    }
}
