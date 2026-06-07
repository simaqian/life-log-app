//! Tauri command 处理（前端 invoke 的入口）。
//! 第一版只暴露：保存 event、查询 events、读写 settings、ping 通用工具。
//! 后续扩展任务/物品/LLM/STT 调用。

use crate::db::Db;
use crate::llm::{self, ChatMessage, LlmConfig};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tauri::State;

pub struct AppState {
    pub db: Arc<Db>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EventIn {
    pub r#type: String,            // "checkin" | "item" | "note"
    pub raw_text: Option<String>,
    pub raw_voice_path: Option<String>,
    pub structured: Option<serde_json::Value>,
    pub tags: Option<Vec<String>>,
    pub media: Option<Vec<String>>,
    pub source: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct EventOut {
    pub id: i64,
    pub ts: i64,
    pub r#type: String,
    pub raw_text: Option<String>,
    pub structured: Option<serde_json::Value>,
    pub tags: Option<Vec<String>>,
}

/// 健康检查
#[tauri::command]
pub fn ping(state: State<AppState>) -> Result<String, String> {
    let n = state.db.table_count().map_err(|e| e.to_string())?;
    Ok(format!("ok ({} 张表)", n))
}

/// 新增一条 event
#[tauri::command]
pub fn create_event(
    input: EventIn,
    state: State<AppState>,
) -> Result<i64, String> {
    let now = chrono::Utc::now().timestamp_millis();
    let conn = state.db.conn.lock().map_err(|e| e.to_string())?;
    let id = conn
        .query_row(
            r#"INSERT INTO events
                (ts, type, raw_text, raw_voice_path, structured, tags, media, source, created_at, updated_at)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)
               RETURNING id"#,
            params![
                now,
                input.r#type,
                input.raw_text,
                input.raw_voice_path,
                input.structured.map(|v| v.to_string()),
                input.tags.map(|t| serde_json::to_string(&t).unwrap_or_default()),
                input.media.map(|m| serde_json::to_string(&m).unwrap_or_default()),
                input.source.unwrap_or_else(|| "manual".into()),
                now,
            ],
            |r| r.get::<_, i64>(0),
        )
        .map_err(|e| e.to_string())?;
    Ok(id)
}

/// 更新一条 event 的 structured / tags / llm 字段（LLM 异步提取完成后调）
#[tauri::command]
pub fn update_event_structured(
    id: i64,
    structured: Option<serde_json::Value>,
    tags: Option<Vec<String>>,
    llm_provider: Option<String>,
    llm_model: Option<String>,
    state: State<AppState>,
) -> Result<(), String> {
    let now = chrono::Utc::now().timestamp_millis();
    let conn = state.db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        r#"UPDATE events
           SET structured = ?1,
               tags = COALESCE(?2, tags),
               llm_provider = COALESCE(?3, llm_provider),
               llm_model = COALESCE(?4, llm_model),
               updated_at = ?5
           WHERE id = ?6"#,
        params![
            structured.map(|v| v.to_string()),
            tags.map(|t| serde_json::to_string(&t).unwrap_or_default()),
            llm_provider,
            llm_model,
            now,
            id,
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// 查最近 N 条 event
#[tauri::command]
pub fn list_recent_events(
    limit: Option<i64>,
    state: State<AppState>,
) -> Result<Vec<EventOut>, String> {
    let limit = limit.unwrap_or(50);
    let conn = state.db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT id, ts, type, raw_text, structured, tags
             FROM events
             WHERE deleted_at IS NULL
             ORDER BY ts DESC
             LIMIT ?1",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![limit], |r| {
            let structured_str: Option<String> = r.get(4)?;
            let tags_str: Option<String> = r.get(5)?;
            Ok(EventOut {
                id: r.get(0)?,
                ts: r.get(1)?,
                r#type: r.get(2)?,
                raw_text: r.get(3)?,
                structured: structured_str
                    .as_deref()
                    .and_then(|s| serde_json::from_str(s).ok()),
                tags: tags_str
                    .as_deref()
                    .and_then(|s| serde_json::from_str(s).ok()),
            })
        })
        .map_err(|e| e.to_string())?;
    let out: Result<Vec<_>, _> = rows.collect();
    out.map_err(|e| e.to_string())
}

/// 读 setting
#[tauri::command]
pub fn get_setting(key: String, state: State<AppState>) -> Result<Option<String>, String> {
    let conn = state.db.conn.lock().map_err(|e| e.to_string())?;
    let v: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![key],
            |r| r.get(0),
        )
        .ok();
    Ok(v)
}

/// 写 setting
#[tauri::command]
pub fn set_setting(
    key: String,
    value: String,
    state: State<AppState>,
) -> Result<(), String> {
    let conn = state.db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

// ============================================================
// LLM 相关
// ============================================================

/// LLM 预设清单（前端设置页下拉用）
#[derive(Serialize)]
pub struct LlmPreset {
    pub name: &'static str,
    pub provider: &'static str,
    pub base_url: &'static str,
    pub model: &'static str,
}

#[tauri::command]
pub fn llm_presets() -> Vec<LlmPreset> {
    llm::presets()
        .into_iter()
        .map(|(name, provider, base_url, model)| LlmPreset {
            name,
            provider,
            base_url,
            model,
        })
        .collect()
}

/// 从 settings 读出当前 LLM 配置
fn read_llm_config(db: &Db) -> Result<LlmConfig, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let get = |k: &str| -> Option<String> {
        conn.query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![k],
            |r| r.get::<_, String>(0),
        )
        .ok()
    };
    let provider = get("llm.provider").unwrap_or_else(|| "openai_compat".into());
    let base_url = get("llm.base_url").ok_or("缺少 llm.base_url（请到设置页配置）")?;
    let api_key = get("llm.api_key").ok_or("缺少 llm.api_key（请到设置页配置）")?;
    let model = get("llm.model").ok_or("缺少 llm.model（请到设置页配置）")?;
    Ok(LlmConfig {
        provider,
        base_url,
        api_key,
        model,
    })
}

/// 测试 LLM 连接（设置页"测试连接"按钮）
#[tauri::command]
pub async fn llm_test_connection(state: State<'_, AppState>) -> Result<String, String> {
    let cfg = read_llm_config(&state.db)?;
    let provider = llm::build_provider(&cfg).map_err(|e| e.to_string())?;
    let resp = provider
        .chat(vec![ChatMessage::user(
            "请回复'pong'两个字，不要任何其他内容。",
        )])
        .await
        .map_err(|e| format!("调用失败：{e}"))?;
    Ok(format!("✅ 连接成功，模型回复：{}", resp.trim()))
}

/// 用 LLM 提取 checkin 的结构化字段
/// 返回 { structured: {...}, tags: [...], provider, model }
#[tauri::command]
pub async fn llm_extract_checkin(
    raw_text: String,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let cfg = read_llm_config(&state.db)?;
    let provider = llm::build_provider(&cfg).map_err(|e| e.to_string())?;

    // checkin 结构化 schema —— 对应 SPEC §4.3
    let schema = json!({
        "type": "object",
        "properties": {
            "activity": { "type": "string", "description": "用一个动词短语概括用户在做什么，比如 写代码、开会、吃饭、休息" },
            "project": { "type": ["string", "null"], "description": "如果提到具体项目/事情，提取出来；否则 null" },
            "mood": {
                "type": ["string", "null"],
                "enum": ["专注", "平稳", "兴奋", "疲惫", "焦虑", "低落", "开心", "无聊", "其他", null],
                "description": "情绪状态。如果用户没明显表达，留 null"
            },
            "energy": { "type": ["integer", "null"], "minimum": 1, "maximum": 10, "description": "精力 1-10，没说就 null" },
            "location": { "type": ["string", "null"], "description": "地点，比如 家、公司、咖啡馆。没说就 null" },
            "with_whom": { "type": "array", "items": { "type": "string" }, "description": "和谁在一起。没说就 []" },
            "tags": { "type": "array", "items": { "type": "string" }, "description": "1-3 个相关标签，比如 #工作 #运动 #社交" }
        },
        "required": ["activity", "tags"]
    });

    let messages = vec![
        ChatMessage::system(
            "你是一个生活记录助手。用户会说一句话描述他这个小时在做什么。\
             你需要从中提取结构化字段。\
             - activity 必须是简短的动词短语（2-6 个汉字）\
             - 如果用户提到具体的项目或事物，放进 project\
             - 情绪要谨慎，没有明显信号就 null\
             - tags 提取 1-3 个，每个不带 # 前缀\
             返回严格的 JSON，不要任何 markdown 包裹。",
        ),
        ChatMessage::user(raw_text.clone()),
    ];

    let structured = provider
        .chat_structured(messages, &schema)
        .await
        .map_err(|e| format!("LLM 调用失败：{e}"))?;

    // tags 单独提出来（前端那一列直接显示）
    let tags = structured
        .get("tags")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Ok(json!({
        "structured": structured,
        "tags": tags,
        "provider": cfg.provider,
        "model": cfg.model,
    }))
}
