//! 本地 whisper.cpp 适配器。
//! 调用用户安装的 whisper-cli 二进制（来自 whisper.cpp 项目）。
//! 第一版只搭好骨架；实际转写在引入二进制前会报错提示用户。

use super::{SttConfig, SttProvider};
use anyhow::{Context, Result};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use tokio::process::Command;

pub struct WhisperLocal {
    cfg: SttConfig,
}

impl WhisperLocal {
    pub fn new(cfg: SttConfig) -> Self {
        Self { cfg }
    }

    fn binary(&self) -> PathBuf {
        PathBuf::from(
            self.cfg
                .binary_path
                .clone()
                .unwrap_or_else(|| "whisper-cli".to_string()),
        )
    }

    fn model(&self) -> Result<PathBuf> {
        let p = self
            .cfg
            .model_path
            .clone()
            .context("whisper_local 缺少 model_path（请在设置中配置 .bin 模型文件路径）")?;
        Ok(PathBuf::from(p))
    }
}

#[async_trait]
impl SttProvider for WhisperLocal {
    fn name(&self) -> &str {
        "whisper_local"
    }
    fn is_local(&self) -> bool {
        true
    }

    async fn transcribe(&self, audio_path: &Path, lang: Option<&str>) -> Result<String> {
        let model = self.model()?;
        let binary = self.binary();
        let lang = lang.or(self.cfg.language.as_deref()).unwrap_or("zh");

        let output = Command::new(&binary)
            .args([
                "-m",
                model.to_string_lossy().as_ref(),
                "-f",
                audio_path.to_string_lossy().as_ref(),
                "-l",
                lang,
                "-otxt",
                "-of",
                "/tmp/life-log-stt-out",
            ])
            .output()
            .await
            .with_context(|| format!("执行 whisper 失败：{}", binary.display()))?;

        if !output.status.success() {
            anyhow::bail!(
                "whisper 退出码 {:?}: {}",
                output.status.code(),
                String::from_utf8_lossy(&output.stderr)
            );
        }
        let txt = std::fs::read_to_string("/tmp/life-log-stt-out.txt")
            .context("读取 whisper 输出失败")?;
        Ok(txt.trim().to_string())
    }

    async fn ping(&self) -> Result<()> {
        // 检查二进制是否存在
        let binary = self.binary();
        let out = Command::new(&binary).arg("--help").output().await;
        match out {
            Ok(_) => Ok(()),
            Err(e) => anyhow::bail!("whisper 二进制不可用 ({}): {e}", binary.display()),
        }
    }
}
