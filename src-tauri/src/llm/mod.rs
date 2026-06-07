//! LLM 抽象层。SPEC §6。
//!
//! 设计目标：用户在设置里选 provider + 填 base_url/api_key/model 即可切换。
//! 国内 90% 的 LLM 厂商都用 OpenAI 兼容协议，所以 `openai_compat` 一个适配器覆盖
//! DeepSeek / Kimi / 通义 / SiliconFlow / OpenRouter / 自定义。
//! Claude 走 anthropic，Ollama 本地走 openai_compat 也可以（Ollama 自带 OpenAI 兼容端点）。

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub mod anthropic;
pub mod ollama;
pub mod openai_compat;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String, // "system" | "user" | "assistant"
    pub content: String,
}

impl ChatMessage {
    pub fn system(s: impl Into<String>) -> Self {
        Self {
            role: "system".into(),
            content: s.into(),
        }
    }
    pub fn user(s: impl Into<String>) -> Self {
        Self {
            role: "user".into(),
            content: s.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    pub provider: String,    // "anthropic" | "openai_compat" | "ollama"
    pub base_url: String,
    pub api_key: String,
    pub model: String,
}

#[async_trait]
pub trait LlmProvider: Send + Sync {
    fn name(&self) -> &str;

    /// 普通对话
    async fn chat(&self, messages: Vec<ChatMessage>) -> Result<String>;

    /// 结构化输出（JSON Schema 约束）
    async fn chat_structured(
        &self,
        messages: Vec<ChatMessage>,
        schema: &serde_json::Value,
    ) -> Result<serde_json::Value>;

    /// 测试连接（设置页"测试连接"按钮）
    async fn ping(&self) -> Result<()>;
}

/// 工厂：按配置造对应的 provider
pub fn build_provider(cfg: &LlmConfig) -> Result<Box<dyn LlmProvider>> {
    match cfg.provider.as_str() {
        "anthropic" => Ok(Box::new(anthropic::Anthropic::new(cfg.clone()))),
        "openai_compat" => Ok(Box::new(openai_compat::OpenAiCompat::new(cfg.clone()))),
        "ollama" => Ok(Box::new(ollama::Ollama::new(cfg.clone()))),
        other => anyhow::bail!("未知 LLM provider: {other}"),
    }
}

/// 预设清单（SPEC §6.3）—— 设置页下拉用
pub fn presets() -> Vec<(&'static str, &'static str, &'static str, &'static str)> {
    // (display_name, provider, base_url, default_model)
    vec![
        (
            "DeepSeek",
            "openai_compat",
            "https://api.deepseek.com/v1",
            "deepseek-chat",
        ),
        (
            "Claude Haiku",
            "anthropic",
            "https://api.anthropic.com",
            "claude-haiku-4-5-20251001",
        ),
        (
            "Kimi (Moonshot)",
            "openai_compat",
            "https://api.moonshot.cn/v1",
            "moonshot-v1-8k",
        ),
        (
            "通义千问",
            "openai_compat",
            "https://dashscope.aliyuncs.com/compatible-mode/v1",
            "qwen-turbo",
        ),
        (
            "SiliconFlow",
            "openai_compat",
            "https://api.siliconflow.cn/v1",
            "Qwen/Qwen2.5-7B-Instruct",
        ),
        (
            "OpenRouter",
            "openai_compat",
            "https://openrouter.ai/api/v1",
            "deepseek/deepseek-chat",
        ),
        (
            "Ollama (本地)",
            "ollama",
            "http://localhost:11434/v1",
            "qwen2.5:7b",
        ),
    ]
}
