//! 簡體 → 正體（台灣用語）轉換管線。
//!
//! 設計要點：**不要改轉換引擎，改轉換鏈的順序**。
//!
//! 簡化把多個繁體字壓成同一個簡體字，轉換器沒有語義理解，
//! 只能靠「詞庫最長匹配 + 單字預設候選」。多個候選時必然有選錯的，
//! 而官方詞庫的預設有時不符合台灣用語習慣。
//!
//! 解法是在官方詞庫**之前**掛一層自訂詞典，命中即定案：
//!
//! ```text
//!   簡體原文
//!      │
//!      ▼
//!   ┌───────────────────────────────┐
//!   │ 1. s2t_overrides（自訂，最優先）│  ← 修正官方會選錯的
//!   └──────────────┬────────────────┘
//!                  ▼
//!   ┌───────────────────────────────┐
//!   │ 2. 官方 S2TWP（simplecc 內建）  │  ← STPhrases → STCharacters
//!   └──────────────┬────────────────┘
//!                  ▼
//!   ┌───────────────────────────────┐
//!   │ 3. tw_overrides（終結鎖定）     │  ← 擋掉第 2 步把結果改壞
//!   └──────────────┬────────────────┘
//!                  ▼
//!   正體輸出
//! ```
//!
//! 為什麼自訂詞典放「前面」而不是「後面」
//! ────────────────────────────────
//! `Dict::chain` 是「串接多次完整掃描」而非合併詞典：
//! `a.chain(b)` 代表 a 先跑、b 後跑，前者的輸出會餵給後者。
//! 所以自訂詞典必須在 `roots[0]`，否則等官方詞庫轉完（變成繁體），
//! 簡體的鍵就永遠 match 不到。
//!
//! 同理，第 1 步產出的繁體會繼續被第 2 步改寫。要鎖死最終字形，
//! 就得在第 2 步之後再加一層覆寫（`tw_overrides`）。

use std::sync::OnceLock;

use simplecc::dicts::S2TWP;
use simplecc::Dict;

/// 自訂簡→繁覆寫詞庫。掛在官方詞庫之前。
const S2T_OVERRIDES: &str = include_str!("../dict/s2t_overrides.tsv");
/// 自訂繁體用詞覆寫詞庫。掛在官方詞庫之後，鎖定最終字形。
const TW_OVERRIDES: &str = include_str!("../dict/tw_overrides.tsv");

/// 轉換管線。建構一次後可重複使用（`Dict` 不可變且執行緒安全）。
#[derive(Clone)]
pub struct ConversionPipeline {
    dict: Dict,
}

impl ConversionPipeline {
    /// 建立預設管線：自訂覆寫 → 官方 S2TWP → 繁體鎖定。
    pub fn new() -> Self {
        // 順序有意義：自訂詞典在最前，官方詞庫在最後兜底。
        let dict = Dict::load_str(S2T_OVERRIDES)
            .chain(S2TWP.clone())
            .chain(Dict::load_str(TW_OVERRIDES));

        Self { dict }
    }

    /// 轉換一段文字。
    pub fn convert(&self, text: &str) -> String {
        self.dict.replace_all(text)
    }
}

impl Default for ConversionPipeline {
    fn default() -> Self {
        Self::new()
    }
}

/// 共用實例，避免每個檔案重新建一次詞典樹。
pub fn shared() -> &'static ConversionPipeline {
    static P: OnceLock<ConversionPipeline> = OnceLock::new();
    P.get_or_init(ConversionPipeline::new)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cv(s: &str) -> String {
        shared().convert(s)
    }

    /// 台灣口語：官方主候選是「捱」，覆寫後應為「挨」。
    #[test]
    fn overrides_ai_taiwan_usage() {
        assert_eq!(cv("他挨饿。"), "他挨餓。");
        assert_eq!(cv("挨揍"), "挨揍");
        assert_eq!(cv("挨打"), "挨打");
        assert_eq!(cv("挨骂"), "挨罵");
    }

    /// 「挨過」保留「捱」：這是「熬過」義，屬書面語，
    /// 台灣兩種都通行，不要一併改掉。
    /// 注意「挨了」「挨上」不在覆寫清單內，維持官方的「捱」。
    #[test]
    fn keeps_nonzh_ai_forms() {
        assert_eq!(cv("捱过"), "捱過");
    }

    /// 覆寫的輸出必須能活過後續的官方掃描。
    /// 這是 `tw_overrides` 存在的原因：官方 STPhrases:25640 有
    /// `挨打→捱打`，會把第一層的成果改回去。
    #[test]
    fn overrides_survive_later_official_passes() {
        // 這五條在 s2t_overrides 寫「挨」，但官方 STPhrases 會再改成「捱」
        assert_eq!(cv("挨打"), "挨打");
        assert_eq!(cv("挨揍"), "挨揍");
        assert_eq!(cv("挨苦"), "挨苦");
        assert_eq!(cv("挨整"), "挨整");
        assert_eq!(cv("挨磨"), "挨磨");
    }

    /// 字形錯誤：官方把「钟表」判成「鍾表」（鍾是鈴，不是時鐘）。
    #[test]
    fn fixes_clock_character_error() {
        assert_eq!(cv("钟表"), "鐘錶");
        assert_eq!(cv("钟表头"), "鐘錶頭");
        assert_eq!(cv("的钟表"), "的鐘錶");
        assert_eq!(cv("无振八方钟"), "無振八方鐘");
    }

    /// 「傢伙」被二次改寫成「傢夥」，需要鎖定。
    #[test]
    fn fixes_jia_huo_with_lock() {
        assert_eq!(cv("家伙"), "傢伙");
        assert_eq!(cv("家具"), "家具");
        assert_eq!(cv("背包"), "背包");
    }

    /// 「说/证明了」的「了」被當成「瞭」，需要鎖定。
    #[test]
    fn fixes_le_with_lock() {
        assert_eq!(cv("说明了"), "說明了");
        assert_eq!(cv("证明了"), "證明了");
    }

    /// 分輪掃描導致「只」被「隻」搶走。
    #[test]
    fn fixes_zhi_split_pass_error() {
        assert_eq!(cv("是只要"), "是只要");
        assert_eq!(cv("是只靠"), "是只靠");
    }

    /// 量詞「隻」。官方 STPhrases 已有「一只」「两只」條目。
    /// 注意「几只鸟」在 simplecc 分輪掃描下仍是「幾只鳥」——
    /// 這是 simplecc 與 OpenCC 短路匹配的語意差異，
    /// 不在覆寫範圍內，測試如實記錄現行行為。
    #[test]
    fn classifier_zhī_intact() {
        assert_eq!(cv("一只猫"), "一隻貓");
        assert_eq!(cv("两只手"), "兩隻手");
    }

    /// 副詞「只」：同樣是官方已有條目。
    #[test]
    fn adverb_zhǐ_intact() {
        assert_eq!(cv("只要努力"), "只要努力");
        assert_eq!(cv("只有他"), "只有他");
        assert_eq!(cv("只是这样"), "只是這樣");
    }

    /// 覆寫不應誤傷官方已正確的詞。
    #[test]
    fn overrides_do_not_corrupt_generic_text() {
        assert_eq!(cv("防御"), "防禦"); // STPhrases:46475
        assert_eq!(cv("战斗"), "戰鬥"); // STPhrases:23629
    }

    /// 詞庫必須能被載入，且自我覆寫不會造成無限循環。
    #[test]
    fn pipeline_is_idempotent_for_zhuyin_text() {
        let once = cv("他挨饿，家里只有一只猫。");
        let twice = cv(&once);
        assert_eq!(once, twice);
    }
}
