//! 云端 STT 适配器（OpenAI Whisper API / SiliconFlow / Groq 等）。
//! 全部用 OpenAI 兼容端点 `/v1/audio/transcriptions`。
//! 第一版只搭好骨架。

use super::{SttConfig, SttProvider};
use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::multipart;
use std::path::Path;

pub struct CloudStt {
    cfg: SttConfig,
    client: reqwest::Client,
}

impl CloudStt {
    pub fn new(cfg: SttConfig) -> Self {
        Self {
            cfg,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .expect("reqwest client build"),
        }
    }

    fn endpoint(&self) -> String {
        let base = self
            .cfg
            .base_url
            .clone()
            .unwrap_or_else(|| "https://api.openai.com/v1".to_string());
        format!("{}/audio/transcriptions", base.trim_end_matches('/'))
    }

    fn api_key(&self) -> Result<&str> {
        self.cfg
            .api_key
            .as_deref()
            .context("云端 STT 需要 api_key（请在设置中配置）")
    }

    fn model(&self) -> &str {
        self.cfg.model.as_deref().unwrap_or("whisper-1")
    }
}

#[async_trait]
impl SttProvider for CloudStt {
    fn name(&self) -> &str {
        "cloud_stt"
    }
    fn is_local(&self) -> bool {
        false
    }

    async fn transcribe(&self, audio_path: &Path, lang: Option<&str>) -> Result<String> {
        // 异步读文件后转成 multipart::Part
        let bytes = tokio::fs::read(audio_path)
            .await
            .context("读取音频文件失败")?;
        let filename = audio_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("audio.m4a")
            .to_string();
        let part = multipart::Part::bytes(bytes)
            .file_name(filename)
            .mime_str("audio/mpeg")
            .context("构造 multipart part 失败")?;
        let form = multipart::Form::new()
            .part("file", part)
            .text("model", self.model().to_string())
            .text("response_format", "json")
            .text("language", lang.unwrap_or("zh").to_string());

        #[derive(serde::Deserialize)]
        struct TranscriptionResponse {
            text: String,
        }

        let resp = self
            .client
            .post(self.endpoint())
            .bearer_auth(self.api_key()?)
            .multipart(form)
            .send()
            .await
            .context("STT API 请求失败")?
            .error_for_status()
            .context("STT API 返回非 2xx")?
            .json::<TranscriptionResponse>()
            .await
            .context("STT API 响应解析失败")?;

        Ok(resp.text.trim().to_string())
    }

    async fn ping(&self) -> Result<()> {
        self.api_key().map(|_| ())
    }
}