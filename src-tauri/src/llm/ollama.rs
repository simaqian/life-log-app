//! 本地 Ollama 适配器。
//! Ollama 自带 OpenAI 兼容端点（base_url 写 http://localhost:11434/v1），所以
//! 大部分场景直接走 openai_compat 即可。这里专门做一个适配器是为了：
//!   - 用 Ollama 原生 /api/chat 拿到 format=json 的稳定支持
//!   - 自动检测模型是否已 pull
//!
//! 第一版先委托给 openai_compat（Ollama 兼容端点完备），后续按需扩展。

use super::{ChatMessage, LlmConfig, LlmProvider, openai_compat::OpenAiCompat};
use anyhow::Result;
use async_trait::async_trait;

pub struct Ollama {
    inner: OpenAiCompat,
}

impl Ollama {
    pub fn new(cfg: LlmConfig) -> Self {
        // Ollama 本地不需要真 api_key，但 openai_compat 会发 Bearer
        // 给个占位值，Ollama 忽略
        let mut cfg = cfg;
        if cfg.api_key.is_empty() {
            cfg.api_key = "ollama".into();
        }
        Self {
            inner: OpenAiCompat::new(cfg),
        }
    }
}

#[async_trait]
impl LlmProvider for Ollama {
    fn name(&self) -> &str {
        "ollama"
    }
    async fn chat(&self, messages: Vec<ChatMessage>) -> Result<String> {
        self.inner.chat(messages).await
    }
    async fn chat_structured(
        &self,
        messages: Vec<ChatMessage>,
        schema: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        self.inner.chat_structured(messages, schema).await
    }
    async fn ping(&self) -> Result<()> {
        self.inner.ping().await
    }
}
