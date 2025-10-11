use anyhow::{anyhow, bail, Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Sample, SizedSample};
use rodio::OutputStreamBuilder;
use std::io::Cursor;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

const MIN_RECORDING_DURATION: Duration = Duration::from_millis(500);

#[derive(Clone, Copy)]
pub(super) enum StreamControl {
    Stop,
    Cancel,
}

struct DictatingStreamParams {
    sample_rate: u32,
    channels: u16,
}

pub struct DictatingStream {
    control_tx: Option<mpsc::Sender<StreamControl>>,
    worker: Option<thread::JoinHandle<Result<(), String>>>,
    samples: Arc<Mutex<Vec<i16>>>,
    sample_rate: u32,
    channels: u16,
    started_at: Instant,
}

impl DictatingStream {
    pub fn new() -> Result<Self> {
        let samples = Arc::new(Mutex::new(Vec::<i16>::new()));
        let (control_tx, control_rx) = mpsc::channel();
        let (ready_tx, ready_rx) = mpsc::channel();

        let samples_for_thread = Arc::clone(&samples);

        let worker = thread::spawn(move || -> Result<(), String> {
            let setup = (|| -> Result<(cpal::Stream, DictatingStreamParams), String> {
                let host = cpal::default_host();
                let device = host
                    .default_input_device()
                    .ok_or_else(|| "未找到可用的输入设备".to_string())?;

                let supported_config = match device.default_input_config() {
                    Ok(config) => config,
                    Err(err) => {
                        tracing::warn!(
                            target = "miaoyu_audio",
                            error = %err,
                            "默认输入配置获取失败，尝试使用第一个可用配置"
                        );
                        let mut configs = device
                            .supported_input_configs()
                            .map_err(|err| format!("无法查询可用录音配置: {err}"))?;
                        configs
                            .next()
                            .ok_or_else(|| "未找到可用的录音配置".to_string())?
                            .with_max_sample_rate()
                    }
                };

                let sample_format = supported_config.sample_format();
                let stream_config: cpal::StreamConfig = supported_config.clone().into();

                let stream = match sample_format {
                    cpal::SampleFormat::F32 => build_input_stream::<f32>(
                        &device,
                        &stream_config,
                        Arc::clone(&samples_for_thread),
                        convert_sample_f32,
                    )?,
                    cpal::SampleFormat::I16 => build_input_stream::<i16>(
                        &device,
                        &stream_config,
                        Arc::clone(&samples_for_thread),
                        convert_sample_i16,
                    )?,
                    cpal::SampleFormat::U16 => build_input_stream::<u16>(
                        &device,
                        &stream_config,
                        Arc::clone(&samples_for_thread),
                        convert_sample_u16,
                    )?,
                    sample => return Err(format!("不支持的输入采样格式: {sample:?}")),
                };

                stream
                    .play()
                    .map_err(|err| format!("无法启动录音流: {err}"))?;

                Ok((
                    stream,
                    DictatingStreamParams {
                        sample_rate: stream_config.sample_rate.0,
                        channels: stream_config.channels,
                    },
                ))
            })();

            match setup {
                Ok((stream, ready)) => {
                    if ready_tx.send(Ok(ready)).is_err() {
                        return Err("主线程未能接收录音初始化结果".to_string());
                    }

                    match control_rx.recv() {
                        Ok(StreamControl::Stop) => {
                            tracing::debug!(target = "miaoyu_audio", "录音流收到 Stop 指令");
                        }
                        Ok(StreamControl::Cancel) => {
                            tracing::debug!(target = "miaoyu_audio", "录音流收到 Cancel 指令");
                        }
                        Err(err) => {
                            tracing::warn!(
                                target = "miaoyu_audio",
                                error = %err,
                                "录音控制通道已关闭，录音流提前退出"
                            );
                        }
                    }

                    drop(stream);
                    Ok(())
                }
                Err(err) => {
                    let _ = ready_tx.send(Err(err.clone()));
                    Err(err)
                }
            }
        });

        match ready_rx.recv() {
            Ok(Ok(ready)) => Ok(Self {
                control_tx: Some(control_tx),
                worker: Some(worker),
                samples,
                sample_rate: ready.sample_rate,
                channels: ready.channels,
                started_at: Instant::now(),
            }),
            Ok(Err(message)) => {
                join_worker(worker, "init_failed");
                bail!(message);
            }
            Err(err) => {
                join_worker(worker, "init_failed");
                bail!(format!("录音线程未能启动: {err}"));
            }
        }
    }

    pub fn cancel(mut self) {
        self.send_control(StreamControl::Cancel);
        self.wait_worker();
    }

    pub fn finish(mut self) -> Result<(Vec<u8>, Duration)> {
        self.send_control(StreamControl::Stop);
        self.wait_worker();
        self.into_wav_bytes()
    }

    fn send_control(&mut self, command: StreamControl) {
        if let Some(tx) = self.control_tx.take() {
            if tx.send(command).is_err() {
                tracing::warn!(target = "miaoyu_audio", "录音线程控制命令发送失败");
            }
        }
    }

    fn wait_worker(&mut self) {
        if let Some(handle) = self.worker.take() {
            join_worker(handle, "wait_worker");
        }
    }

    fn into_wav_bytes(self) -> Result<(Vec<u8>, Duration)> {
        let sample_rate = self.sample_rate;
        let channels = self.channels;
        let duration = self.started_at.elapsed();

        let samples = {
            let mut guard = self
                .samples
                .lock()
                .map_err(|_| anyhow!("录音缓冲区状态异常"))?;
            std::mem::take(&mut *guard)
        };

        let spec = hound::WavSpec {
            channels,
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut cursor = Cursor::new(Vec::new());
        {
            let mut writer =
                hound::WavWriter::new(&mut cursor, spec).context("无法创建 WAV 写入器")?;
            for sample in samples {
                writer
                    .write_sample(sample)
                    .map_err(|err| anyhow!("写入音频样本失败: {err}"))?;
            }
            writer
                .finalize()
                .map_err(|err| anyhow!("完成 WAV 数据写入失败: {err}"))?;
        }
        let wav_data = cursor.into_inner();

        Ok((wav_data, duration))
    }
}

impl Drop for DictatingStream {
    fn drop(&mut self) {
        self.send_control(StreamControl::Cancel);
        self.wait_worker();
    }
}

fn build_input_stream<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    samples: Arc<Mutex<Vec<i16>>>,
    convert: fn(T) -> i16,
) -> Result<cpal::Stream, String>
where
    T: Sample + SizedSample + Send + 'static,
{
    let err_fn = |err| tracing::error!(target = "miaoyu_audio", "输入流发生错误: {err}");

    device
        .build_input_stream(
            config,
            move |data: &[T], _| {
                if let Ok(mut buffer) = samples.lock() {
                    buffer.reserve(data.len());
                    for &sample in data {
                        buffer.push(convert(sample));
                    }
                }
            },
            err_fn,
            None,
        )
        .map_err(|err| format!("无法创建录音输入流: {err}"))
}

fn join_worker(handle: thread::JoinHandle<Result<(), String>>, context: &'static str) {
    match handle.join() {
        Ok(Ok(())) => {}
        Ok(Err(message)) => {
            tracing::warn!(
                target = "miaoyu_audio",
                context = context,
                error = %message,
                "录音线程返回错误"
            );
        }
        Err(err) => {
            tracing::warn!(
                target = "miaoyu_audio",
                context = context,
                ?err,
                "录音线程在 join 时发生 panic"
            );
        }
    }
}

fn convert_sample_f32(sample: f32) -> i16 {
    (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16
}

fn convert_sample_i16(sample: i16) -> i16 {
    sample
}

fn convert_sample_u16(sample: u16) -> i16 {
    (sample as i32 - 32_768) as i16
}

enum AudioSounds {
    StartDictating,
    StopDictating,
    Notification,
}

impl AudioSounds {
    fn play(&self) {
        let bytes = self.get_sound_bytes();
        thread::spawn(move || {
            if let Ok(mut stream) = OutputStreamBuilder::open_default_stream() {
                stream.log_on_drop(false);
                if let Ok(sink) = rodio::play(stream.mixer(), Cursor::new(bytes)) {
                    sink.sleep_until_end();
                }
            }
        });
    }

    fn get_sound_bytes(&self) -> &'static [u8] {
        match self {
            Self::StartDictating => include_bytes!("../../sounds/start-dictating.ogg"),
            Self::StopDictating => include_bytes!("../../sounds/stop-dictating.ogg"),
            Self::Notification => include_bytes!("../../sounds/notification.ogg"),
        }
    }
}

pub struct AudioDictating;

impl AudioDictating {
    pub fn play_start_sound() {
        AudioSounds::StartDictating.play();
    }

    pub fn play_stop_sound() {
        AudioSounds::StopDictating.play();
    }

    pub fn play_notification_sound() {
        AudioSounds::Notification.play();
    }

    pub fn validate_duration(duration: Duration) -> Result<()> {
        if duration < MIN_RECORDING_DURATION {
            bail!("录音时间过短，请重试");
        }
        Ok(())
    }
}
