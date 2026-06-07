//! 本地 SenseVoice 适配器（推荐）。
//! SenseVoice 是阿里达摩院开源的语音模型，中文效果比 Whisper 好。
//! 调用方式：通过 funasr-onnx 的 Python/SenseVoice 离线工具，或直接调用
//! `sensevoice` CLI（funasr-onnx 包装的 Go/Rust binary）。
//!
//! 第一版同样搭骨架。

use super::{SttConfig, SttProvider};
use anyhow::{Context, Result};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use tokio::process::Command;

pub struct SenseVoice {
    cfg: SttConfig,
}

impl SenseVoice {
    pub fn new(cfg: SttConfig) -> Self {
        Self { cfg }
    }

    fn binary(&self) -> PathBuf {
        PathBuf::from(
            self.cfg
                .binary_path
                .clone()
                .unwrap_or_else(|| "sensevoice-cli".to_string()),
        )
    }
}

#[async_trait]
impl SttProvider for SenseVoice {
    fn name(&self) -> &str {
        "sensevoice"
    }
    fn is_local(&self) -> bool {
        true
    }

    async fn transcribe(&self, audio_path: &Path, _lang: Option<&str>) -> Result<String> {
        let binary = self.binary();
        let output = Command::new(&binary)
            .arg(audio_path.to_string_lossy().as_ref())
            .output()
            .await
            .with_context(|| format!("执行 sensevoice 失败：{}", binary.display()))?;

        if !output.status.success() {
            anyhow::bail!(
                "sensevoice 退出码 {:?}: {}",
                output.status.code(),
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    async fn ping(&self) -> Result<()> {
        let binary = self.binary();
        match Command::new(&binary).arg("--help").output().await {
            Ok(_) => Ok(()),
            Err(e) => anyhow::bail!("sensevoice 二进制不可用 ({}): {e}", binary.display()),
        }
    }
}