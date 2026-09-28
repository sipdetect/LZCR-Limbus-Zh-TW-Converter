//! 戰鬥語音氣泡 `BattleSpeechBubble*.json` 自動著色。
//!
//! 規則對齊 LocalizeLimbusCompany `scripts/colorize_battle_speech_bubble.py`
//! 與 `syler/Translation_Notes.md` §3–4、`Color_Analysis.md`。

use std::fs;
use std::path::Path;
use std::sync::OnceLock;

use regex::Regex;
use serde_json::{json, Value};

use crate::error::AppError;

// ── 罪人代表色（Translation_Notes.md §3）──────────────────────────────
// 罪人代表色（官方主題色名）
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

// Color_Analysis.md + 主要 NPC
const COLOR_KROMER: &str = "af612c";
const COLOR_JIA_QIU: &str = "cc273d";
const COLOR_LEI_HENG: &str = "ba5e6f";
const COLOR_JIA_HUAN_GUBO: &str = "9d1230";
const COLOR_JIA_MU: &str = "ffffff";
const COLOR_QUEEN_HATRED: &str = "ff6baa";
const COLOR_HONGYUAN: &str = "5bffde";
const COLOR_LUNAR: &str = "5b88cd";
const COLOR_BLOODFIEND: &str = "c41e23";
const COLOR_DISTORTED: &str = "b98788";
const COLOR_KIM_SATGAT: &str = "3e5bea";
const COLOR_KOMI: &str = "7fa188";
const COLOR_DONGRANG: &str = "d4e1e8"; // 與李箱同系
const COLOR_DANTE: &str = "b01c37"; // Inferno Red
const COLOR_VERGILIUS_CHARON: &str = "84a79d"; // Stygian Cobalt

const SPECIAL_ID_PREFIXES: &[(&str, &str)] = &[
    ("battle_speechbubble_cromer", COLOR_KROMER),
    ("battle_1137_", COLOR_JIA_QIU),
];

const ID_TOKEN_COLORS: &[(&str, &str)] = &[("cromer", COLOR_KROMER)];

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

/// 在字串中找 101xx–112xx（五碼）；不用 lookaround（regex crate 不支援）。
fn find_personality_code(s: &str) -> Option<&str> {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i + 5 <= bytes.len() {
        let slice = &bytes[i..i + 5];
        if slice.iter().all(|b| b.is_ascii_digit()) {
            // ASCII digits → 一定是有效 UTF-8
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

fn re_sinner_seq() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"battle_s3_4000(2[5-9]|3[0-6])_").unwrap())
}

fn re_outer_color() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // (?s) 讓 . 跨行；非貪婪中段，靠結尾 </color>
    RE.get_or_init(|| Regex::new(r"(?s)^\s*<color=#([0-9a-fA-F]{6})>(.*)</color>\s*$").unwrap())
}

struct SpecialIdRule {
    re: Regex,
    color: &'static str,
    /// true = 取 capture 1 做 400025→101 序列
    sinner_seq: bool,
}

fn special_id_rules() -> &'static [SpecialIdRule] {
    static RULES: OnceLock<Vec<SpecialIdRule>> = OnceLock::new();
    RULES.get_or_init(|| {
        vec![
            SpecialIdRule {
                re: Regex::new(r"^8SV-0(1[5-9]|2[0-4])$").unwrap(),
                color: COLOR_JIA_QIU,
                sinner_seq: false,
            },
            SpecialIdRule {
                re: Regex::new(r"^8SV-0(2[5-9]|3[0-2])$").unwrap(),
                color: COLOR_LEI_HENG,
                sinner_seq: false,
            },
            SpecialIdRule {
                re: Regex::new(r"^8SV-0(3[4-9]|4[0-2])$").unwrap(),
                color: COLOR_JIA_HUAN_GUBO,
                sinner_seq: false,
            },
            SpecialIdRule {
                re: Regex::new(r"^8SV-0(5[1-9]|6[0-1])$").unwrap(),
                color: COLOR_JIA_MU,
                sinner_seq: false,
            },
            SpecialIdRule {
                re: Regex::new(r"^6SV-0(1[8-9]|2[0-9])").unwrap(),
                color: COLOR_QUEEN_HATRED,
                sinner_seq: false,
            },
            SpecialIdRule {
                re: Regex::new(r"^6SV-0(0[9]|1[0-7])").unwrap(),
                color: COLOR_BLOODFIEND,
                sinner_seq: false,
            },
            SpecialIdRule {
                re: Regex::new(r"^6SV-00[1-8]").unwrap(),
                color: COLOR_DISTORTED,
                sinner_seq: false,
            },
            SpecialIdRule {
                re: Regex::new(r"^7SV-BV-").unwrap(),
                color: COLOR_BLOODFIEND,
                sinner_seq: false,
            },
            SpecialIdRule {
                re: Regex::new(r"^952SV-").unwrap(),
                color: COLOR_KIM_SATGAT,
                sinner_seq: false,
            },
            SpecialIdRule {
                re: Regex::new(r"30501").unwrap(),
                color: COLOR_LUNAR,
                sinner_seq: false,
            },
            SpecialIdRule {
                re: Regex::new(r"400005").unwrap(),
                color: COLOR_HONGYUAN,
                sinner_seq: false,
            },
            SpecialIdRule {
                re: Regex::new(r"battle_s3_4000(2[5-9]|3[0-6])_").unwrap(),
                color: "SINNER_SEQ",
                sinner_seq: true,
            },
        ]
    })
}

struct SpeakerRule {
    re: Regex,
    color: &'static str,
}

fn speaker_rules() -> &'static [SpeakerRule] {
    static RULES: OnceLock<Vec<SpeakerRule>> = OnceLock::new();
    RULES.get_or_init(|| {
        let mut rules = Vec::new();
        let push = |rules: &mut Vec<SpeakerRule>, pat: &str, color: &'static str| {
            rules.push(SpeakerRule {
                re: Regex::new(pat).unwrap(),
                color,
            });
        };

        // NPC / Boss
        push(&mut rules, r"^(桑丘|산초)", COLOR_BLOODFIEND);
        push(&mut rules, r"^(卡塞蒂|카세티|薩莎|사샤)", COLOR_BLOODFIEND);
        push(&mut rules, r"^(克羅默|크로머|cromer)", COLOR_KROMER);
        push(&mut rules, r"^(賈丘|가치우)", COLOR_JIA_QIU);
        push(&mut rules, r"^(雷橫|뇌횡)", COLOR_LEI_HENG);
        push(&mut rules, r"^(賈環|가환|仇甫|구보)", COLOR_JIA_HUAN_GUBO);
        push(&mut rules, r"^(賈母|가모|史彌胤|사미윤)", COLOR_JIA_MU);
        push(&mut rules, r"^(賈惜春|가시춘|惜春)", COLOR_HONGYUAN);
        push(&mut rules, r"^(憎惡女王|증오의\s*여왕)", COLOR_QUEEN_HATRED);
        push(&mut rules, r"^(金笠|김삿갓)", COLOR_KIM_SATGAT);
        push(&mut rules, r"^(東朗|동랑)", COLOR_DONGRANG);
        push(&mut rules, r"^(可米|꼬미)", COLOR_KOMI);
        push(&mut rules, r"^(時間殺人魔|시간살인마)", COLOR_DISTORTED);
        push(&mut rules, r"^(蜚蠊皇帝|바퀴|Cockroach)", COLOR_DISTORTED);
        push(&mut rules, r"^(扭曲)", COLOR_DISTORTED);
        // 但丁／維吉里烏斯／卡戎
        push(
            &mut rules,
            r"^(但丁|단테|Dante)",
            COLOR_DANTE,
        );
        push(
            &mut rules,
            r"^(維吉里烏斯|维吉里乌斯|베르길리우스|Vergilius|維吉爾|维吉尔)",
            COLOR_VERGILIUS_CHARON,
        );
        push(
            &mut rules,
            r"^(卡戎|카론|Charon)",
            COLOR_VERGILIUS_CHARON,
        );

        // 罪人（含「未來 辛克萊」）— 繁簡前綴
        let sinners: &[(&str, &str)] = &[
            (r"^(?:未來\s*|未来\s*)?(李箱|이상)", "101"),
            (r"^(?:未來\s*|未来\s*)?(浮士德|파우스트)", "102"),
            (r"^(?:未來\s*|未来\s*)?(堂吉訶德|堂吉诃德|돈키호테)", "103"),
            (r"^(?:未來\s*|未来\s*)?(良秀|료슈)", "104"),
            (r"^(?:未來\s*|未来\s*)?(默爾索|默尔索|뫼르소)", "105"),
            (r"^(?:未來\s*|未来\s*)?(鴻璐|鸿璐|홍루)", "106"),
            (r"^(?:未來\s*|未来\s*)?(希斯克利夫|히스클리프)", "107"),
            (r"^(?:未來\s*|未来\s*)?(以實瑪利|以实玛利|이스마엘)", "108"),
            (r"^(?:未來\s*|未来\s*)?(羅佳|罗佳|로쟈)", "109"),
            (r"^(?:未來\s*|未来\s*)?(辛克萊|辛克莱|싱클레어)", "110"),
            (r"^(?:未來\s*|未来\s*)?(奧提斯|奥提스|오티스)", "111"),
            (r"^(?:未來\s*|未来\s*)?(格里高爾|格里高尔|그레고르)", "112"),
        ];
        for (pat, code) in sinners {
            let color = sinner_color(code).expect("sinner color");
            push(&mut rules, pat, color);
        }
        rules
    })
}

fn sinner_seq_color(entry_id: &str) -> Option<&'static str> {
    let m = re_sinner_seq().captures(entry_id)?;
    let n: i32 = m.get(1)?.as_str().parse().ok()?;
    let code = (101 + (n - 25)).to_string();
    sinner_color(&code)
}

/// 回傳 6 碼 hex（無 #）；無法判定則 None。
pub fn resolve_color(entry_id: &str, desc: &str) -> Option<&'static str> {
    for (prefix, color) in SPECIAL_ID_PREFIXES {
        if entry_id.starts_with(prefix) {
            return Some(*color);
        }
    }

    for rule in special_id_rules() {
        if rule.re.is_match(entry_id) {
            if rule.sinner_seq {
                return sinner_seq_color(entry_id);
            }
            return Some(rule.color);
        }
    }

    if let Some(full) = find_personality_code(entry_id) {
        let code3 = &full[..3];
        if let Some(c) = sinner_color(code3) {
            return Some(c);
        }
    }

    let lower = entry_id.to_ascii_lowercase();
    for (token, color) in ID_TOKEN_COLORS {
        if lower.contains(token) {
            return Some(*color);
        }
    }

    let desc = desc.trim();
    if !desc.is_empty() {
        for rule in speaker_rules() {
            if rule.re.is_match(desc) {
                return Some(rule.color);
            }
        }
    }

    None
}

pub fn strip_outer_color(dlg: &str) -> String {
    if let Some(caps) = re_outer_color().captures(dlg) {
        return caps.get(2).map(|m| m.as_str().to_string()).unwrap_or_default();
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

/// 處理單一 BattleSpeechBubble JSON。`force` 時剝除外層色後重套。
pub fn process_file(path: &Path, force: bool) -> Result<ColorizeStats, AppError> {
    let raw = fs::read_to_string(path).map_err(AppError::Io)?;
    let mut data: Value = serde_json::from_str(raw.trim_start_matches('\u{feff}'))
        .map_err(|e| AppError::Other(format!("解析 {}: {e}", path.display())))?;

    let mut stats = ColorizeStats::default();
    let Some(list) = data.get_mut("dataList").and_then(|v| v.as_array_mut()) else {
        return Ok(stats);
    };

    for entry in list.iter_mut() {
        stats.total += 1;
        let id = entry
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let desc = entry
            .get("desc")
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

        match resolve_color(&id, &desc) {
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

/// 掃描 `root` 下 `BattleSpeechBubble*.json` 並著色。
pub fn colorize_directory(root: &Path, force: bool) -> Result<ColorizeStats, AppError> {
    let mut grand = ColorizeStats::default();
    if !root.is_dir() {
        return Ok(grand);
    }

    let mut paths: Vec<_> = fs::read_dir(root)
        .map_err(AppError::Io)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.extension().and_then(|x| x.to_str()) == Some("json")
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("BattleSpeechBubble"))
                    .unwrap_or(false)
        })
        .collect();
    paths.sort();

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
    fn resolve_sinner_personality() {
        assert_eq!(
            resolve_color("battle_speechbubble_10808_1", ""),
            Some("ff9500")
        );
        assert_eq!(resolve_color("battle_react_10710_1", ""), Some("4e3076"));
        assert_eq!(resolve_color("battle_react_10310_10", ""), Some("ffef23"));
        assert_eq!(resolve_color("battle_eE_10415_1", ""), Some("cf0000"));
    }

    #[test]
    fn resolve_special_ids() {
        assert_eq!(
            resolve_color("battle_speechbubble_cromer_1", ""),
            Some("af612c")
        );
        assert_eq!(resolve_color("8SV-015", ""), Some("cc273d"));
        assert_eq!(resolve_color("8SV-025", ""), Some("ba5e6f"));
        assert_eq!(resolve_color("6SV-018", ""), Some("ff6baa"));
        assert_eq!(resolve_color("6SV-001", ""), Some("b98788"));
        assert_eq!(resolve_color("battle_s3_400025_1", ""), Some("d4e1e8"));
        assert_eq!(resolve_color("battle_s3_400036_1", ""), Some("69350b"));
    }

    #[test]
    fn resolve_speaker_prefix_not_opponent() {
        assert_eq!(resolve_color("9SV-BAT7-06", "與良秀拼點勝利時"), None);
        assert_eq!(
            resolve_color("x", "未來 辛克萊 3技能"),
            Some("8b9c15")
        );
        assert_eq!(resolve_color("x", "蜚蠊皇帝 1技能"), Some("b98788"));
        assert_eq!(resolve_color("x", "但丁"), Some("b01c37"));
        assert_eq!(resolve_color("x", "維吉里烏斯 攻擊"), Some("84a79d"));
        assert_eq!(resolve_color("x", "卡戎"), Some("84a79d"));
    }

    #[test]
    fn wrap_strip_outer() {
        let plain = "測試\n第二行";
        let wrapped = wrap_color(plain, "ffef23");
        assert_eq!(wrapped, "<color=#ffef23>測試\n第二行</color>");
        assert_eq!(strip_outer_color(&wrapped), plain);
        let inner = "前。<color=#50acff>*嗶*</color>後。";
        let out = wrap_color(inner, "ffef23");
        assert!(out.contains("<color=#50acff>*嗶*</color>"));
        assert_eq!(strip_outer_color(&out), inner);
    }

    #[test]
    fn process_file_wraps_uncolored() {
        let dir = std::env::temp_dir().join(format!("lzcr-bubble-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("BattleSpeechBubbleDlg_test.json");
        let content = r#"{
  "dataList": [
    {"id": "battle_react_10710_1", "desc": "test", "dlg": "你就這麼迎接客人嗎？！"},
    {"id": "9SV-BAT7-06", "desc": "與良秀拼點勝利時", "dlg": "那樣不對。"}
  ]
}
"#;
        let mut f = fs::File::create(&path).unwrap();
        f.write_all(content.as_bytes()).unwrap();

        let st = process_file(&path, true).unwrap();
        assert_eq!(st.wrapped + st.recolored + st.skipped_already, 1);
        assert_eq!(st.no_color, 1);

        let data: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        let dlg0 = data["dataList"][0]["dlg"].as_str().unwrap();
        assert!(dlg0.starts_with("<color=#4e3076>"));
        let dlg1 = data["dataList"][1]["dlg"].as_str().unwrap();
        assert!(!dlg1.starts_with("<color=#cf0000>"));
        assert!(!dlg1.starts_with("<color="));

        let _ = fs::remove_dir_all(&dir);
    }
}
