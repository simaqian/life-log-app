//! OpenAI 兼容 API 适配器。
//! 覆盖 DeepSeek / Kimi / 通义 / SiliconFlow / OpenRouter / Ollama / 任何自定义 OpenAI 兼容端点。

use super::{ChatMessage, LlmConfig, LlmProvider};
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;

pub struct OpenAiCompat {
    cfg: LlmConfig,
    client: reqwest::Client,
}

impl OpenAiCompat {
    pub fn new(cfg: LlmConfig) -> Self {
        Self {
            cfg,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .expect("reqwest client build"),
        }
    }

    fn endpoint(&self, path: &str) -> String {
        format!("{}{}", self.cfg.base_url.trim_end_matches('/'), path)
    }
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: &'a [ChatMessage],
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: RespMessage,
}

#[derive(Deserialize)]
struct RespMessage {
    content: String,
}

#[async_trait]
impl LlmProvider for OpenAiCompat {
    fn name(&self) -> &str {
        "openai_compat"
    }

    async fn chat(&self, messages: Vec<ChatMessage>) -> Result<String> {
        let req = ChatRequest {
            model: &self.cfg.model,
            messages: &messages,
            response_format: None,
            temperature: Some(0.3),
        };
        let resp = self
            .client
            .post(self.endpoint("/chat/completions"))
            .bearer_auth(&self.cfg.api_key)
            .json(&req)
            .send()
            .await
            .context("LLM 请求失败")?
            .error_for_status()
            .context("LLM 返回非 2xx")?
            .json::<ChatResponse>()
            .await
            .context("LLM 响应解析失败")?;
        Ok(resp
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .unwrap_or_default())
    }

    async fn chat_structured(
        &self,
        messages: Vec<ChatMessage>,
        schema: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        // OpenAI 兼容端点的结构化输出有两种方式：
        //   1) response_format: {type: "json_object"} —— 仅保证返回合法 JSON，schema 靠 prompt
        //   2) response_format: {type: "json_schema", json_schema: ...} —— 严格 schema（仅 OpenAI 原生 + 部分新版本）
        // 这里走方案 1（最大兼容性），同时把 schema 塞进 system prompt
        let mut msgs = messages;
        msgs.insert(
            0,
            ChatMessage::system(format!(
                "你必须严格按以下 JSON Schema 返回 JSON，不要任何 markdown 包裹或解释文字：\n{}",
                serde_json::to_string_pretty(schema)?
            )),
        );

        let req = ChatRequest {
            model: &self.cfg.model,
            messages: &msgs,
            response_format: Some(json!({"type": "json_object"})),
            temperature: Some(0.1),
        };

        let resp = self
            .client
            .post(self.endpoint("/chat/completions"))
            .bearer_auth(&self.cfg.api_key)
            .json(&req)
            .send()
            .await
            .context("LLM 请求失败")?
            .error_for_status()
            .context("LLM 返回非 2xx")?
            .json::<ChatResponse>()
            .await
            .context("LLM 响应解析失败")?;
        let content = resp
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .unwrap_or_default();
        serde_json::from_str(&content)
            .with_context(|| format!("LLM 返回非合法 JSON: {content}"))
    }

    async fn ping(&self) -> Result<()> {
        // 发一个最便宜的请求
        let _ = self
            .chat(vec![ChatMessage::user("ping")])
            .await?;
        Ok(())
    }
}
