use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use include_dir::{include_dir, Dir};
use reqwest::blocking::Client;
use serde::Deserialize;
use zip::ZipArchive;

use crate::colorize_bubble;
use crate::colorize_voice;
use crate::config::{self, Config, TranslationConfig};
use crate::conversion::ConversionPipeline;
use crate::error::AppError;
use crate::steam_registry;
use crate::util::{format_bytes, normalize_release_date};

static FONT_DIR: Dir<'_> = include_dir!("Font");

const USER_AGENT: &str = "LZCR-TUI/2.0";

macro_rules! clog {
    ($this:expr, $($arg:tt)*) => {
        if $this.progress_callback.is_none() {
            println!($($arg)*);
        }
    };
}

#[derive(Debug)]
struct FileInfo {
    relative_path: PathBuf,
}

#[derive(Debug, Deserialize)]
pub struct GitHubRelease {
    pub tag_name: String,
    pub published_at: Option<String>,
    pub created_at: Option<String>,
    pub assets: Vec<ReleaseAsset>,
}

#[derive(Debug, Deserialize)]
pub struct ReleaseAsset {
    pub name: String,
    pub browser_download_url: String,
}

pub type ProgressCallback =
    Box<dyn Fn(f64, String, Option<String>, Option<usize>, Option<usize>) + Send>;

pub struct Converter {
    config: Config,
    client: Client,
    pipeline: ConversionPipeline,
    progress_callback: Option<ProgressCallback>,
    cancel_flag: Option<Arc<AtomicBool>>,
    temp_dir: PathBuf,
}

impl Converter {
    pub fn new() -> Result<Self, AppError> {
        let config = config::load_config()?;
        let client = Client::builder()
            .user_agent(USER_AGENT)
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .map_err(|e| AppError::Other(format!("無法建立 HTTP 用戶端: {e}")))?;

        let temp_dir = std::env::temp_dir().join(format!("lzcr-{}", std::process::id()));

        Ok(Self {
            config,
            client,
            pipeline: ConversionPipeline::new(),
            progress_callback: None,
            cancel_flag: None,
            temp_dir,
        })
    }

    pub fn new_with_callback_and_cancel(
        callback: ProgressCallback,
        cancel_flag: Arc<AtomicBool>,
    ) -> Result<Self, AppError> {
        let mut converter = Self::new()?;
        converter.progress_callback = Some(callback);
        converter.cancel_flag = Some(cancel_flag);
        Ok(converter)
    }

    fn report_progress(
        &self,
        progress: f64,
        message: String,
        current_file: Option<String>,
        total_files: Option<usize>,
        processed_files: Option<usize>,
    ) {
        if let Some(ref callback) = self.progress_callback {
            callback(
                progress,
                message,
                current_file,
                total_files,
                processed_files,
            );
        }
    }

    fn check_cancelled(&self) -> Result<(), AppError> {
        if let Some(cancel_flag) = &self.cancel_flag {
            if cancel_flag.load(Ordering::Relaxed) {
                return Err(AppError::Cancelled);
            }
        }
        Ok(())
    }

    fn fetch_latest_release(&self) -> Result<GitHubRelease, AppError> {
        self.check_cancelled()?;
        let api_url = format!(
            "https://api.github.com/repos/{}/{}/releases/latest",
            self.config.repo_owner, self.config.repo_name
        );
        clog!(self, "[INFO] 連線至 GitHub API: {api_url}");
        self.report_progress(
            5.0,
            "正在連線至 GitHub API...".to_string(),
            None,
            None,
            None,
        );

        let response = self
            .client
            .get(&api_url)
            .header("User-Agent", USER_AGENT)
            .send()
            .map_err(AppError::Network)?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .unwrap_or_else(|_| "無法讀取錯誤訊息".to_string());
            return Err(AppError::Other(format!(
                "GitHub API 回傳錯誤 {status}: {error_text}"
            )));
        }

        let release: GitHubRelease = response
            .json()
            .map_err(|e| AppError::Other(format!("無法解析 Release 資訊: {e}")))?;

        clog!(
            self,
            "[INFO] 最新 Release 版本: {}",
            release.tag_name
        );
        self.report_progress(
            10.0,
            format!("找到最新版本: {}", release.tag_name),
            None,
            None,
            None,
        );
        Ok(release)
    }

    fn download_zip(&self, download_url: &str) -> Result<Vec<u8>, AppError> {
        self.check_cancelled()?;
        clog!(self, "[INFO] 正在從 GitHub 下載 Release 資料...");
        self.report_progress(
            15.0,
            "正在下載更新資料...".to_string(),
            None,
            None,
            None,
        );

        let mut response = self
            .client
            .get(download_url)
            .header("User-Agent", USER_AGENT)
            .send()
            .map_err(AppError::Network)?;

        let status = response.status();
        if !status.is_success() {
            return Err(AppError::Other(format!(
                "下載失敗，HTTP 狀態: {status}"
            )));
        }

        let total_size = response
            .headers()
            .get(reqwest::header::CONTENT_LENGTH)
            .and_then(|ct_len| ct_len.to_str().ok())
            .and_then(|ct_len| ct_len.parse::<u64>().ok())
            .unwrap_or(0);

        let mut buffer = Vec::new();
        let mut downloaded = 0u64;
        let mut chunk = vec![0; 8192];

        loop {
            self.check_cancelled()?;
            let bytes_read = response.read(&mut chunk).map_err(AppError::Io)?;
            if bytes_read == 0 {
                break;
            }

            buffer.extend_from_slice(&chunk[..bytes_read]);
            downloaded += bytes_read as u64;

            let download_progress = if total_size > 0 {
                15.0 + (downloaded as f64 / total_size as f64) * 25.0
            } else {
                15.0 + (downloaded as f64 / 10_000_000.0).min(1.0) * 25.0
            };

            self.report_progress(
                download_progress,
                format!(
                    "下載中... {} / {}",
                    format_bytes(downloaded),
                    format_bytes(total_size)
                ),
                None,
                None,
                None,
            );
        }

        self.report_progress(40.0, "下載完成".to_string(), None, None, None);
        Ok(buffer)
    }

    fn extract_files(&self, zip_data: Vec<u8>) -> Result<Vec<FileInfo>, AppError> {
        self.check_cancelled()?;
        clog!(self, "[INFO] 正在從壓縮檔解壓...");
        self.report_progress(45.0, "正在解壓檔案...".to_string(), None, None, None);

        if self.temp_dir.exists() {
            fs::remove_dir_all(&self.temp_dir)?;
        }
        fs::create_dir_all(&self.temp_dir)?;

        let cursor = std::io::Cursor::new(zip_data);
        let mut archive = ZipArchive::new(cursor)?;
        let mut files = Vec::new();

        let mut root_dir = String::new();
        for i in 0..archive.len() {
            let file = archive.by_index(i)?;
            let file_path = file.name();
            if let Some(first_slash) = file_path.find('/') {
                root_dir = file_path[..first_slash].to_string();
                clog!(self, "[INFO] 偵測到 ZIP 根目錄: {root_dir}");
                break;
            }
        }

        let target_folder = if root_dir.is_empty() {
            self.config.source_folder.clone()
        } else {
            format!("{root_dir}/{}", self.config.source_folder)
        };

        clog!(self, "[INFO] 尋找來源資料夾: {target_folder}");

        let total_files = archive.len();
        let mut processed = 0usize;

        for i in 0..archive.len() {
            if i % 200 == 0 {
                self.check_cancelled()?;
            }

            let mut file = match archive.by_index(i) {
                Ok(f) => f,
                Err(e) => {
                    clog!(self, "[WARN] 無法存取索引 {i} 的檔案: {e}");
                    continue;
                }
            };

            let file_path_str = file.name().to_string();
            if !file_path_str.starts_with(&target_folder) || !file_path_str.ends_with(".json") {
                continue;
            }

            let relative_str = if file_path_str.starts_with(&format!("{target_folder}/")) {
                &file_path_str[target_folder.len() + 1..]
            } else {
                &file_path_str[target_folder.len()..]
            };

            let relative_path = PathBuf::from(relative_str);
            let mut content = Vec::new();
            if let Err(e) = file.read_to_end(&mut content) {
                clog!(
                    self,
                    "[ERROR] 無法讀取 {file_path_str}: {e}"
                );
                continue;
            }

            let temp_path = self.temp_dir.join(&relative_path);
            if let Some(parent) = temp_path.parent() {
                if let Err(e) = fs::create_dir_all(parent) {
                    clog!(
                        self,
                        "[ERROR] 無法建立目錄 {}: {}",
                        temp_path.display(),
                        e
                    );
                    continue;
                }
            }

            if let Err(e) = fs::write(&temp_path, &content) {
                clog!(
                    self,
                    "[ERROR] 無法寫入 {}: {}",
                    temp_path.display(),
                    e
                );
                continue;
            }

            files.push(FileInfo { relative_path });
            processed += 1;

            if processed % 100 == 0 {
                let extract_progress = 45.0 + (processed as f64 / total_files as f64) * 10.0;
                self.report_progress(
                    extract_progress,
                    format!("解壓中... {processed} 個檔案"),
                    None,
                    None,
                    None,
                );
            }
        }

        clog!(
            self,
            "[INFO] 解壓完成，找到 {} 個 JSON 檔案",
            files.len()
        );
        self.report_progress(
            55.0,
            format!("找到 {} 個待轉換檔案", files.len()),
            None,
            None,
            None,
        );
        Ok(files)
    }

    fn convert_file(&self, input_path: &Path, output_path: &Path) -> Result<(), AppError> {
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).map_err(AppError::Io)?;
        }

        let bytes = fs::read(input_path).map_err(AppError::Io)?;
        let content = String::from_utf8(bytes).unwrap_or_else(|e| {
            clog!(
                self,
                "[WARN] {} 不是有效 UTF-8，使用 lossy 轉換",
                input_path.display()
            );
            String::from_utf8_lossy(e.as_bytes()).into_owned()
        });

        let converted = self.pipeline.convert(&content);
        // 一律覆寫既有檔（不保留舊內容）
        fs::write(output_path, converted.as_bytes()).map_err(AppError::Io)?;
        Ok(())
    }

    fn process_files(&self, files: Vec<FileInfo>) -> Result<(), AppError> {
        clog!(self, "[INFO] 開始轉換檔案...");
        let total_files = files.len();
        self.report_progress(
            60.0,
            "開始轉換檔案...".to_string(),
            None,
            Some(total_files),
            Some(0),
        );

        let mut conversion_errors = 0usize;
        for (index, file_info) in files.iter().enumerate() {
            self.check_cancelled()?;

            let source_file = self.temp_dir.join(&file_info.relative_path);
            let output_file = Path::new(&self.config.output_base).join(&file_info.relative_path);

            let file_name = file_info
                .relative_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            let convert_progress = if total_files > 0 {
                60.0 + (index as f64 / total_files as f64) * 22.0
            } else {
                82.0
            };
            self.report_progress(
                convert_progress,
                format!("轉換中... {} / {} 個檔案", index + 1, total_files),
                Some(file_name),
                Some(total_files),
                Some(index + 1),
            );

            if let Err(e) = self.convert_file(&source_file, &output_file) {
                conversion_errors += 1;
                clog!(
                    self,
                    "[WARN] 轉換失敗 {}: {}",
                    file_info.relative_path.display(),
                    e
                );
                if conversion_errors > 10 {
                    clog!(self, "[ERROR] 轉換錯誤過多，停止處理");
                    return Err(e);
                }
            }
        }

        clog!(
            self,
            "[INFO] 轉換完成，共 {conversion_errors} 個錯誤"
        );
        self.report_progress(
            82.0,
            "檔案轉換完成！".to_string(),
            None,
            Some(total_files),
            Some(total_files),
        );
        Ok(())
    }

    /// 對輸出目錄內 `BattleSpeechBubble*.json` 依 syler 規則著色。
    fn colorize_speech_bubbles(&self) -> Result<(), AppError> {
        self.check_cancelled()?;
        let out = Path::new(&self.config.output_base);
        clog!(
            self,
            "[INFO] 正在為戰鬥語音氣泡著色: {}",
            out.display()
        );
        self.report_progress(
            84.0,
            "正在為戰鬥語音氣泡著色...".to_string(),
            None,
            None,
            None,
        );

        let stats = colorize_bubble::colorize_directory(out, true)?;
        clog!(
            self,
            "[OK] 氣泡著色完成: files={} total={} +{} recolor={} already={} no_color={}",
            stats.files,
            stats.total,
            stats.wrapped,
            stats.recolored,
            stats.skipped_already,
            stats.no_color
        );
        self.report_progress(
            87.0,
            format!(
                "氣泡著色完成（{} 檔，上色 {}，無色規則 {}）",
                stats.files,
                stats.wrapped + stats.recolored,
                stats.no_color
            ),
            None,
            None,
            None,
        );
        Ok(())
    }

    /// 對 `PersonalityVoiceDlg` 人格語音依罪人代表色著色。
    fn colorize_personality_voices(&self) -> Result<(), AppError> {
        self.check_cancelled()?;
        let voice_dir = Path::new(&self.config.output_base).join("PersonalityVoiceDlg");
        if !voice_dir.is_dir() {
            clog!(
                self,
                "[WARN] 找不到 PersonalityVoiceDlg，跳過語音著色: {}",
                voice_dir.display()
            );
            self.report_progress(
                90.0,
                "無 PersonalityVoiceDlg，跳過語音著色".to_string(),
                None,
                None,
                None,
            );
            return Ok(());
        }

        clog!(
            self,
            "[INFO] 正在為人格語音著色: {}",
            voice_dir.display()
        );
        self.report_progress(
            88.0,
            "正在為人格語音（PersonalityVoiceDlg）著色...".to_string(),
            None,
            None,
            None,
        );

        let stats = colorize_voice::colorize_directory(&voice_dir, true)?;
        clog!(
            self,
            "[OK] 語音著色完成: files={} total={} +{} recolor={} already={} no_color={}",
            stats.files,
            stats.total,
            stats.wrapped,
            stats.recolored,
            stats.skipped_already,
            stats.no_color
        );
        self.report_progress(
            91.0,
            format!(
                "語音著色完成（{} 檔，上色 {}，無色規則 {}）",
                stats.files,
                stats.wrapped + stats.recolored,
                stats.no_color
            ),
            None,
            None,
            None,
        );
        Ok(())
    }

    fn write_font_folder(&self) -> Result<(), AppError> {
        self.check_cancelled()?;
        self.report_progress(92.0, "正在複製字型檔...".to_string(), None, None, None);

        let out_font_dir = Path::new(&self.config.output_base).join("Font");
        if out_font_dir.exists() {
            fs::remove_dir_all(&out_font_dir)?;
        }

        self.write_dir(&FONT_DIR, &out_font_dir)?;
        clog!(
            self,
            "[INFO] 字型已匯出至 {}",
            out_font_dir.display()
        );
        self.report_progress(
            95.0,
            "字型複製完成！".to_string(),
            None,
            None,
            None,
        );
        Ok(())
    }

    fn write_dir(&self, dir: &Dir, path: &Path) -> Result<(), AppError> {
        fs::create_dir_all(path)?;

        for file in dir.files() {
            let file_path = path.join(file.path().file_name().unwrap());
            fs::write(&file_path, file.contents())?;
        }

        for subdir in dir.dirs() {
            let sub_path = path.join(subdir.path().file_name().unwrap());
            self.write_dir(subdir, &sub_path)?;
        }

        Ok(())
    }

    fn show_installation_info(&self) -> Result<(), AppError> {
        self.check_cancelled()?;
        clog!(self, "[INFO] 正在尋找 Limbus Company 遊戲目錄...");
        self.report_progress(
            2.0,
            "正在尋找遊戲目錄...".to_string(),
            None,
            None,
            None,
        );

        match steam_registry::find_limbus_company_path() {
            Ok(game_path) => {
                clog!(self, "[OK] 找到遊戲目錄: {}", game_path.display());
                self.report_progress(3.0, "已找到遊戲目錄".to_string(), None, None, None);
            }
            Err(e) => {
                clog!(self, "[WARN] 找不到遊戲目錄: {e}");
                clog!(
                    self,
                    "   使用預設輸出目錄: {}",
                    self.config.output_base
                );
                self.report_progress(
                    3.0,
                    "找不到遊戲目錄，使用預設位置".to_string(),
                    None,
                    None,
                    None,
                );
            }
        }

        Ok(())
    }

    pub fn run(&mut self) -> Result<(), AppError> {
        clog!(self, "[INFO] ===== 轉換流程開始（強制覆蓋） =====");
        self.show_installation_info()?;
        self.check_cancelled()?;

        // 每次執行都完整重跑：下載 → 轉換覆蓋 → 氣泡著色 → 字型
        // （不再因版本相同而跳過下載／寫入）
        let release = self.fetch_latest_release()?;
        let latest_release_tag = release.tag_name.clone();
        let latest_release_date = normalize_release_date(
            release
                .published_at
                .as_deref()
                .or(release.created_at.as_deref()),
        );

        if let Ok(false) = config::should_update(&latest_release_tag) {
            clog!(
                self,
                "[INFO] 遠端版本 {} 與本地相同，仍強制重新下載並覆蓋",
                latest_release_tag
            );
        }

        clog!(
            self,
            "[INFO] 正在下載 LocalizeLimbusCompany 最新 Release（強制）..."
        );
        let zip_url = zip_url_from_release(&release)?;
        clog!(self, "[INFO] ZIP 下載網址: {zip_url}");
        let zip_data = self.download_zip(&zip_url)?;
        clog!(self, "[OK] ZIP 下載完成");

        clog!(self, "[INFO] 正在解壓...");
        let files = self.extract_files(zip_data)?;
        clog!(self, "[OK] 解壓完成，共 {} 個 JSON", files.len());

        clog!(self, "[INFO] 正在簡→正體轉換並強制覆寫輸出目錄...");
        self.process_files(files)?;
        clog!(self, "[OK] 轉換完成（已強制覆寫）");

        self.check_cancelled()?;
        self.colorize_speech_bubbles()?;

        self.check_cancelled()?;
        self.colorize_personality_voices()?;

        self.check_cancelled()?;
        self.write_font_folder()?;

        clog!(self, "[INFO] 更新版本資訊...");
        config::update_version_info(&latest_release_tag, latest_release_date.as_deref())?;
        clog!(self, "[OK] 版本資訊已更新: {latest_release_tag}");

        self.report_progress(
            96.0,
            "正在建立設定檔...".to_string(),
            None,
            None,
            None,
        );
        self.ensure_game_config()?;
        config::ensure_translation_config()?;

        self.report_progress(
            98.0,
            "正在清理暫存檔...".to_string(),
            None,
            None,
            None,
        );
        if self.temp_dir.exists() {
            fs::remove_dir_all(&self.temp_dir)?;
        }

        clog!(self, "[OK] 轉換完成！");
        clog!(self, "[INFO] ===== 轉換流程結束 =====");
        self.report_progress(100.0, "轉換完成！".to_string(), None, None, None);
        Ok(())
    }

    fn ensure_game_config(&self) -> Result<(), AppError> {
        self.check_cancelled()?;
        let full_output_path = Path::new(&self.config.output_base);

        let Some(lang_dir) = full_output_path.parent() else {
            return Err(AppError::Other(format!(
                "無法解析 Lang 資料夾路徑: {}",
                self.config.output_base
            )));
        };

        let config_file_path = lang_dir.join("config.json");
        if config_file_path.exists() {
            clog!(self, "[INFO] 保留現有 config.json，不覆寫使用者設定");
            return Ok(());
        }

        let config = TranslationConfig::default();
        let content = serde_json::to_string_pretty(&config)?;
        fs::write(&config_file_path, content).map_err(|e| {
            AppError::Other(format!(
                "無法建立設定檔 {}: {}",
                config_file_path.display(),
                e
            ))
        })?;
        Ok(())
    }
}

pub fn zip_url_from_release(release: &GitHubRelease) -> Result<String, AppError> {
    let zip_asset = release
        .assets
        .iter()
        .find(|asset| asset.name.ends_with(".zip") && !asset.name.contains("Source"))
        .or_else(|| {
            release
                .assets
                .iter()
                .find(|asset| asset.name.ends_with(".zip"))
        })
        .ok_or_else(|| AppError::Other("找不到 ZIP 檔案".to_string()))?;

    Ok(zip_asset.browser_download_url.clone())
}