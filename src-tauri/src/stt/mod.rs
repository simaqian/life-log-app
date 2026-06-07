//! STT 抽象层。SPEC §7。
//!
//! 第一版三个 provider：本地 whisper.cpp、本地 SenseVoice、云端（OpenAI/SiliconFlow）。
//! 默认推荐本地 SenseVoice（中文最准）。
//!
//! 注意：本地 provider 需要先下载模型/二进制。第一版骨架不包含模型下载逻辑，
//! 模型路径由用户在设置里指定，找不到就报错。

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::Path;

pub mod cloud;
pub mod sensevoice;
pub mod whisper_local;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SttConfig {
    pub provider: String,        // "whisper_local" | "sensevoice" | "cloud"
    pub model_path: Option<String>,
    pub binary_path: Option<String>,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub language: Option<String>, // "zh" / "en" / None=auto
}

#[async_trait]
pub trait SttProvider: Send + Sync {
    fn name(&self) -> &str;
    fn is_local(&self) -> bool;

    /// 转写音频文件
    async fn transcribe(&self, audio_path: &Path, lang: Option<&str>) -> Result<String>;

    /// 测试可用性
    async fn ping(&self) -> Result<()>;
}

pub fn build_provider(cfg: &SttConfig) -> Result<Box<dyn SttProvider>> {
    match cfg.provider.as_str() {
        "whisper_local" => Ok(Box::new(whisper_local::WhisperLocal::new(cfg.clone()))),
        "sensevoice" => Ok(Box::new(sensevoice::SenseVoice::new(cfg.clone()))),
        "cloud" => Ok(Box::new(cloud::CloudStt::new(cfg.clone()))),
        other => anyhow::bail!("未知 STT provider: {other}"),
    }
}
