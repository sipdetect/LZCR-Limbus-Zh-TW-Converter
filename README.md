<p align="center"> LZCR - Limbus Company 零協會文本正體中文轉換工具 </p>

<p align="center">   <img width="237" height="190" alt="LZCR" src="https://github.com/user-attachments/assets/7c923dbf-a1d4-449e-add6-eb8411bed15b" /> </p>

---
<img width="852" height="431" alt="{C245EE0B-2ED8-4DCE-A2C8-114499105A22}" src="https://github.com/user-attachments/assets/a6d80803-b19b-4ba2-b7d4-48ecafd6a072" />

執行後會自動將遊戲改為正體中文。請在 Steam 安裝好遊戲後再執行此程式。

每次執行都會**強制**從 [LocalizeLimbusCompany](https://github.com/LocalizeLimbusCompany/LocalizeLimbusCompany) 下載最新簡中套件，簡→正體後**覆寫**安裝至遊戲 `Lang/LLC_zh-Hant`，並為戰鬥語音氣泡上色。

## 更新速度

遊戲劇情文本：零協會的文本釋出時間
<https://github.com/LocalizeLimbusCompany/LocalizeLimbusCompany>

戰鬥氣泡文本：這邊也同時改為跟零協會同步的文本，同時通過正則表達式在轉換途中幫戰鬥氣泡上屬於罪人的代表色

## 解決Bug 或 解除安裝本程式
如有 卡住 或是 安裝過其他漢化 可以透過下面的動作移除該資料夾後再使用本程式

<img width="718" height="897" alt="圖片" src="https://github.com/user-attachments/assets/857169dc-bee3-40ca-afdb-5593fdc850b9" />


## 軟體 Icon

* 提供者   : Nina妮娜
* 聯繫方式 : Nianqichen592@gmail[.]com


## 安全

<img width="1131" height="656" alt="{AD117E40-3AFB-4204-A84D-23AF344E64F8}" src="https://github.com/user-attachments/assets/fd6b1618-76a0-418a-8e65-b1173ec103fb" />

基本上無問題，如果有安全疑慮可以審視程式碼並且自己構建

## 轉換管線

簡體轉繁體是「一簡對多繁」的問題：一個簡體字對應多個繁體字，
而轉換器沒有語義理解，只能靠「詞庫最長匹配 + 單字預設候選」。
多個候選時必然有選錯的，且官方詞庫的預設不一定符合台灣用語。

```
簡體原文
   │
   ├─▶ ① s2t_overrides.tsv   61 條   命中即定案
   │      修正官方會選錯的：鍾表→鐘錶、心象→心像、并→並
   │
   ├─▶ ② simplecc S2TWP      官方內建詞典（合併 Trie，最長匹配）
   │      沒命中才走到這裡
   │
   └─▶ ③ tw_overrides.tsv    16 條   鎖定最終字形
          擋掉 ① 的成果被 ② 改回去
   │
   ▼
正體輸出
```

## 完整流程

```
按 S / --headless
        │
        ▼
┌───────────────────┐
│ 1. 初始化          │  偵測 Steam 遊戲目錄、Lang 輸出路徑
└─────────┬─────────┘
          ▼
┌───────────────────┐
│ 2. 下載 / 解壓     │  GitHub releases/latest → ZIP
│                   │  取出 Lang/LLC_zh-CN/**/*.json
│                   │  （版本相同也強制重下）
└─────────┬─────────┘
          ▼
┌───────────────────┐
│ 3. 文字轉換        │  簡體 → 正體（台灣用語）
│                   │  自訂覆寫 → simplecc S2TWP → 鎖定
│                   │  詳見「轉換管線」一節
│                   │  寫入 LLC_zh-Hant/** 強制覆寫
└─────────┬─────────┘
          ▼
┌───────────────────┐
│ 4. 氣泡／語音著色  │  BattleSpeechBubble*.json
│                   │  PersonalityVoiceDlg/Voice_*.json
│                   │  罪人／Boss 代表色 <color=#…>
│                   │  force 重套外層色
└─────────┬─────────┘
          ▼
┌───────────────────┐
│ 5. 收尾            │  覆寫 Font/、更新 lzcr-info.json
│                   │  config.json 僅在不存在時建立
│                   │  清理暫存目錄
└───────────────────┘
```

| 步驟 | 做什麼 | 覆蓋？ |
|------|--------|--------|
| 下載 | LLC 最新 Release ZIP | 每次都下載 |
| 轉換 | `LLC_zh-CN` → `LLC_zh-Hant` JSON | **強制覆寫**同路徑檔 |
| 氣泡 | `BattleSpeechBubble*.json` 上色 | **強制重套**外層色 |
| 語音 | `PersonalityVoiceDlg` 罪人色 | **強制重套**外層色 |
| 字型 | 內嵌 `Font/` → 輸出 `Font/` | 先刪再寫 |
| 語言設定 | `Lang/config.json` | **不覆寫**既有（避免改掉使用者選語） |
| 版本記錄 | `Lang/lzcr-info.json` | 覆寫 tag／日期 |

## 功能

- 自動偵測 Steam 安裝的 Limbus Company 路徑
- **每次執行強制下載** LocalizeLimbusCompany 最新套件（不因版本相同跳過）
- 簡體中文 JSON → 台灣正體中文，**強制覆寫**輸出檔
- 自訂覆寫詞庫修正官方詞典的字形錯誤與用詞差異（見「轉換管線」）
- **戰鬥語音氣泡** + **人格語音**（`PersonalityVoiceDlg`）自動上色（對齊 LLC `syler` 規則）
- 內嵌字型、TUI 進度與日誌

## 專案結構

```
lzcr/
  dict/
    s2t_overrides.tsv    ① 簡體層覆寫（61 條）
    tw_overrides.tsv     ③ 繁體層鎖定（16 條）
  src/
    conversion.rs        轉換管線（ConversionPipeline）
    converter.rs         下載／解壓／轉換／著色的流程控制
  tests/
    conversion_test.rs   詞庫回歸測試
```

## 使用方式

### TUI 模式（預設）

```cmd
lzcr.exe
```

| 按鍵 | 功能 |
|------|------|
| `S` | 開始轉換（完整強制流程） |
| `X` | 取消進行中的轉換 |
| `R` | 重新載入設定與遊戲路徑 |
| `Q` / `Esc` | 離開（執行中則取消） |
| `↑` `↓` | 捲動日誌 |
| `PageUp` `PageDown` | 快速捲動日誌 |
| `Home` / `End` | 跳到日誌最舊 / 最新 |

### 無介面模式

```cmd
lzcr.exe --headless
```

適合腳本或排程自動執行（同樣強制下載與覆寫）。

## 建置

需要安裝 [Rust](https://rustup.rs/)。

```cmd
cd lzcr
cargo build --release
```

或使用專案根目錄的 `build_and_copy_to_desktop.cmd`（會複製到**系統實際桌面**的 `LZCR-Build`）。


## 測試

```cmd
cd lzcr
cargo test
```

單元測試可在離線環境執行。需要 GitHub 連線的整合測試預設略過：

```cmd
cargo test -- --ignored
```

### 詞庫回歸測試

`tests/conversion_test.rs` 是詞庫的安全網。自訂詞典的每一條都能改變
任意檔案任意位置的輸出，一條寫錯就是全玩家看到壞掉的正體中文，
而且不會有任何錯誤訊息。

其中兩個是「元測試」，會自動守住整個詞庫：

| 測試 | 作用 |
|------|------|
| `every_override_survives_the_whole_chain` | 逐條驗證 `s2t_overrides.tsv` 的最終輸出等於宣告值；失敗時直接指出 `tw_overrides.tsv` 缺哪個鎖定項 |
| `tw_overrides_has_no_redundant_entries` | 驗證每個鎖定都真的必要，防止未來累積多餘條目 |

**新增或修改 `dict/*.tsv` 後，必須跑一次 `cargo test`。**

## 授權

MIT License
