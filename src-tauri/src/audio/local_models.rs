use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use bzip2::read::BzDecoder;
use futures::StreamExt;
use serde::Serialize;
use specta::Type;
use tauri::{path::BaseDirectory, AppHandle, Emitter, Manager, Wry};
use tokio::fs as async_fs;
use tokio::io::AsyncWriteExt;
use tracing::{info, warn};

pub const PARAFORMER_MODEL_ID: &str = "sherpa-onnx-paraformer-zh-small-2024-03-09";
pub const SENSEVOICE_MODEL_ID: &str = "sherpa-onnx-sense-voice-zh-en-ja-ko-yue-int8-2025-09-09";

pub const DEFAULT_MODEL_ID: &str = PARAFORMER_MODEL_ID;

struct LocalModelSpec {
    id: &'static str,
    title: &'static str,
    archive_url: &'static str,
    required_files: &'static [(&'static str, &'static str)],
}

const LOCAL_MODEL_SPECS: &[LocalModelSpec] = &[
    LocalModelSpec {
        id: PARAFORMER_MODEL_ID,
        title: "Paraformer 小尺寸离线识别",
        archive_url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-paraformer-zh-small-2024-03-09.tar.bz2",
        required_files: &[
            ("model.int8.onnx", "ASR 模型文件"),
            ("tokens.txt", "词表文件"),
        ],
    },
    LocalModelSpec {
        id: SENSEVOICE_MODEL_ID,
        title: "SenseVoice 多语种离线识别",
        archive_url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-sense-voice-zh-en-ja-ko-yue-int8-2025-09-09.tar.bz2",
        required_files: &[
            ("model.int8.onnx", "ASR 模型文件"),
            ("tokens.txt", "词表文件"),
        ],
    },
];

fn get_spec(model_id: &str) -> Option<&'static LocalModelSpec> {
    LOCAL_MODEL_SPECS.iter().find(|spec| spec.id == model_id)
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct OfflineModelsStatus {
    pub ready: bool,
    pub missing_files: Vec<String>,
    pub install_dir: String,
    pub models: Vec<OfflineAsrModelStatus>,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct OfflineAsrModelStatus {
    pub id: String,
    pub title: String,
    pub ready: bool,
    pub missing_files: Vec<String>,
    pub install_dir: String,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct OfflineModelDownloadProgress {
    pub model_id: String,
    pub received_bytes: u64,
    pub total_bytes: Option<u64>,
}

#[tauri::command]
#[specta::specta]
pub fn get_offline_models_status(app: AppHandle) -> Result<OfflineModelsStatus, String> {
    status(&app).map_err(|err| err.to_string())
}

pub fn ensure_model_ready(app: &AppHandle<Wry>, model_id: &str) -> Result<(), String> {
    let spec = get_spec(model_id).ok_or_else(|| format!("未知离线模型: {model_id}"))?;
    let info = status(app)
        .map_err(|err| err.to_string())?
        .models
        .into_iter()
        .find(|model| model.id == spec.id)
        .ok_or_else(|| format!("未知离线模型: {}", spec.id))?;
    if info.ready {
        Ok(())
    } else {
        Err(format!(
            "{} 未就绪，请先下载所需文件：{}",
            spec.title,
            info.missing_files.join("，")
        ))
    }
}

#[tauri::command(async)]
#[specta::specta]
pub async fn download_offline_models(
    app: AppHandle,
    model_id: String,
) -> Result<OfflineModelsStatus, String> {
    let spec = get_spec(&model_id).ok_or_else(|| format!("不支持的离线模型: {model_id}"))?;

    let models_dir = models_root(&app).map_err(|err| err.to_string())?;
    if let Err(error) = download_and_extract(&app, spec, &models_dir).await {
        warn!(
            target = "miaoyu_audio",
            error = %error,
            "下载离线模型失败"
        );
        return Err(error.to_string());
    }

    status(&app).map_err(|err| err.to_string())
}

pub fn resolve_model_file(app: &AppHandle<Wry>, model_id: &str, relative: &str) -> Result<PathBuf> {
    let spec = get_spec(model_id).ok_or_else(|| anyhow!("未知离线模型: {model_id}"))?;
    let root = models_root(app)?;
    let target = root.join(spec.id).join(relative);
    if target.exists() {
        Ok(target)
    } else {
        Err(anyhow!(
            "缺少离线模型文件 {}，请先下载 {}",
            target.display(),
            spec.title
        ))
    }
}

fn models_root(app: &AppHandle<Wry>) -> Result<PathBuf> {
    let dir = app
        .path()
        .resolve("models", BaseDirectory::AppData)
        .map_err(|err| anyhow!("无法定位模型目录: {err}"))?;
    fs::create_dir_all(&dir).with_context(|| format!("无法创建模型目录: {}", dir.display()))?;
    Ok(dir)
}

fn status(app: &AppHandle<Wry>) -> Result<OfflineModelsStatus> {
    let root = models_root(app)?;
    let mut models = Vec::new();
    let mut missing_all = Vec::new();
    for spec in LOCAL_MODEL_SPECS {
        let model_status = status_for_spec(&root, spec);
        if !model_status.ready {
            missing_all.extend(model_status.missing_files.iter().cloned());
        }
        models.push(model_status);
    }

    Ok(OfflineModelsStatus {
        ready: missing_all.is_empty(),
        missing_files: missing_all,
        install_dir: root.display().to_string(),
        models,
    })
}

fn status_for_spec(root: &Path, spec: &LocalModelSpec) -> OfflineAsrModelStatus {
    let model_dir = root.join(spec.id);
    let mut missing_files = Vec::new();
    for (file, description) in spec.required_files {
        let path = model_dir.join(file);
        if !path.exists() {
            missing_files.push(format!("{}（{}/{}）", description, spec.id, file));
        }
    }

    OfflineAsrModelStatus {
        id: spec.id.to_string(),
        title: spec.title.to_string(),
        ready: missing_files.is_empty(),
        missing_files,
        install_dir: model_dir.display().to_string(),
    }
}

async fn download_and_extract(
    app: &AppHandle,
    spec: &LocalModelSpec,
    models_dir: &Path,
) -> Result<()> {
    info!(
        target = "miaoyu_audio",
        model = spec.id,
        "开始下载 {} 离线模型",
        spec.title
    );
    let temp_dir = std::env::temp_dir().join(format!("miaoyu-model-{}", uuid::Uuid::new_v4()));
    async_fs::create_dir_all(&temp_dir).await?;
    let archive_path = temp_dir.join(format!("{}.tar.bz2", spec.id));
    download_file(app, spec.id, spec.archive_url, &archive_path).await?;
    extract_tar_bz2(&archive_path, &temp_dir).await?;
    copy_model_contents(spec, &temp_dir, models_dir).await?;
    async_fs::remove_dir_all(&temp_dir).await.ok();
    Ok(())
}

async fn download_file(
    app: &AppHandle,
    model_id: &str,
    url: &str,
    destination: &Path,
) -> Result<()> {
    if let Some(parent) = destination.parent() {
        async_fs::create_dir_all(parent).await?;
    }
    let response = reqwest::Client::new()
        .get(url)
        .send()
        .await?
        .error_for_status()?;
    let mut file = async_fs::File::create(destination).await?;
    let total = response.content_length();
    let mut stream = response.bytes_stream();
    let mut downloaded = 0u64;
    emit_download_progress(app, model_id, downloaded, total);
    while let Some(chunk) = stream.next().await {
        let data = chunk?;
        file.write_all(&data).await?;
        downloaded = downloaded.saturating_add(data.len() as u64);
        emit_download_progress(app, model_id, downloaded, total);
    }
    file.flush().await?;
    emit_download_progress(app, model_id, downloaded, total);
    Ok(())
}

async fn extract_tar_bz2(archive_path: &Path, destination: &Path) -> Result<()> {
    let archive = archive_path.to_owned();
    let dest = destination.to_owned();
    tokio::task::spawn_blocking(move || {
        let file = fs::File::open(&archive)?;
        let decoder = BzDecoder::new(file);
        let mut archive = tar::Archive::new(decoder);
        archive.unpack(&dest)?;
        Result::<_, anyhow::Error>::Ok(())
    })
    .await??;
    Ok(())
}

async fn copy_model_contents(
    spec: &LocalModelSpec,
    temp_root: &Path,
    models_root: &Path,
) -> Result<()> {
    let source =
        find_model_dir(temp_root, spec).ok_or_else(|| anyhow!("归档中缺少 {} 目录", spec.id))?;
    let destination = models_root.join(spec.id);
    tokio::task::spawn_blocking(move || {
        if destination.exists() {
            fs::remove_dir_all(&destination)?;
        }
        fs::create_dir_all(&destination)?;
        for entry in fs::read_dir(&source)? {
            let entry = entry?;
            let target = destination.join(entry.file_name());
            if entry.file_type()?.is_dir() {
                copy_dir_recursive(entry.path(), target)?;
            } else {
                fs::copy(entry.path(), target)?;
            }
        }
        Result::<_, anyhow::Error>::Ok(())
    })
    .await??;
    Ok(())
}

fn find_model_dir(root: &Path, spec: &LocalModelSpec) -> Option<PathBuf> {
    let mut stack = vec![root.to_path_buf()];
    while let Some(path) = stack.pop() {
        if path.file_name().and_then(|n| n.to_str()) == Some(spec.id) {
            return Some(path);
        }
        if let Ok(entries) = fs::read_dir(&path) {
            for entry in entries.flatten() {
                if let Ok(kind) = entry.file_type() {
                    if kind.is_dir() {
                        stack.push(entry.path());
                    }
                }
            }
        }
    }
    None
}

fn copy_dir_recursive(src: PathBuf, dest: PathBuf) -> Result<()> {
    if dest.exists() {
        fs::remove_dir_all(&dest)?;
    }
    fs::create_dir_all(&dest)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let target = dest.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_recursive(entry.path(), target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

fn emit_download_progress(app: &AppHandle, model_id: &str, received: u64, total: Option<u64>) {
    let payload = OfflineModelDownloadProgress {
        model_id: model_id.to_string(),
        received_bytes: received,
        total_bytes: total,
    };
    app.emit("offline-model-download-progress", payload).ok();
}
