// 前端 → Rust 后端的胶水层
import { invoke } from "@tauri-apps/api/core";

export interface EventIn {
  type: "checkin" | "item" | "note";
  raw_text?: string;
  raw_voice_path?: string;
  structured?: Record<string, unknown>;
  tags?: string[];
  media?: string[];
  source?: string;
}

export interface EventOut {
  id: number;
  ts: number;
  type: string;
  raw_text?: string;
  structured?: Record<string, unknown>;
  tags?: string[];
}

export interface LlmPreset {
  name: string;
  provider: string;
  base_url: string;
  model: string;
}

export interface ExtractResult {
  structured: Record<string, unknown>;
  tags: string[];
  provider: string;
  model: string;
}

export const api = {
  ping: () => invoke<string>("ping"),
  createEvent: (input: EventIn) => invoke<number>("create_event", { input }),
  updateEventStructured: (
    id: number,
    structured: Record<string, unknown> | null,
    tags: string[] | null,
    llm_provider: string | null,
    llm_model: string | null
  ) =>
    invoke<void>("update_event_structured", {
      id,
      structured,
      tags,
      llmProvider: llm_provider,
      llmModel: llm_model,
    }),
  listRecentEvents: (limit?: number) =>
    invoke<EventOut[]>("list_recent_events", { limit }),
  getSetting: (key: string) => invoke<string | null>("get_setting", { key }),
  setSetting: (key: string, value: string) =>
    invoke<void>("set_setting", { key, value }),

  // LLM
  llmPresets: () => invoke<LlmPreset[]>("llm_presets"),
  llmTestConnection: () => invoke<string>("llm_test_connection"),
  llmExtractCheckin: (raw_text: string) =>
    invoke<ExtractResult>("llm_extract_checkin", { rawText: raw_text }),
};
