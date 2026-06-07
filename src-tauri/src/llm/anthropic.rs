//! Anthropic / Claude API 适配器。
//! Claude 用自己的 API 格式，不兼容 OpenAI。结构化输出走 tool_use。

use super::{ChatMessage, LlmConfig, LlmProvider};
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;

const API_VERSION: &str = "2023-06-01";

pub struct Anthropic {
    cfg: LlmConfig,
    client: reqwest::Client,
}

impl Anthropic {
    pub fn new(cfg: LlmConfig) -> Self {
        Self {
            cfg,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .expect("reqwest client build"),
        }
    }

    fn endpoint(&self) -> String {
        format!(
            "{}/v1/messages",
            self.cfg.base_url.trim_end_matches('/')
        )
    }

    fn split_system(messages: &[ChatMessage]) -> (Option<String>, Vec<serde_json::Value>) {
        // Anthropic 要求 system 单独传，user/assistant 走 messages 数组
        let mut system: Option<String> = None;
        let mut msgs = vec![];
        for m in messages {
            if m.role == "system" {
                system = Some(m.content.clone());
            } else {
                msgs.push(json!({"role": m.role, "content": m.content}));
            }
        }
        (system, msgs)
    }
}

#[derive(Deserialize)]
struct MessageResponse {
    content: Vec<ContentBlock>,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse { input: serde_json::Value, #[allow(dead_code)] name: String },
}

#[async_trait]
impl LlmProvider for Anthropic {
    fn name(&self) -> &str {
        "anthropic"
    }

    async fn chat(&self, messages: Vec<ChatMessage>) -> Result<String> {
        let (system, msgs) = Self::split_system(&messages);
        let mut body = json!({
            "model": &self.cfg.model,
            "max_tokens": 4096,
            "messages": msgs,
        });
        if let Some(s) = system {
            body["system"] = json!(s);
        }
        let resp = self
            .client
            .post(self.endpoint())
            .header("x-api-key", &self.cfg.api_key)
            .header("anthropic-version", API_VERSION)
            .json(&body)
            .send()
            .await
            .context("Anthropic 请求失败")?
            .error_for_status()
            .context("Anthropic 返回非 2xx")?
            .json::<MessageResponse>()
            .await
            .context("Anthropic 响应解析失败")?;

        let text = resp
            .content
            .into_iter()
            .filter_map(|b| match b {
                ContentBlock::Text { text } => Some(text),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n");
        Ok(text)
    }

    async fn chat_structured(
        &self,
        messages: Vec<ChatMessage>,
        schema: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        let (system, msgs) = Self::split_system(&messages);
        // Anthropic 用 tool_use 做结构化：定义一个 tool，强制模型调用
        let tool = json!({
            "name": "extract_structured",
            "description": "提取结构化字段并返回 JSON。",
            "input_schema": schema,
        });
        let mut body = json!({
            "model": &self.cfg.model,
            "max_tokens": 4096,
            "messages": msgs,
            "tools": [tool],
            "tool_choice": {"type": "tool", "name": "extract_structured"},
        });
        if let Some(s) = system {
            body["system"] = json!(s);
        }

        let resp = self
            .client
            .post(self.endpoint())
            .header("x-api-key", &self.cfg.api_key)
            .header("anthropic-version", API_VERSION)
            .json(&body)
            .send()
            .await
            .context("Anthropic 请求失败")?
            .error_for_status()
            .context("Anthropic 返回非 2xx")?
            .json::<MessageResponse>()
            .await
            .context("Anthropic 响应解析失败")?;

        for block in resp.content {
            if let ContentBlock::ToolUse { input, .. } = block {
                return Ok(input);
            }
        }
        anyhow::bail!("Anthropic 未返回 tool_use 块")
    }

    async fn ping(&self) -> Result<()> {
        let _ = self.chat(vec![ChatMessage::user("ping")]).await?;
        Ok(())
    }
}
