//! PersonalityVoiceDlg 語音台詞自動著色。
//!
//! 規則對齊 LocalizeLimbusCompany `scripts/colorize_personality_voice.py`
//! 與 `syler/Translation_Notes.md` §3。

use std::fs;
use std::path::Path;
use std::sync::OnceLock;

use regex::Regex;
use serde_json::{json, Value};

use crate::error::AppError;

const SINNER_COLORS: &[(&str, &str)] = &[
    ("101", "d4e1e8"), // 李箱 Dreamy Gray
    ("102", "ffb1b4"), // 浮士德 Cerebral Pink
    ("103", "ffef23"), // 堂吉訶德 Oblivion Yellow
    ("104", "cf0000"), // 良秀 Smoky Scarlet
    ("105", "293b95"), // 默爾索 Decay Blue
    ("106", "5bffde"), // 鴻璐 Naive Cyan
    ("107", "4e3076"), // 希斯克利夫 Furious Violet
    ("108", "ff9500"), // 以實瑪利 Isolate Orange
    ("109", "820000"), // 羅佳 Lusty Burgundy
    ("110", "8b9c15"), // 辛克萊 Immature Green
    ("111", "325339"), // 奧提斯 Militant Olive
    ("112", "69350b"), // 格里高爾 Verminous Brown
];

/// 檔名片段備援（上游偶有拼寫錯誤）
const NAME_COLORS: &[(&str, &str)] = &[
    ("yisang", "d4e1e8"),
    ("faust", "ffb1b4"),
    ("donquixote", "ffef23"),
    ("ryoshu", "cf0000"),
    ("meursault", "293b95"),
    ("meursalut", "293b95"),
    ("honglu", "5bffde"),
    ("heathcliff", "4e3076"),
    ("ishmael", "ff9500"),
    ("rodion", "820000"),
    ("sinclair", "8b9c15"),
    ("outis", "325339"),
    ("outist", "325339"),
    ("gregor", "69350b"),
    ("dante", "b01c37"),
    ("vergilius", "84a79d"),
    ("charon", "84a79d"),
];

#[derive(Debug, Default, Clone)]
pub struct ColorizeStats {
    pub files: usize,
    pub total: usize,
    pub wrapped: usize,
    pub recolored: usize,
    pub skipped_already: usize,
    pub skipped_empty: usize,
    pub no_color: usize,
}

fn sinner_color(code3: &str) -> Option<&'static str> {
    SINNER_COLORS
        .iter()
        .find(|(k, _)| *k == code3)
        .map(|(_, c)| *c)
}

/// 在字串中找 101xx–112xx（五碼）；不用 lookaround。
fn find_personality_code(s: &str) -> Option<&str> {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i + 5 <= bytes.len() {
        let slice = &bytes[i..i + 5];
        if slice.iter().all(|b| b.is_ascii_digit()) {
            let code = std::str::from_utf8(slice).unwrap();
            let prefix = &code[..3];
            let ok_prefix = matches!(
                prefix,
                "101" | "102" | "103" | "104" | "105" | "106" | "107" | "108" | "109"
                    | "110" | "111" | "112"
            );
            let left_ok = i == 0 || !bytes[i - 1].is_ascii_digit();
            let right_ok = i + 5 >= bytes.len() || !bytes[i + 5].is_ascii_digit();
            if ok_prefix && left_ok && right_ok {
                return Some(code);
            }
        }
        i += 1;
    }
    None
}

fn re_file_personality() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // _10101.json / _10101A.json
    RE.get_or_init(|| {
        Regex::new(r"(?i)_((?:10[1-9]|11[0-2])\d{2})[A-Za-z]?\.json$").unwrap()
    })
}

fn re_outer_color() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?s)^\s*<color=#([0-9a-fA-F]{6})>(.*)</color>\s*$").unwrap())
}

pub fn color_from_personality_code(code5: &str) -> Option<&'static str> {
    if code5.len() < 3 {
        return None;
    }
    sinner_color(&code5[..3])
}

pub fn resolve_color_from_filename(path: &Path) -> Option<&'static str> {
    let name = path.file_name()?.to_str()?;
    if let Some(caps) = re_file_personality().captures(name) {
        return color_from_personality_code(caps.get(1)?.as_str());
    }
    let lower = name.to_ascii_lowercase();
    let padded = format!("_{lower}");
    for (key, color) in NAME_COLORS {
        if padded.contains(&format!("_{key}_")) || lower.starts_with(&format!("voice_{key}_")) {
            return Some(*color);
        }
    }
    None
}

pub fn resolve_color_from_entry_id(entry_id: &str) -> Option<&'static str> {
    find_personality_code(entry_id).and_then(color_from_personality_code)
}

/// 優先：條目 id 五碼 → 檔名五碼／角色名。
pub fn resolve_color(path: &Path, entry_id: &str) -> Option<&'static str> {
    resolve_color_from_entry_id(entry_id).or_else(|| resolve_color_from_filename(path))
}

pub fn strip_outer_color(dlg: &str) -> String {
    if let Some(caps) = re_outer_color().captures(dlg) {
        return caps
            .get(2)
            .map(|m| m.as_str().to_string())
            .unwrap_or_default();
    }
    dlg.to_string()
}

pub fn wrap_color(dlg: &str, color: &str) -> String {
    if dlg.trim().is_empty() {
        return dlg.to_string();
    }
    format!("<color=#{color}>{dlg}</color>")
}

fn has_outer_color(dlg: &str) -> bool {
    re_outer_color().is_match(dlg)
}

pub fn process_file(path: &Path, force: bool) -> Result<ColorizeStats, AppError> {
    let raw = fs::read_to_string(path).map_err(AppError::Io)?;
    let mut data: Value = serde_json::from_str(raw.trim_start_matches('\u{feff}'))
        .map_err(|e| AppError::Other(format!("解析 {}: {e}", path.display())))?;

    let mut stats = ColorizeStats::default();
    let Some(list) = data.get_mut("dataList").and_then(|v| v.as_array_mut()) else {
        return Ok(stats);
    };

    let file_default = resolve_color_from_filename(path);

    for entry in list.iter_mut() {
        stats.total += 1;
        let id = entry
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let Some(dlg_val) = entry.get("dlg") else {
            stats.skipped_empty += 1;
            continue;
        };
        let Some(raw_dlg) = dlg_val.as_str() else {
            stats.skipped_empty += 1;
            continue;
        };
        if raw_dlg.is_empty() {
            stats.skipped_empty += 1;
            continue;
        }

        let had_outer = has_outer_color(raw_dlg);
        let plain = strip_outer_color(raw_dlg);

        if had_outer && !force {
            stats.skipped_already += 1;
            continue;
        }

        let color = resolve_color(path, &id).or(file_default);
        match color {
            None => {
                if force && had_outer {
                    entry["dlg"] = json!(plain);
                    stats.recolored += 1;
                }
                stats.no_color += 1;
            }
            Some(color) => {
                let new_dlg = wrap_color(&plain, color);
                if new_dlg == raw_dlg {
                    stats.skipped_already += 1;
                } else {
                    entry["dlg"] = json!(new_dlg);
                    if had_outer {
                        stats.recolored += 1;
                    } else {
                        stats.wrapped += 1;
                    }
                }
            }
        }
    }

    let out = serde_json::to_string_pretty(&data)
        .map_err(|e| AppError::Other(format!("序列化 {}: {e}", path.display())))?;
    fs::write(path, format!("{out}\n")).map_err(AppError::Io)?;
    Ok(stats)
}

/// 掃描 `PersonalityVoiceDlg` 目錄（或輸出根下的該子目錄）並著色。
pub fn colorize_directory(voice_dir: &Path, force: bool) -> Result<ColorizeStats, AppError> {
    let mut grand = ColorizeStats::default();
    if !voice_dir.is_dir() {
        return Ok(grand);
    }

    let mut paths: Vec<_> = fs::read_dir(voice_dir)
        .map_err(AppError::Io)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.extension().and_then(|x| x.to_str()) == Some("json")
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("Voice_") || n.ends_with(".json"))
                    .unwrap_or(false)
        })
        .collect();

    // 先 Voice_* 再其他 json（對齊 Python）
    paths.sort_by(|a, b| {
        let a_voice = a
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.starts_with("Voice_"))
            .unwrap_or(false);
        let b_voice = b
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.starts_with("Voice_"))
            .unwrap_or(false);
        match (a_voice, b_voice) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.cmp(b),
        }
    });

    for path in paths {
        let st = process_file(&path, force)?;
        grand.files += 1;
        grand.total += st.total;
        grand.wrapped += st.wrapped;
        grand.recolored += st.recolored;
        grand.skipped_already += st.skipped_already;
        grand.skipped_empty += st.skipped_empty;
        grand.no_color += st.no_color;
    }

    Ok(grand)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn filename_personality_codes() {
        let cases = [
            ("Voice_Yisang_LCB_10101.json", "d4e1e8"),
            ("Voice_Faust_Dawn_10216.json", "ffb1b4"),
            ("Voice_DonQuixote_Bloodfiend_10310.json", "ffef23"),
            ("Voice_Ryoshu_Hongyuan_10413.json", "cf0000"),
            ("Voice_Heathcliff_Erlking_10710A.json", "4e3076"),
            ("Voice_Outist_SevenAsso6_11104.json", "325339"),
            ("Voice_Meursalut_LEGO_10514.json", "293b95"),
            ("Voice_Gregor_LCB_11201.json", "69350b"),
            ("Voice_Sinclair_LCB_11001.json", "8b9c15"),
            ("Voice_Rodion_LCB_10901.json", "820000"),
        ];
        for (name, expect) in cases {
            assert_eq!(
                resolve_color_from_filename(Path::new(name)),
                Some(expect),
                "{name}"
            );
        }
    }

    #[test]
    fn entry_id_overrides_when_present() {
        let p = Path::new("Voice_Special_999.json");
        assert_eq!(resolve_color_from_filename(p), None);
        assert_eq!(
            resolve_color(p, "battle_clear_10301_2"),
            Some("ffef23")
        );
    }

    #[test]
    fn wrap_preserves_inner_color() {
        let inner = "前半段。<color=#ff6baa>粉紅插話</color>後半。";
        let out = wrap_color(inner, "ffef23");
        assert!(out.starts_with("<color=#ffef23>"));
        assert!(out.contains("<color=#ff6baa>粉紅插話</color>"));
        assert_eq!(strip_outer_color(&out), inner);
    }

    #[test]
    fn process_file_wraps_voice() {
        let dir = std::env::temp_dir().join(format!("lzcr-voice-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("Voice_Ishmael_LCB_10801.json");
        let content = r#"{
  "dataList": [
    {"id": "battle_10801_1", "dlg": "沒問題。"},
    {"id": "x", "dlg": ""}
  ]
}
"#;
        let mut f = fs::File::create(&path).unwrap();
        f.write_all(content.as_bytes()).unwrap();

        let st = process_file(&path, true).unwrap();
        assert_eq!(st.wrapped, 1);
        assert_eq!(st.skipped_empty, 1);

        let data: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        let dlg = data["dataList"][0]["dlg"].as_str().unwrap();
        assert!(dlg.starts_with("<color=#ff9500>"));

        let _ = fs::remove_dir_all(&dir);
    }
}
