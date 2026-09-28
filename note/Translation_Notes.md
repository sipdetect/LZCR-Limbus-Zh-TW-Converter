# Limbus Company 翻譯與修正筆記

本檔案記錄了在翻譯和校對過程中需要注意的特殊術語、翻譯規則以及顏色標記標準。

## 1. 術語修正 (Terminology Fixes)

| 原文/錯誤翻譯 | 修正後翻譯 | 備註 |
| :--- | :--- | :--- |
| **虹原** | **鴻園** | 對應 "Hong Yuan" 或 "Grand Garden" (特殊鏡像世界術語)。 |
| **領主 (Lord)** | **家主 (Family Head)** | 在劍契/鴻園/黑雲會等特定語境下，Lord 應翻譯為「家主」而非「領主」。 |
| **吸血鬼** | **血魔** | 官方術語判定。 |
| **幹脆** | **乾脆** | 繁體中文用字修正 (幹/乾區分)。 |

## 2. 翻譯規則 (Translation Rules)

- **技能/特效名稱中英對照**：
  - 當遇到英文技能名後跟括號內的漢字時 (如 `English Name(Kanji)` )，需將括號外的英文翻譯為中文，保留括號內的漢字作為對照或補充。
  - 範例： 
    - 原文：`Flame Rooster's Death Defiance(炎鳥不死戦)`
    - 翻譯：`炎鳥不死戰(炎鳥不死戦)`
  - 範例：
    - 原文：`Rooster’s Rampaging Blades Under the Ensanguined Heaven(血天下雞舞亂刀)`
    - 翻譯：`血染天下雞舞亂刀(血天下雞舞亂刀)`

- **標點符號**：
  - 翻譯後的文本應使用全形標點符號 (，。？！……)。

## 3. 顏色標記 (Color Code Standards)

翻譯對話時，應根據角色身份加上對應的顏色標籤 `<color=#xxxxxx>...</color>`。

| 顏色代碼 | 角色/陣營 | 範例 ID |
| :--- | :--- | :--- |
| **#cf0000** | **良秀 (Ryoshu)** | 104xx, 10714 (Chef), 漂泊之刃 |
| **#a60000** | **羅佳 (Rodion)** | 109xx, R Corp, 黑雲會 |
| **#ff9500** | **以實瑪利 (Ishmael)** | 108xx |
| **#6e44a6** | **希斯克利夫 (Heathcliff)** | 107xx |
| **#ffef23** | **堂吉訶德 (Don Quixote)** | 103xx |
| **#d4dfe8** | **李箱 (Yi Sang)** | 101xx |
| **#fa5a29** | **浮士德 (Faust)** | 102xx |
| **#293b95** | **默爾索 (Meursault)** | 105xx |
| **#5bffde** | **鴻路 (Hong Lu) / 鴻園** | 106xx |
| **#94e619** | **辛克萊 (Sinclair)** | 110xx |
| **#4c4945** | **奧提斯 (Outis)** | 111xx |
| **#a0522d** | **格里高爾 (Gregor)** | 112xx |

> 詳細顏色分析請參考 `Color_Analysis.md`。

## 4. 特殊 ID 註記

- **ID 10714**: 雖然編號以 107 (Heathcliff) 開頭，但在特定上下文中(如 `Rooster`) 若對話內容明顯屬於良秀 (Ryoshu)，應使用良秀的顏色 `#cf0000`。
- **ID 10413**: 漂泊之刃良秀 (Drifting Blade Ryoshu)，應使用良秀顏色 `#cf0000`。
