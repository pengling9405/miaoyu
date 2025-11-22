use std::{io::ErrorKind, path::PathBuf};

use base64::engine::general_purpose::STANDARD as Base64;
use base64::Engine;
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{path::BaseDirectory, AppHandle, Manager, Wry};
use tokio::{fs, task::spawn_blocking};
use uuid::Uuid;

use crate::models;

const HISTORY_DB_PATH: &str = "history/history.db";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum HistoryKind {
    Dictation,
    Diary,
}

impl HistoryKind {
    fn as_str(&self) -> &'static str {
        match self {
            HistoryKind::Dictation => "dictation",
            HistoryKind::Diary => "diary",
        }
    }

    fn from_str(value: &str) -> Option<Self> {
        match value {
            "dictation" => Some(HistoryKind::Dictation),
            "diary" => Some(HistoryKind::Diary),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum LlmPolishStatus {
    Success,
    #[default]
    Skipped,
    QuotaExceeded,
    Failed,
}

impl LlmPolishStatus {
    fn as_str(self) -> &'static str {
        match self {
            LlmPolishStatus::Success => "success",
            LlmPolishStatus::Skipped => "skipped",
            LlmPolishStatus::QuotaExceeded => "quota_exceeded",
            LlmPolishStatus::Failed => "failed",
        }
    }

    fn from_str(value: &str) -> Self {
        match value {
            "success" => LlmPolishStatus::Success,
            "quota_exceeded" => LlmPolishStatus::QuotaExceeded,
            "failed" => LlmPolishStatus::Failed,
            _ => LlmPolishStatus::Skipped,
        }
    }
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: String,
    pub title: Option<String>,
    pub text: String,
    pub kind: HistoryKind,
    pub created_at: String,
    pub duration_seconds: u32,
    pub audio_file_path: Option<String>,
    pub llm_model: Option<String>,
    pub llm_variant_id: Option<String>,
    pub asr_model: Option<String>,
    pub asr_variant_id: Option<String>,
    pub total_words: u32,
    pub total_tokens: u32,
    pub llm_total_tokens: Option<u32>,
    pub source_app: Option<String>,
    pub llm_polish_status: LlmPolishStatus,
    pub llm_polish_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct HistoryListFilter {
    #[serde(default)]
    pub kind: Option<HistoryKind>,
    #[serde(default)]
    pub limit: Option<u32>,
    #[serde(default)]
    pub offset: Option<u32>,
}

impl Default for HistoryListFilter {
    fn default() -> Self {
        Self {
            kind: None,
            limit: Some(50),
            offset: Some(0),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct NewHistoryEntry {
    #[serde(default)]
    pub id: Option<String>,
    pub text: String,
    pub kind: HistoryKind,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub duration_seconds: u32,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub audio_file_path: Option<String>,
    #[serde(default)]
    pub llm_model: Option<String>,
    #[serde(default)]
    pub llm_variant_id: Option<String>,
    #[serde(default)]
    pub asr_model: Option<String>,
    #[serde(default)]
    pub asr_variant_id: Option<String>,
    #[serde(default)]
    pub total_words: Option<u32>,
    #[serde(default)]
    pub total_tokens: Option<u32>,
    #[serde(default)]
    pub llm_total_tokens: Option<u32>,
    #[serde(default)]
    pub source_app: Option<String>,
    #[serde(default)]
    pub llm_polish_status: LlmPolishStatus,
    #[serde(default)]
    pub llm_polish_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct HistoryStats {
    pub total_entries: u32,
    pub total_words: u32,
    #[specta(type = u32)]
    pub total_duration_seconds: u64,
    pub total_apps_used: u32,
}

#[derive(Debug, Clone)]
struct HistoryRemovalInfo {
    audio_path: Option<String>,
    llm_variant_id: Option<String>,
    llm_total_tokens: Option<u32>,
    asr_variant_id: Option<String>,
    duration_seconds: u32,
}

fn history_db_path(app: &AppHandle<Wry>) -> Result<PathBuf, String> {
    let resolved = app
        .path()
        .resolve(HISTORY_DB_PATH, BaseDirectory::AppData)
        .map_err(|e| format!("无法定位历史记录数据库: {e}"))?;

    if let Some(parent) = resolved.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("无法创建历史记录目录: {e}"))?;
    }

    Ok(resolved)
}

fn init_db(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS history_entries (
            id TEXT PRIMARY KEY,
            title TEXT,
            text TEXT NOT NULL,
            kind TEXT NOT NULL,
            created_at TEXT NOT NULL,
            duration_seconds INTEGER NOT NULL,
            audio_file_path TEXT,
            llm_model TEXT,
            llm_variant_id TEXT,
            asr_model TEXT,
            asr_variant_id TEXT,
            total_words INTEGER DEFAULT 0,
            total_tokens INTEGER DEFAULT 0,
            llm_total_tokens INTEGER,
            source_app TEXT,
            llm_polish_status TEXT DEFAULT 'skipped',
            llm_polish_error TEXT
        );

        CREATE INDEX IF NOT EXISTS idx_history_kind_created_at
            ON history_entries(kind, created_at DESC);
        "#,
    )
    .map_err(|e| format!("初始化历史记录数据库失败: {e}"))?;

    let _ = conn.execute(
        "ALTER TABLE history_entries ADD COLUMN llm_variant_id TEXT",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE history_entries ADD COLUMN asr_variant_id TEXT",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE history_entries ADD COLUMN llm_total_tokens INTEGER",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE history_entries ADD COLUMN llm_polish_status TEXT DEFAULT 'skipped'",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE history_entries ADD COLUMN llm_polish_error TEXT",
        [],
    );

    Ok(())
}

fn map_history_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<HistoryEntry> {
    let kind_value: String = row.get("kind")?;
    let status_value: Option<String> = row.get("llm_polish_status")?;
    Ok(HistoryEntry {
        id: row.get("id")?,
        title: row.get("title")?,
        text: row.get("text")?,
        kind: HistoryKind::from_str(&kind_value).unwrap_or(HistoryKind::Dictation),
        created_at: row.get("created_at")?,
        duration_seconds: row.get::<_, i64>("duration_seconds")? as u32,
        audio_file_path: row.get("audio_file_path")?,
        llm_model: row.get("llm_model")?,
        llm_variant_id: row.get("llm_variant_id")?,
        asr_model: row.get("asr_model")?,
        asr_variant_id: row.get("asr_variant_id")?,
        total_words: row.get::<_, i64>("total_words")? as u32,
        total_tokens: row.get::<_, i64>("total_tokens")? as u32,
        llm_total_tokens: row
            .get::<_, Option<i64>>("llm_total_tokens")?
            .map(|value| value as u32),
        source_app: row.get("source_app")?,
        llm_polish_status: status_value
            .as_deref()
            .map(LlmPolishStatus::from_str)
            .unwrap_or_default(),
        llm_polish_error: row.get("llm_polish_error")?,
    })
}

fn query_history(
    conn: &Connection,
    filter: &HistoryListFilter,
) -> Result<Vec<HistoryEntry>, String> {
    let limit = filter.limit.unwrap_or(50).min(200) as i64;
    let offset = filter.offset.unwrap_or(0) as i64;

    let mut entries = Vec::new();

    if let Some(kind) = &filter.kind {
        let mut stmt = conn
            .prepare(
                "SELECT * FROM history_entries WHERE kind = ?1 ORDER BY created_at DESC LIMIT ?2 OFFSET ?3",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![kind.as_str(), limit, offset], map_history_row)
            .map_err(|e| e.to_string())?;
        for row in rows {
            entries.push(row.map_err(|e| e.to_string())?);
        }
    } else {
        let mut stmt = conn
            .prepare("SELECT * FROM history_entries ORDER BY created_at DESC LIMIT ?1 OFFSET ?2")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![limit, offset], map_history_row)
            .map_err(|e| e.to_string())?;
        for row in rows {
            entries.push(row.map_err(|e| e.to_string())?);
        }
    }

    Ok(entries)
}

fn insert_history_entry(
    conn: &Connection,
    entry: &NewHistoryEntry,
) -> Result<HistoryEntry, String> {
    let id = entry
        .id
        .clone()
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let created_at = entry
        .created_at
        .clone()
        .unwrap_or_else(|| Utc::now().to_rfc3339());
    let total_words = entry.total_words.unwrap_or(0) as i64;
    let total_tokens = entry.total_tokens.unwrap_or(0) as i64;
    conn.execute(
        "INSERT OR REPLACE INTO history_entries (id, title, text, kind, created_at, duration_seconds, audio_file_path, llm_model, llm_variant_id, asr_model, asr_variant_id, total_words, total_tokens, llm_total_tokens, source_app, llm_polish_status, llm_polish_error)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
        params![
            id,
            entry.title.clone(),
            entry.text.clone(),
            entry.kind.as_str(),
            created_at,
            entry.duration_seconds as i64,
            entry.audio_file_path.clone(),
            entry.llm_model.clone(),
            entry.llm_variant_id.clone(),
            entry.asr_model.clone(),
            entry.asr_variant_id.clone(),
            total_words,
            total_tokens,
            entry
                .llm_total_tokens
                .map(|value| value as i64)
                .unwrap_or(0),
            entry.source_app.clone(),
            entry.llm_polish_status.as_str(),
            entry.llm_polish_error.clone(),
        ],
    )
    .map_err(|e| format!("写入历史记录失败: {e}"))?;

    Ok(HistoryEntry {
        id,
        title: entry.title.clone(),
        text: entry.text.clone(),
        kind: entry.kind,
        created_at,
        duration_seconds: entry.duration_seconds,
        audio_file_path: entry.audio_file_path.clone(),
        llm_model: entry.llm_model.clone(),
        llm_variant_id: entry.llm_variant_id.clone(),
        asr_model: entry.asr_model.clone(),
        asr_variant_id: entry.asr_variant_id.clone(),
        total_words: total_words as u32,
        total_tokens: total_tokens as u32,
        llm_total_tokens: entry.llm_total_tokens,
        source_app: entry.source_app.clone(),
        llm_polish_status: entry.llm_polish_status,
        llm_polish_error: entry.llm_polish_error.clone(),
    })
}

fn remove_history_entry(conn: &Connection, id: &str) -> Result<Option<HistoryRemovalInfo>, String> {
    let info = conn
        .query_row(
            "SELECT audio_file_path, llm_variant_id, llm_total_tokens, asr_variant_id, duration_seconds
            FROM history_entries WHERE id = ?1",
            params![id],
            |row| {
                let audio: Option<String> = row.get(0)?;
                let llm_variant: Option<String> = row.get(1)?;
                let llm_tokens: Option<i64> = row.get(2)?;
                let asr_variant: Option<String> = row.get(3)?;
                let duration: i64 = row.get(4)?;
                Ok(HistoryRemovalInfo {
                    audio_path: audio.filter(|value| !value.trim().is_empty()),
                    llm_variant_id: llm_variant,
                    llm_total_tokens: llm_tokens.map(|value| value as u32),
                    asr_variant_id: asr_variant,
                    duration_seconds: duration.max(0) as u32,
                })
            },
        )
        .optional()
        .map_err(|e| format!("查询历史记录失败: {e}"))?;
    conn.execute("DELETE FROM history_entries WHERE id = ?1", params![id])
        .map_err(|e| format!("删除历史记录失败: {e}"))?;
    Ok(info)
}

fn clear_history(conn: &Connection) -> Result<Vec<String>, String> {
    let mut stmt = conn
        .prepare("SELECT audio_file_path FROM history_entries WHERE audio_file_path IS NOT NULL")
        .map_err(|e| format!("查询历史音频失败: {e}"))?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| format!("读取历史音频失败: {e}"))?;
    let mut files = Vec::new();
    for path in rows.flatten() {
        if !path.trim().is_empty() {
            files.push(path);
        }
    }
    conn.execute("DELETE FROM history_entries", [])
        .map_err(|e| format!("清空历史记录失败: {e}"))?;
    Ok(files)
}

fn read_stats(conn: &Connection) -> Result<HistoryStats, String> {
    let mut stmt = conn
        .prepare(
            "SELECT COUNT(*) as total_entries,
                    COALESCE(SUM(total_words), 0) as total_words,
                    COALESCE(SUM(duration_seconds), 0) as total_duration,
                    COUNT(DISTINCT source_app) as total_apps
            FROM history_entries",
        )
        .map_err(|e| e.to_string())?;
    let mut rows = stmt.query([]).map_err(|e| e.to_string())?;
    if let Some(row) = rows.next().map_err(|e| e.to_string())? {
        Ok(HistoryStats {
            total_entries: row
                .get::<_, i64>("total_entries")
                .map_err(|e| e.to_string())? as u32,
            total_words: row
                .get::<_, i64>("total_words")
                .map_err(|e| e.to_string())? as u32,
            total_duration_seconds: row
                .get::<_, i64>("total_duration")
                .map_err(|e| e.to_string())? as u64,
            total_apps_used: row.get::<_, i64>("total_apps").map_err(|e| e.to_string())? as u32,
        })
    } else {
        Ok(HistoryStats {
            total_entries: 0,
            total_words: 0,
            total_duration_seconds: 0,
            total_apps_used: 0,
        })
    }
}

async fn with_connection<T, F>(app: AppHandle, task: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce(Connection) -> Result<T, String> + Send + 'static,
{
    let path = history_db_path(&app)?;
    spawn_blocking(move || {
        let conn = Connection::open(path).map_err(|e| format!("无法打开历史记录数据库: {e}"))?;
        init_db(&conn)?;
        task(conn)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
#[specta::specta]
pub async fn list_history_entries(
    app: AppHandle,
    filter: Option<HistoryListFilter>,
) -> Result<Vec<HistoryEntry>, String> {
    let query = filter.unwrap_or_default();
    with_connection(app, move |conn| query_history(&conn, &query)).await
}

#[tauri::command]
#[specta::specta]
pub async fn add_history_entry(
    app: AppHandle,
    entry: NewHistoryEntry,
) -> Result<HistoryEntry, String> {
    with_connection(app, move |conn| insert_history_entry(&conn, &entry)).await
}

async fn delete_history_audio_file(app: &AppHandle<Wry>, path: &str) {
    let file_path = match resolve_history_audio_path(app, path) {
        Ok(path) => path,
        Err(error) => {
            tracing::warn!(
                target = "miaoyu_history",
                error = %error,
                "删除历史音频时路径校验失败"
            );
            return;
        }
    };

    if let Err(error) = fs::remove_file(&file_path).await {
        if error.kind() != ErrorKind::NotFound {
            tracing::warn!(
                target = "miaoyu_history",
                error = %error,
                file = %file_path.display(),
                "删除历史音频文件失败"
            );
        }
    }
}

#[tauri::command]
#[specta::specta]
pub async fn delete_history_entry(app: AppHandle, id: String) -> Result<(), String> {
    tracing::debug!(target = "miaoyu_history", entry_id = %id, "收到删除历史记录请求");
    let app_for_db = app.clone();
    let entry_id = id.clone();
    let removal = with_connection(app_for_db, move |conn| {
        remove_history_entry(&conn, &entry_id)
    })
    .await?;
    if let Some(info) = removal {
        if let Some(path) = info.audio_path {
            delete_history_audio_file(&app, &path).await;
        }
        if let (Some(variant), Some(tokens)) = (info.llm_variant_id, info.llm_total_tokens) {
            if let Err(error) = models::revert_llm_usage(&app, &variant, tokens) {
                tracing::warn!(
                    target = "miaoyu_models",
                    error = %error,
                    variant = %variant,
                    "回退文本模型统计失败"
                );
            }
        }
        if let Some(variant) = info.asr_variant_id {
            if let Err(error) = models::revert_asr_usage(&app, &variant, info.duration_seconds) {
                tracing::warn!(
                    target = "miaoyu_models",
                    error = %error,
                    variant = %variant,
                    "回退语音模型统计失败"
                );
            }
        }
    }
    tracing::info!(target = "miaoyu_history", entry_id = %id, "历史记录删除完成");
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn clear_history_entries(app: AppHandle) -> Result<(), String> {
    let app_for_db = app.clone();
    let audio_files = with_connection(app_for_db, move |conn| clear_history(&conn)).await?;
    tracing::info!(
        target = "miaoyu_history",
        count = audio_files.len(),
        "开始清理历史记录音频"
    );
    for path in audio_files {
        delete_history_audio_file(&app, &path).await;
    }
    if let Err(error) = models::reset_usage_stats(&app) {
        tracing::warn!(
            target = "miaoyu_models",
            error = %error,
            "重置模型使用统计失败"
        );
    }
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn load_history_audio(app: AppHandle, path: String) -> Result<String, String> {
    let file_path = resolve_history_audio_path(&app, &path)?;
    let data = fs::read(&file_path)
        .await
        .map_err(|e| format!("读取历史音频失败: {e}"))?;
    Ok(Base64.encode(data))
}

pub async fn save_history_audio_clip(
    app: &AppHandle<Wry>,
    samples: &[f32],
    sample_rate: u32,
) -> Result<String, String> {
    let history_root = app
        .path()
        .resolve("history", BaseDirectory::AppData)
        .map_err(|e| format!("无法定位历史记录目录: {e}"))?;
    let audio_root = history_root.join("audio");
    fs::create_dir_all(&audio_root)
        .await
        .map_err(|e| format!("无法创建音频目录: {e}"))?;

    let file_name = format!("{}.wav", Uuid::new_v4());
    let audio_path = audio_root.join(&file_name);
    let samples = samples.to_vec();
    spawn_blocking(move || {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(&audio_path, spec)
            .map_err(|e| format!("创建音频文件失败: {e}"))?;
        for sample in samples {
            let scaled = (sample * i16::MAX as f32).clamp(i16::MIN as f32, i16::MAX as f32) as i16;
            writer
                .write_sample(scaled)
                .map_err(|e| format!("写入音频样本失败: {e}"))?;
        }
        writer
            .finalize()
            .map_err(|e| format!("写入音频文件失败: {e}"))?;
        Ok::<_, String>(())
    })
    .await
    .map_err(|e| format!("写入音频任务失败: {e}"))??;

    Ok(format!("audio/{file_name}"))
}

fn resolve_history_audio_path(app: &AppHandle<Wry>, raw: &str) -> Result<PathBuf, String> {
    let history_root = app
        .path()
        .resolve("history", BaseDirectory::AppData)
        .map_err(|e| format!("无法定位历史记录目录: {e}"))?;
    let audio_root = history_root.join("audio");

    let candidate = PathBuf::from(raw);
    let absolute = if candidate.is_absolute() {
        candidate
    } else {
        history_root.join(candidate)
    };

    if !absolute.starts_with(&audio_root) {
        return Err("无效的历史记录音频路径".to_string());
    }

    Ok(absolute)
}

#[tauri::command]
#[specta::specta]
pub async fn get_history_stats(app: AppHandle) -> Result<HistoryStats, String> {
    with_connection(app, move |conn| read_stats(&conn)).await
}
