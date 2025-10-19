use std::{
    io::Cursor,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{anyhow, bail, Context, Result};
use once_cell::sync::OnceCell;
use serde::Serialize;
use specta::Type;
use tauri::{path::BaseDirectory, AppHandle, Manager, Wry};
use tokio::sync::Mutex;
use tracing::{debug, warn};

use sherpa_rs::{
    paraformer::{ParaformerConfig, ParaformerRecognizer},
    punctuate::{Punctuation, PunctuationConfig},
    silero_vad::{SileroVad, SileroVadConfig},
};

const TARGET_SAMPLE_RATE: u32 = 16_000;
const VAD_WINDOW_SIZE: usize = 512;
const VAD_BUFFER_DURATION: f32 = 60.0 * 10.0;
const VAD_PADDING_SECONDS: u32 = 3;

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptionResult {
    pub text: String,
    pub duration_ms: Option<u32>,
    pub utterances: Vec<TranscriptionUtterance>,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptionUtterance {
    pub text: String,
    pub start_time: u32,
    pub end_time: u32,
}

#[derive(Debug)]
struct PreparedAudio {
    waveform: Vec<f32>,
    duration_ms: u32,
}

#[derive(Debug, Clone)]
struct Segment {
    start_sample: usize,
    samples: Vec<f32>,
}

#[derive(Debug, Clone)]
struct RecognizedSegment {
    text: String,
    start_ms: u32,
    end_ms: u32,
}

struct LocalSherpa {
    recognizer: Mutex<ParaformerRecognizer>,
    punctuator: Mutex<Punctuation>,
    vad_model_path: String,
}

static LOCAL_SHERPA: OnceCell<Arc<LocalSherpa>> = OnceCell::new();

pub struct AudioTranscribing;

impl AudioTranscribing {
    pub async fn transcribe(
        app: &AppHandle<Wry>,
        wav_data: Vec<u8>,
    ) -> Result<TranscriptionResult> {
        let service = LocalSherpa::instance(app)?;
        service.transcribe(wav_data).await
    }
}

impl LocalSherpa {
    fn instance(app: &AppHandle<Wry>) -> Result<Arc<Self>> {
        LOCAL_SHERPA
            .get_or_try_init(|| {
                let svc = Self::create(app)?;
                Ok(Arc::new(svc))
            })
            .map(Arc::clone)
    }

    async fn transcribe(&self, wav_data: Vec<u8>) -> Result<TranscriptionResult> {
        let prepared = self.prepare_audio(&wav_data)?;
        if prepared.waveform.is_empty() {
            bail!("录音数据为空");
        }

        let segments = self.run_vad(&prepared.waveform)?;
        debug!(
            target = "miaoyu_audio",
            segment_count = segments.len(),
            "VAD 检测完成"
        );

        let recognized = self
            .recognize_segments(segments, &prepared.waveform)
            .await?;

        let utterances: Vec<TranscriptionUtterance> = recognized
            .iter()
            .map(|segment| TranscriptionUtterance {
                text: segment.text.clone(),
                start_time: segment.start_ms,
                end_time: segment.end_ms,
            })
            .collect();

        let text = recognized
            .iter()
            .map(|segment| segment.text.clone())
            .filter(|t| !t.is_empty())
            .collect::<Vec<_>>()
            .join(" ");

        if utterances.is_empty() {
            bail!("未检测到语音，请检查麦克风是否正常并在录音时保持发声");
        }

        Ok(TranscriptionResult {
            text,
            duration_ms: Some(prepared.duration_ms),
            utterances,
        })
    }

    fn create(app: &AppHandle<Wry>) -> Result<Self> {
        let asr_model = resolve_model_path(app, "models/asr/model.int8.onnx")
            .context("未找到 ASR 模型文件 models/asr/model.int8.onnx")?;
        let tokens = resolve_model_path(app, "models/asr/tokens.txt")
            .context("未找到 ASR 词表文件 models/asr/tokens.txt")?;
        let vad_model = resolve_model_path(app, "models/vad/silero_vad.onnx")
            .or_else(|_| resolve_model_path(app, "models/vad/model.onnx"))
            .context("未找到 VAD 模型文件（期望 models/vad/silero_vad.onnx 或 model.onnx）")?;
        let punc_model = resolve_model_path(app, "models/punc/model.onnx")
            .context("未找到标点模型文件 models/punc/model.onnx")?;

        debug!(
            target = "miaoyu_audio",
            asr_model = %asr_model.display(),
            tokens = %tokens.display(),
            vad_model = %vad_model.display(),
            punc_model = %punc_model.display(),
            "加载本地语音模型"
        );

        let recognizer_config = ParaformerConfig {
            model: asr_model.to_string_lossy().to_string(),
            tokens: tokens.to_string_lossy().to_string(),
            provider: Some(sherpa_rs::get_default_provider()),
            num_threads: Some(2),
            ..Default::default()
        };
        let recognizer = ParaformerRecognizer::new(recognizer_config)
            .map_err(|err| anyhow!("创建 ASR 识别器失败: {err}"))?;

        let punctuator_config = PunctuationConfig {
            model: punc_model.to_string_lossy().to_string(),
            num_threads: Some(2),
            ..Default::default()
        };
        let punctuator = Punctuation::new(punctuator_config)
            .map_err(|err| anyhow!("加载标点模型失败: {err}"))?;

        Ok(Self {
            recognizer: Mutex::new(recognizer),
            punctuator: Mutex::new(punctuator),
            vad_model_path: vad_model.to_string_lossy().to_string(),
        })
    }

    fn prepare_audio(&self, wav_data: &[u8]) -> Result<PreparedAudio> {
        let cursor = Cursor::new(wav_data);
        let mut reader =
            hound::WavReader::new(cursor).context("解析录音 WAV 数据失败，请稍后重试")?;
        let spec = reader.spec();

        if spec.channels == 0 {
            bail!("录音通道数无效");
        }

        let sample_rate = spec.sample_rate;
        if sample_rate == 0 {
            bail!("录音采样率无效");
        }

        let channels = spec.channels as usize;
        let mut samples = Vec::new();
        for sample in reader.samples::<i16>() {
            samples.push(sample.context("读取音频样本失败")? as f32 / i16::MAX as f32);
        }

        let mono: Vec<f32> = if channels == 1 {
            samples
        } else {
            samples
                .chunks(channels)
                .map(|frame| frame.iter().copied().sum::<f32>() / channels as f32)
                .collect()
        };

        let duration_ms = ((mono.len() as f64) / (sample_rate as f64) * 1000.0)
            .round()
            .clamp(0.0, u32::MAX as f64) as u32;

        let waveform = if sample_rate == TARGET_SAMPLE_RATE {
            mono
        } else {
            resample_linear(&mono, sample_rate, TARGET_SAMPLE_RATE)
        };

        Ok(PreparedAudio {
            waveform,
            duration_ms,
        })
    }

    fn run_vad(&self, waveform: &[f32]) -> Result<Vec<Segment>> {
        let mut vad = self.create_vad()?;

        let mut padded = waveform.to_vec();
        padded.extend(
            std::iter::repeat(0.0)
                .take((TARGET_SAMPLE_RATE as usize) * (VAD_PADDING_SECONDS as usize)),
        );

        let mut segments = Vec::new();
        let mut index = 0;
        while index + VAD_WINDOW_SIZE <= padded.len() {
            vad.accept_waveform(padded[index..index + VAD_WINDOW_SIZE].to_vec());
            if vad.is_speech() {
                while !vad.is_empty() {
                    let speech = vad.front();
                    segments.push(Segment {
                        start_sample: speech.start.max(0) as usize,
                        samples: speech.samples,
                    });
                    vad.pop();
                }
            }
            index += VAD_WINDOW_SIZE;
        }

        vad.flush();
        while !vad.is_empty() {
            let speech = vad.front();
            segments.push(Segment {
                start_sample: speech.start.max(0) as usize,
                samples: speech.samples,
            });
            vad.pop();
        }

        if segments.is_empty() && !waveform.is_empty() {
            warn!(
                target = "miaoyu_audio",
                "VAD 未检测到语音，回退到整段音频识别"
            );
            segments.push(Segment {
                start_sample: 0,
                samples: waveform.to_vec(),
            });
        }

        Ok(segments)
    }

    fn create_vad(&self) -> Result<SileroVad> {
        let mut config = SileroVadConfig {
            model: self.vad_model_path.clone(),
            sample_rate: TARGET_SAMPLE_RATE,
            window_size: VAD_WINDOW_SIZE as i32,
            ..Default::default()
        };
        config.min_silence_duration = 0.3;
        config.min_speech_duration = 0.2;
        config.max_speech_duration = 120.0;
        config.threshold = 0.5;
        config.provider = Some(sherpa_rs::get_default_provider());

        SileroVad::new(config, VAD_BUFFER_DURATION)
            .map_err(|err| anyhow!("初始化语音活动检测器失败: {err}"))
    }

    async fn recognize_segments(
        &self,
        segments: Vec<Segment>,
        fallback_waveform: &[f32],
    ) -> Result<Vec<RecognizedSegment>> {
        let mut recognizer = self.recognizer.lock().await;
        let mut punctuator = self.punctuator.lock().await;

        let mut recognized = Vec::new();
        for segment in &segments {
            if segment.samples.is_empty() {
                continue;
            }

            let samples = segment.samples.clone();
            let recognition =
                tokio::task::block_in_place(|| recognizer.transcribe(TARGET_SAMPLE_RATE, &samples));
            let raw_text = recognition.text.trim().to_string();
            if raw_text.is_empty() {
                continue;
            }

            let punctuated = tokio::task::block_in_place(|| punctuator.add_punctuation(&raw_text));
            let text = punctuated.trim().to_string();
            if text.is_empty() {
                continue;
            }

            recognized.push(RecognizedSegment {
                text,
                start_ms: samples_to_ms(segment.start_sample),
                end_ms: samples_to_ms(segment.start_sample + segment.samples.len()),
            });
        }

        if recognized.is_empty() && !fallback_waveform.is_empty() {
            let all_samples = fallback_waveform.to_vec();
            let recognition = tokio::task::block_in_place(|| {
                recognizer.transcribe(TARGET_SAMPLE_RATE, &all_samples)
            });
            let raw_text = recognition.text.trim().to_string();
            if !raw_text.is_empty() {
                let punctuated =
                    tokio::task::block_in_place(|| punctuator.add_punctuation(&raw_text));
                let text = punctuated.trim().to_string();
                if !text.is_empty() {
                    recognized.push(RecognizedSegment {
                        text,
                        start_ms: 0,
                        end_ms: samples_to_ms(all_samples.len()),
                    });
                }
            }
        }

        if recognized.is_empty() {
            warn!(
                target = "miaoyu_audio",
                "ASR 未能识别到有效文本，可能的原因是麦克风输入过低或环境噪声过大"
            );
        }

        Ok(recognized)
    }
}

fn resample_linear(samples: &[f32], src_rate: u32, dst_rate: u32) -> Vec<f32> {
    if samples.is_empty() || src_rate == dst_rate {
        return samples.to_vec();
    }

    let ratio = dst_rate as f64 / src_rate as f64;
    let output_len = ((samples.len() as f64) * ratio).ceil() as usize;
    if output_len == 0 {
        return Vec::new();
    }

    if samples.len() == 1 {
        return vec![samples[0]; output_len];
    }

    let mut output = Vec::with_capacity(output_len);
    for index in 0..output_len {
        let src_pos = index as f64 / ratio;
        let base_index = src_pos.floor() as usize;
        let frac = (src_pos - base_index as f64) as f32;

        let current = samples
            .get(base_index)
            .copied()
            .unwrap_or_else(|| *samples.last().unwrap());
        let next = samples.get(base_index + 1).copied().unwrap_or(current);

        output.push(current + (next - current) * frac);
    }

    output
}

fn samples_to_ms(samples: usize) -> u32 {
    ((samples as f64) / (TARGET_SAMPLE_RATE as f64) * 1000.0)
        .round()
        .clamp(0.0, u32::MAX as f64) as u32
}

fn resolve_model_path(app: &AppHandle<Wry>, relative: &str) -> Result<PathBuf> {
    let path = Path::new(relative);

    if let Ok(resolved) = app.path().resolve(path, BaseDirectory::Resource) {
        if resolved.exists() {
            return Ok(resolved);
        }
    }

    let current_dir = std::env::current_dir().context("无法获取当前工作目录")?;
    let fallback = current_dir.join("src-tauri").join(path);
    if fallback.exists() {
        return Ok(fallback);
    }

    let direct = current_dir.join(path);
    if direct.exists() {
        return Ok(direct);
    }

    bail!("模型文件不存在: {}", relative);
}
