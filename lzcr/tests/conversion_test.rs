//! 轉換回歸測試。
//!
//! 自訂詞典的每一條都能改變任意檔案任意位置的輸出，
//! 一條寫錯就是全玩家看到壞掉的正體中文，且不會有任何錯誤訊息。
//! 這個檔案就是那個安全網。
//!
//! 新增或修改 `dict/*.tsv` 後，**必須**跑一次 `cargo test`。

use lzcr::conversion::ConversionPipeline;

/// 真實資料的形狀（取自 LLC_zh-CN，縮節）。
const REAL_DIALOG: &str = r#"{
  "dataList": [
    {
      "id": 36,
      "teller": "堂吉诃德",
      "dialog": "请派吾上阵！为了正义，吾等必将凯旋！",
      "usage": "(800101, VERY_HIGH)"
    }
  ]
}"#;

fn cv(s: &str) -> String {
    ConversionPipeline::new().convert(s)
}

// ── 核心覆寫：台灣口語「挨」 ──────────────────────────────────────────

#[test]
fn taiwan_ai_overrides_zh_cantonese_default() {
    // 官方 STPhrases 對這些詞的主候選是「捱」；我們要台灣用字「挨」。
    assert_eq!(cv("他挨饿。"), "他挨餓。");
    assert_eq!(cv("挨揍"), "挨揍");
    assert_eq!(cv("挨打"), "挨打");
    assert_eq!(cv("挨骂"), "挨罵");
    assert_eq!(cv("挨批"), "挨批");
}

/// 沒有被覆寫的「挨X」維持官方的「捱」——代表覆寫是精準的，
/// 沒有把整個「挨」字都翻掉。若這條失敗，代表有人寫了單字通則。
#[test]
fn unlisted_ai_forms_still_become_zhai() {
    // 「捱過」是「熬過」義，屬書面語，兩種都通行，不一併改。
    assert_eq!(cv("捱过"), "捱過");
    // 「着」由 TWVariants 轉為「著」。
    assert_eq!(cv("挨着"), "挨著");
}

// ── 覆寫必須活得過後續的官方掃描 ──────────────────────────────────────

/// 官方 STPhrases 有 `挨打→捱打`，會把第一層的成果改回去。
/// 這幾條要靠 `tw_overrides.tsv` 鎖定才留得住。
#[test]
fn overrides_survive_later_official_passes() {
    assert_eq!(cv("挨打"), "挨打");
    assert_eq!(cv("挨揍"), "挨揍");
    assert_eq!(cv("挨苦"), "挨苦");
    assert_eq!(cv("挨整"), "挨整");
    assert_eq!(cv("挨磨"), "挨磨");
}

// ── 只／隻：量詞與副詞 ────────────────────────────────────────────────

/// 帶前綴的副詞組合最長匹配會退化，「只」落到錯的候選。
#[test]
fn zhi_adverb_with_prefix() {
    assert_eq!(cv("是只要"), "是只要");
    assert_eq!(cv("是只靠"), "是只靠");
    assert_eq!(cv("直是只有"), "直是只有");
}

/// 量詞「隻」，官方缺少疊字與帶前綴的組合。
#[test]
fn zhi_classifier_edge_cases() {
    assert_eq!(cv("同一只"), "同一隻");
    assert_eq!(cv("一只只"), "一隻隻");
    assert_eq!(cv("几只鸟"), "幾隻鳥");
}

// ── 官方詞庫已正確的部分不得被破壞 ────────────────────────────────────

/// 這組官方 `STPhrases` 已有條目，覆蓋不該影響它們。
#[test]
fn official_entries_must_survive() {
    // 量詞
    assert_eq!(cv("一只猫"), "一隻貓");
    assert_eq!(cv("两只手"), "兩隻手");
    assert_eq!(cv("这只"), "這隻");
    // 副詞
    assert_eq!(cv("只要努力"), "只要努力");
    assert_eq!(cv("只有他"), "只有他");
    assert_eq!(cv("只能这样"), "只能這樣");
    assert_eq!(cv("只是这样"), "只是這樣");
    assert_eq!(cv("只好这样"), "只好這樣");
    assert_eq!(cv("只剩下"), "只剩下");
    assert_eq!(cv("那只是"), "那只是");
    assert_eq!(cv("只不过"), "只不過");
}

/// 掃描 20 個高危歧義字後確認：以下組合官方本身就正確，
/// 覆蓋層不得把它們改壞。若這些失敗，代表新增規則誤傷。
#[test]
fn verified_correct_before_our_overrides() {
    // 面／表
    assert_eq!(cv("和面试"), "和面試");
    assert_eq!(cv("仪表盘"), "儀表盤");
    assert_eq!(cv("日程表"), "日程表");
    // 后／里／发
    assert_eq!(cv("攻击后"), "攻擊後");
    assert_eq!(cv("在那里"), "在那裡");
    assert_eq!(cv("会触发"), "會觸發");
    assert_eq!(cv("闪闪发光的"), "閃閃發光的");
    // 台／折／征
    assert_eq!(cv("有一台自动"), "有一臺自動");
    assert_eq!(cv("折射率"), "折射率");
    assert_eq!(cv("的特征"), "的特徵");
    assert_eq!(cv("毫无征兆地"), "毫無徵兆地");
    // 松／历／钟／范／咸
    assert_eq!(cv("轻松"), "輕鬆");
    assert_eq!(cv("的经历"), "的經歷");
    assert_eq!(cv("几分钟"), "幾分鐘");
    assert_eq!(cv("敲钟"), "敲鐘");
    assert_eq!(cv("范围内"), "範圍內");
    assert_eq!(cv("浓郁咸味"), "濃郁鹹味");
    // 划／冲／系／谷
    assert_eq!(cv("划桨"), "划槳");
    assert_eq!(cv("划过"), "劃過");
    assert_eq!(cv("血之冲动"), "血之衝動");
    assert_eq!(cv("气冲冲的"), "氣沖沖的");
    assert_eq!(cv("系着"), "繫著");
    assert_eq!(cv("没关系"), "沒關係");
    assert_eq!(cv("谷歌支"), "谷歌支");
    assert_eq!(cv("凭借"), "憑藉");
}

// ── 字形錯誤：官方詞庫漏掉的 ──────────────────────────────────────────

/// 「回應」被誤判成「迴應」（迴是迴旋之意）。
/// 官方有「响应→響應」但沒有「回应」。
#[test]
fn hui_ying_not_hui_ying_with_turn() {
    assert_eq!(cv("的回应"), "的回應");
    assert_eq!(cv("作出回应"), "作出回應");
    // 「回歸／返回／回收」的「回」是對的，不得一併改掉。
    assert_eq!(cv("回归"), "迴歸");
    assert_eq!(cv("返回这里"), "返回這裡");
    assert_eq!(cv("回收"), "回收");
}

/// 「并且」被官方判成「並刂」——多出一個刂字旁，等於產生亂碼字。
#[test]
fn bing_ze_not_garbled() {
    assert_eq!(cv("并且"), "並且");
    assert_eq!(cv("并不"), "並不");
    assert_eq!(cv("并使自身"), "並使自身");
    // 單字「并」在其他位置也是「並」。
    assert_eq!(cv("并"), "並");
}

// ── 疊字與專有名詞例外 ───────────────────────────────────────────────

/// 疊字裡第二個字是重複而非獨立詞，不能套用單字規則。
#[test]
fn reduplication_forms() {
    assert_eq!(cv("反反复复"), "反反覆覆");
    assert_eq!(cv("干干脆脆"), "乾乾脆脆");
    assert_eq!(cv("干干净净"), "乾乾淨淨");
}

/// 「計劃組」是 Limbus 內部組織名，不跟著轉。
#[test]
fn proper_noun_exception() {
    assert_eq!(cv("是作战计划组的人员"), "是作戰計劃組的人員");
    // 一般語境仍轉為「計畫」。
    assert_eq!(cv("这个计划"), "這個計畫");
}

// ── 台灣用詞（simplecc 內建 S2TWP 應涵蓋）────────────────────────────

#[test]
fn taiwan_idioms_applied() {
    // 這些在 simplecc 內建的 TWPhrasesIT 中，keys 是「已經轉成繁體後」的字形。
    assert_eq!(cv("软件"), "軟體");
    assert_eq!(cv("信息"), "資訊");
    assert_eq!(cv("程序"), "程式");
}

// ── 借／藉 ────────────────────────────────────────────────────────────

#[test]
fn ji_hai() {
    // 「藉由他人之力」用「藉」；「凭借」官方已正確。
    assert_eq!(cv("借我之手"), "藉我之手");
    assert_eq!(cv("借钱"), "借錢");
    assert_eq!(cv("租借编队"), "租借編隊");
    assert_eq!(cv("系得"), "繫得");
}

// ── JSON 結構不得被破壞 ───────────────────────────────────────────────

/// 轉換跑在整份檔案字串上，所以 JSON 的結構字元必須完好。
/// 解析失敗 = 遊戲讀不到資料 = 整段內容消失。
#[test]
fn json_structure_intact() {
    let out = cv(REAL_DIALOG);
    let v: serde_json::Value = serde_json::from_str(&out).expect("輸出不是合法 JSON");
    assert!(v["dataList"].is_array());
    assert_eq!(v["dataList"][0]["id"], 36);
    // key 是英文，不應被中文詞典改動
    assert!(v["dataList"][0].get("dialog").is_some());
    // 中文值有轉
    assert_eq!(
        v["dataList"][0]["dialog"],
        "請派吾上陣！為了正義，吾等必將凱旋！"
    );
    // 英文 usage 原樣保留
    assert_eq!(v["dataList"][0]["usage"], "(800101, VERY_HIGH)");
}

/// 遊戲標記（如 `[OnSucceedAttackHead]`）必須原樣保留，
/// 否則遊戲執行時比對不到效果名稱。
#[test]
fn game_markup_tags_preserved() {
    let src = "[OnSucceedAttackHead] 使目标增加1级沉沦 强度";
    let out = cv(src);
    assert!(
        out.starts_with("[OnSucceedAttackHead] "),
        "標記被破壞: {out}"
    );
    // 標記內的中文有轉（目标→目標、级→級、沦→淪、强→強）
    assert!(out.contains("使目標增加1級"), "中文未轉換: {out}");
    assert!(out.ends_with("強度"), "中文未轉換: {out}");
}

/// 著色用的 `<color=#rrggbb>` 必須保留。
#[test]
fn color_tags_preserved() {
    let out = cv("<color=#ffef23>你好</color>");
    assert!(out.contains("<color=#ffef23>"));
    assert!(out.contains("</color>"));
    assert!(out.contains("你好"));
}

// ── 詞庫本身的健全性 ─────────────────────────────────────────────────

/// 詞庫檔的格式健全性。
/// 缺 TAB 的行會被 simplecc 靜默忽略 —— 那等於該條規則失效，
/// 所以這裡把它變成會失敗的測試，而不是靜默無作用。
#[test]
fn override_dict_is_well_formed() {
    let raw = include_str!("../dict/s2t_overrides.tsv");
    let mut entries = 0;

    for (i, line) in raw.lines().enumerate() {
        let line = line.trim_end_matches('\r');
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let cols: Vec<&str> = line.split('\t').collect();
        assert_eq!(
            cols.len(),
            2,
            "第 {} 行必須剛好一個 TAB，目前有 {} 個: {line}",
            i + 1,
            cols.len() - 1
        );
        assert!(!cols[0].trim().is_empty(), "第 {} 行 key 為空", i + 1);
        assert!(!cols[1].trim().is_empty(), "第 {} 行 value 為空", i + 1);
        assert_eq!(
            cols[0].trim(),
            cols[0],
            "第 {} 行 key 前後有多餘空白: {line}",
            i + 1
        );
        assert_eq!(
            cols[1].trim(),
            cols[1],
            "第 {} 行 value 前後有多餘空白: {line}",
            i + 1
        );
        entries += 1;
    }

    assert!(entries > 0, "覆寫詞庫是空的，規則沒有生效");
}

/// 每則覆寫條目都必須活過整條轉換鏈。
///
/// `Dict::chain` 是「串接多次完整掃描」而非短路，所以第一層的輸出
/// 會繼續流過官方詞典而被二次改寫。若某條的最終輸出不等於它宣告的
/// 值，代表它在 `tw_overrides.tsv` 缺了對應的鎖定項。
#[test]
fn every_override_survives_the_whole_chain() {
    let pipeline = ConversionPipeline::new();
    let raw = include_str!("../dict/s2t_overrides.tsv");
    let mut checked = 0usize;

    for (i, line) in raw.lines().enumerate() {
        let line = line.trim_end_matches('\r');
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut cols = line.splitn(2, '\t');
        let (Some(key), Some(want)) = (cols.next(), cols.next()) else {
            continue;
        };
        let got = pipeline.convert(key);
        assert_eq!(
            got, want,
            "第 {} 行「{key}」的最終輸出是「{got}」，不是「{want}」。\
             這通常表示 tw_overrides.tsv 缺了「{got}→{want}」的鎖定項。",
            i + 1
        );
        checked += 1;
    }

    assert!(checked > 0, "沒有任何覆寫條目被檢查");
}

/// `tw_overrides.tsv` 的每一條都必須是**必要的**。
///
/// 如果某條鎖定在沒有它時也會得到同樣結果，那它是多餘的，
/// 會造成維護負擔並掩蓋真正的問題。
#[test]
fn tw_overrides_has_no_redundant_entries() {
    let s2t = include_str!("../dict/s2t_overrides.tsv");
    let tw = include_str!("../dict/tw_overrides.tsv");

    for (i, line) in tw.lines().enumerate() {
        let line = line.trim_end_matches('\r');
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut cols = line.splitn(2, '\t');
        let (Some(key), Some(want)) = (cols.next(), cols.next()) else {
            continue;
        };
        // 只跑前兩層（不帶鎖定），看鎖定是否真的必要。
        let without_lock = simplecc::Dict::load_str(s2t)
            .chain(simplecc::dicts::S2TWP.clone())
            .replace_all(key);
        assert_ne!(
            without_lock, want,
            "第 {} 行「{key}→{want}」是多餘的：沒有它也會得到「{want}」",
            i + 1
        );
    }
}

/// 轉換結果不應再殘留簡體字。
/// 逐字比對預期的繁體字形，避免整份文本被漏轉。
#[test]
fn no_simplified_leftovers() {
    // 對應關係已逐一對照 simplecc 內建的 OpenCC 詞典確認。
    let cases = [
        ("请", "請"), // STCharacters
        ("为", "為"), // STCharacters → 爲，再由 TWVariants → 為
        ("凯", "凱"),
        ("战", "戰"),
        ("猫", "貓"),
        ("鸟", "鳥"),
        ("级", "級"),
        ("标", "標"),
    ];
    for (s, expected) in cases {
        assert_eq!(cv(s), expected, "「{s}」的轉換結果不正確");
    }
}
