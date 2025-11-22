use std::sync::Arc;

use anyhow::{anyhow, Result};
use once_cell::sync::OnceCell;
use sherpa_rs::paraformer::{ParaformerConfig, ParaformerRecognizer};
use sherpa_rs::sense_voice::{SenseVoiceConfig, SenseVoiceRecognizer};
use specta::Type;
use tauri::{AppHandle, Wry};
use tokio::sync::Mutex;
use tracing::debug;

use super::local_models;
use crate::history::LlmPolishStatus;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptionResult {
    pub text: String,
    pub duration_ms: Option<u32>,
    pub utterances: Vec<TranscriptionUtterance>,
    #[serde(default)]
    pub llm_polish_status: LlmPolishStatus,
    #[serde(default)]
    pub llm_polish_error: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptionUtterance {
    pub text: String,
    pub start_time: u32,
    pub end_time: u32,
}

const TARGET_SAMPLE_RATE: u32 = 16_000;

enum RecognizerKind {
    Paraformer,
    SenseVoice,
}

impl RecognizerKind {
    fn from_model_id(model_id: &str) -> Self {
        if model_id == local_models::SENSEVOICE_MODEL_ID {
            Self::SenseVoice
        } else {
            Self::Paraformer
        }
    }
}

struct ParaformerService {
    recognizer: Mutex<ParaformerRecognizer>,
}

struct SenseVoiceService {
    recognizer: Mutex<SenseVoiceRecognizer>,
}

static PARAFORMER_SERVICE: OnceCell<Arc<ParaformerService>> = OnceCell::new();
static SENSE_VOICE_SERVICE: OnceCell<Arc<SenseVoiceService>> = OnceCell::new();

impl ParaformerService {
    fn instance(app: &AppHandle<Wry>) -> Result<Arc<Self>> {
        PARAFORMER_SERVICE
            .get_or_try_init(|| {
                let svc = Self::create(app)?;
                Ok(Arc::new(svc))
            })
            .map(Arc::clone)
    }

    fn create(app: &AppHandle<Wry>) -> Result<Self> {
        let model_path = local_models::resolve_model_file(
            app,
            local_models::PARAFORMER_MODEL_ID,
            "model.int8.onnx",
        )?;
        let tokens_path =
            local_models::resolve_model_file(app, local_models::PARAFORMER_MODEL_ID, "tokens.txt")?;

        debug!(
            target = "miaoyu_audio",
            model = %model_path.display(),
            tokens = %tokens_path.display(),
            "加载 Paraformer Small 离线模型"
        );

        let config = ParaformerConfig {
            model: model_path.to_string_lossy().to_string(),
            tokens: tokens_path.to_string_lossy().to_string(),
            provider: Some(sherpa_rs::get_default_provider()),
            num_threads: Some(2),
            ..Default::default()
        };

        let recognizer = ParaformerRecognizer::new(config)
            .map_err(|err| anyhow!("初始化 Paraformer 失败: {err}"))?;

        Ok(Self {
            recognizer: Mutex::new(recognizer),
        })
    }

    async fn transcribe(&self, waveform: Vec<f32>) -> Result<String> {
        let mut recognizer = self.recognizer.lock().await;
        let text = tokio::task::block_in_place(|| {
            let result = recognizer.transcribe(TARGET_SAMPLE_RATE, &waveform);
            result.text
        });
        Ok(text)
    }
}

impl SenseVoiceService {
    fn instance(app: &AppHandle<Wry>) -> Result<Arc<Self>> {
        SENSE_VOICE_SERVICE
            .get_or_try_init(|| {
                let svc = Self::create(app)?;
                Ok(Arc::new(svc))
            })
            .map(Arc::clone)
    }

    fn create(app: &AppHandle<Wry>) -> Result<Self> {
        let model_path = local_models::resolve_model_file(
            app,
            local_models::SENSEVOICE_MODEL_ID,
            "model.int8.onnx",
        )?;
        let tokens_path =
            local_models::resolve_model_file(app, local_models::SENSEVOICE_MODEL_ID, "tokens.txt")?;

        debug!(
            target = "miaoyu_audio",
            model = %model_path.display(),
            tokens = %tokens_path.display(),
            "加载 SenseVoice 离线模型"
        );

        let config = SenseVoiceConfig {
            model: model_path.to_string_lossy().to_string(),
            tokens: tokens_path.to_string_lossy().to_string(),
            provider: Some(sherpa_rs::get_default_provider()),
            num_threads: Some(2),
            ..Default::default()
        };

        let recognizer = SenseVoiceRecognizer::new(config)
            .map_err(|err| anyhow!("初始化 SenseVoice 失败: {err}"))?;

        Ok(Self {
            recognizer: Mutex::new(recognizer),
        })
    }

    async fn transcribe(&self, waveform: Vec<f32>) -> Result<String> {
        let mut recognizer = self.recognizer.lock().await;
        let text = tokio::task::block_in_place(|| {
            let result = recognizer.transcribe(TARGET_SAMPLE_RATE, &waveform);
            result.text
        });
        Ok(text)
    }
}

pub struct AudioTranscribing;

impl AudioTranscribing {
    pub async fn transcribe(
        app: &AppHandle<Wry>,
        mut samples: Vec<f32>,
        sample_rate: u32,
        model_id: &str,
    ) -> Result<TranscriptionResult> {
        if samples.is_empty() {
            return Err(anyhow!("录音数据为空"));
        }

        if sample_rate != TARGET_SAMPLE_RATE {
            samples = resample_linear(&samples, sample_rate, TARGET_SAMPLE_RATE);
        }

        if samples.is_empty() {
            return Err(anyhow!("录音数据为空"));
        }

        let text = match RecognizerKind::from_model_id(model_id) {
            RecognizerKind::SenseVoice => {
                let service = SenseVoiceService::instance(app)?;
                service.transcribe(samples.clone()).await?
            }
            RecognizerKind::Paraformer => {
                let service = ParaformerService::instance(app)?;
                service.transcribe(samples.clone()).await?
            }
        }
        .trim()
        .to_string();

        if text.is_empty() {
            return Err(anyhow!("未识别到有效文本，请重新尝试"));
        }

        let duration_ms = samples_to_ms(samples.len());
        Ok(TranscriptionResult {
            text: text.clone(),
            duration_ms: Some(duration_ms),
            utterances: vec![TranscriptionUtterance {
                text,
                start_time: 0,
                end_time: duration_ms,
            }],
            llm_polish_status: LlmPolishStatus::Skipped,
            llm_polish_error: None,
        })
    }
}

fn resample_linear(samples: &[f32], src_rate: u32, dst_rate: u32) -> Vec<f32> {
    if samples.is_empty() || src_rate == dst_rate {
        return samples.to_vec();
    }

    let ratio = dst_rate as f64 / src_rate as f64;
    let output_len = (samples.len() as f64 * ratio).ceil() as usize;
    if output_len == 0 {
        return Vec::new();
    }

    let mut output = Vec::with_capacity(output_len);
    for index in 0..output_len {
        let src_pos = index as f64 / ratio;
        let base = src_pos.floor() as usize;
        let frac = (src_pos - base as f64) as f32;
        let current = samples.get(base).copied().unwrap_or(0.0);
        let next = samples.get(base + 1).copied().unwrap_or(current);
        output.push(current + (next - current) * frac);
    }

    output
}

fn samples_to_ms(samples: usize) -> u32 {
    ((samples as f64 / TARGET_SAMPLE_RATE as f64) * 1000.0)
        .round()
        .clamp(0.0, u32::MAX as f64) as u32
}
