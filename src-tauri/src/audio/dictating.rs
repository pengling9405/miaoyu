use std::io::Cursor;
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use tracing::{error, info};
const START_SOUND_BYTES: &[u8] =
    include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/sounds/start.mp3"));
const END_SOUND_BYTES: &[u8] =
    include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/sounds/end.mp3"));
const NOTIFICATION_SOUND_BYTES: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/sounds/notification.mp3"
));

use rodio::{Decoder, OutputStream, Sink};

pub struct DictatingStream {
    stream: cpal::Stream,
    sample_rate: u32,
    buffer: Arc<Mutex<Vec<f32>>>,
}

unsafe impl Send for DictatingStream {}
unsafe impl Sync for DictatingStream {}

impl DictatingStream {
    pub fn new() -> Result<Self, String> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .or_else(|| {
                host.input_devices()
                    .ok()
                    .and_then(|mut devices| devices.next())
            })
            .ok_or_else(|| "未找到可用的音频输入设备".to_string())?;
        let config = device
            .default_input_config()
            .map_err(|e| format!("获取麦克风配置失败: {e}"))?;
        let sample_rate = config.sample_rate().0;
        let channels = config.channels();
        let stream_config: cpal::StreamConfig = config.clone().into();
        let buffer: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
        let buffer_clone = buffer.clone();
        let err_fn = |err| error!(target = "miaoyu_audio", error = %err, "音频输入流错误");

        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => device
                .build_input_stream(
                    &stream_config,
                    move |data: &[f32], _| write_buffer(&buffer_clone, data, channels),
                    err_fn,
                    None,
                )
                .map_err(|e| format!("启动音频输入失败: {e}"))?,
            cpal::SampleFormat::I16 => device
                .build_input_stream(
                    &stream_config,
                    move |data: &[i16], _| {
                        let converted: Vec<f32> =
                            data.iter().map(|s| *s as f32 / i16::MAX as f32).collect();
                        write_buffer(&buffer_clone, &converted, channels)
                    },
                    err_fn,
                    None,
                )
                .map_err(|e| format!("启动音频输入失败: {e}"))?,
            cpal::SampleFormat::U16 => device
                .build_input_stream(
                    &stream_config,
                    move |data: &[u16], _| {
                        let converted: Vec<f32> = data
                            .iter()
                            .map(|s| (*s as f32 / u16::MAX as f32) * 2.0 - 1.0)
                            .collect();
                        write_buffer(&buffer_clone, &converted, channels)
                    },
                    err_fn,
                    None,
                )
                .map_err(|e| format!("启动音频输入失败: {e}"))?,
            _ => {
                return Err("当前平台的采样格式暂不受支持".to_string());
            }
        };

        stream
            .play()
            .map_err(|e| format!("播放音频输入流失败: {e}"))?;

        Ok(Self {
            stream,
            sample_rate,
            buffer,
        })
    }

    pub fn into_samples(self) -> (Vec<f32>, u32) {
        drop(self.stream);
        let samples = self
            .buffer
            .lock()
            .map(|buf| buf.clone())
            .unwrap_or_default();
        (samples, self.sample_rate)
    }
}

fn write_buffer(buffer: &Arc<Mutex<Vec<f32>>>, data: &[f32], channels: u16) {
    if channels == 0 {
        return;
    }
    if let Ok(mut guard) = buffer.lock() {
        if channels == 1 {
            guard.extend_from_slice(data);
        } else {
            // 仅取第一个声道，避免双声道造成体积翻倍
            guard.extend(data.iter().step_by(channels as usize).copied());
        }
    }
}

pub struct AudioDictating;

impl AudioDictating {
    fn play_sound(bytes: &'static [u8], label: &str) -> Result<(), String> {
        info!(target = "miaoyu_audio", "播放 {label} 音效");
        let cursor = Cursor::new(bytes);
        let decoder = Decoder::new(cursor).map_err(|e| format!("解码音效失败: {e}"))?;
        let (_stream, handle) =
            OutputStream::try_default().map_err(|e| format!("初始化音频输出失败: {e}"))?;
        let sink = Sink::try_new(&handle).map_err(|e| format!("创建音频输出失败: {e}"))?;
        sink.append(decoder);
        sink.sleep_until_end();
        Ok(())
    }

    pub fn play_notification_sound() -> Result<(), String> {
        Self::play_sound(NOTIFICATION_SOUND_BYTES, "通知")
    }

    pub fn play_start_sound() -> Result<(), String> {
        Self::play_sound(START_SOUND_BYTES, "开始录音")
    }

    pub fn play_stop_sound() -> Result<(), String> {
        Self::play_sound(END_SOUND_BYTES, "结束录音")
    }
}
