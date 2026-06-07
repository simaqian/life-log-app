<script lang="ts">
  import { api, type LlmPreset } from "../lib/api";

  let presets = $state<LlmPreset[]>([]);
  let selectedPresetName = $state<string>("");
  let provider = $state("openai_compat");
  let baseUrl = $state("");
  let apiKey = $state("");
  let model = $state("");

  let testResult = $state<string | null>(null);
  let testError = $state<string | null>(null);
  let testing = $state(false);
  let saving = $state(false);
  let saveMsg = $state<string | null>(null);

  // 初始化：加载预设 + 读已有配置
  $effect(() => {
    (async () => {
      try {
        presets = await api.llmPresets();
        const [p, b, k, m] = await Promise.all([
          api.getSetting("llm.provider"),
          api.getSetting("llm.base_url"),
          api.getSetting("llm.api_key"),
          api.getSetting("llm.model"),
        ]);
        if (p) provider = p;
        if (b) baseUrl = b;
        if (k) apiKey = k;
        if (m) model = m;
        // 猜出当前选了哪个预设
        const matched = presets.find(
          (x) => x.provider === provider && x.base_url === baseUrl
        );
        if (matched) selectedPresetName = matched.name;
      } catch (e) {
        testError = String(e);
      }
    })();
  });

  function applyPreset(name: string) {
    const p = presets.find((x) => x.name === name);
    if (!p) return;
    selectedPresetName = name;
    provider = p.provider;
    baseUrl = p.base_url;
    if (!model || presets.some((x) => x.model === model)) {
      model = p.model;
    }
  }

  async function save() {
    saving = true;
    saveMsg = null;
    try {
      await api.setSetting("llm.provider", provider);
      await api.setSetting("llm.base_url", baseUrl);
      await api.setSetting("llm.api_key", apiKey);
      await api.setSetting("llm.model", model);
      saveMsg = "已保存";
      setTimeout(() => (saveMsg = null), 2000);
    } catch (e) {
      saveMsg = "保存失败：" + String(e);
    } finally {
      saving = false;
    }
  }

  async function test() {
    testing = true;
    testResult = null;
    testError = null;
    try {
      // 先保存，再测（不然 backend 读到的是旧值）
      await save();
      testResult = await api.llmTestConnection();
    } catch (e) {
      testError = String(e);
    } finally {
      testing = false;
    }
  }
</script>

<div class="p-6 max-w-xl mx-auto">
  <div class="flex items-center justify-between mb-6">
    <h1 class="text-xl font-semibold">设置</h1>
    <button
      onclick={() => (window.location.hash = "#/")}
      class="px-3 py-1 text-sm text-gray-600 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800 rounded">
      ← 返回主页
    </button>
  </div>

  <section class="mb-6">
    <h2 class="text-sm font-medium text-gray-700 dark:text-gray-300 mb-3 border-b border-gray-200 dark:border-gray-700 pb-1">
      LLM 服务
    </h2>

    <div class="space-y-3">
      <div>
        <label class="block text-xs text-gray-500 mb-1" for="preset">预设</label>
        <select
          id="preset"
          class="w-full px-2 py-1.5 text-sm border border-gray-300 dark:border-gray-600 rounded bg-white dark:bg-gray-800"
          value={selectedPresetName}
          onchange={(e) => applyPreset((e.target as HTMLSelectElement).value)}
        >
          <option value="">（自定义）</option>
          {#each presets as p}
            <option value={p.name}>{p.name}</option>
          {/each}
        </select>
      </div>

      <div>
        <label class="block text-xs text-gray-500 mb-1" for="base">Base URL</label>
        <input
          id="base"
          type="text"
          bind:value={baseUrl}
          placeholder="https://api.deepseek.com/v1"
          class="w-full px-2 py-1.5 text-sm border border-gray-300 dark:border-gray-600 rounded bg-white dark:bg-gray-800 font-mono"
        />
      </div>

      <div>
        <label class="block text-xs text-gray-500 mb-1" for="key">API Key</label>
        <input
          id="key"
          type="password"
          bind:value={apiKey}
          placeholder="sk-..."
          class="w-full px-2 py-1.5 text-sm border border-gray-300 dark:border-gray-600 rounded bg-white dark:bg-gray-800 font-mono"
        />
        <p class="text-xs text-gray-400 mt-1">
          ⚠️ 当前版本明文存在本地数据库；后续会迁移到 macOS Keychain
        </p>
      </div>

      <div>
        <label class="block text-xs text-gray-500 mb-1" for="model">Model</label>
        <input
          id="model"
          type="text"
          bind:value={model}
          placeholder="deepseek-chat"
          class="w-full px-2 py-1.5 text-sm border border-gray-300 dark:border-gray-600 rounded bg-white dark:bg-gray-800 font-mono"
        />
      </div>

      <div>
        <label class="block text-xs text-gray-500 mb-1" for="provider">Provider 类型</label>
        <select
          id="provider"
          bind:value={provider}
          class="w-full px-2 py-1.5 text-sm border border-gray-300 dark:border-gray-600 rounded bg-white dark:bg-gray-800"
        >
          <option value="openai_compat">openai_compat（DeepSeek/Kimi/通义/SiliconFlow 等）</option>
          <option value="anthropic">anthropic（Claude）</option>
          <option value="ollama">ollama（本地）</option>
        </select>
      </div>

      <div class="flex gap-2 pt-2">
        <button
          onclick={save}
          disabled={saving}
          class="px-4 py-1.5 text-sm bg-blue-500 text-white rounded hover:bg-blue-600 disabled:opacity-50"
        >{saving ? "保存中…" : "保存"}</button>
        <button
          onclick={test}
          disabled={testing}
          class="px-4 py-1.5 text-sm bg-gray-200 dark:bg-gray-700 rounded hover:bg-gray-300 disabled:opacity-50"
        >{testing ? "测试中…" : "保存并测试连接"}</button>
        {#if saveMsg}
          <span class="text-xs self-center text-gray-500">{saveMsg}</span>
        {/if}
      </div>

      {#if testResult}
        <p class="text-sm text-green-600 mt-2 p-2 bg-green-50 dark:bg-green-950 rounded">{testResult}</p>
      {/if}
      {#if testError}
        <p class="text-sm text-red-600 mt-2 p-2 bg-red-50 dark:bg-red-950 rounded">{testError}</p>
      {/if}
    </div>
  </section>

  <section>
    <h2 class="text-sm font-medium text-gray-700 dark:text-gray-300 mb-3 border-b border-gray-200 dark:border-gray-700 pb-1">
      关于
    </h2>
    <p class="text-xs text-gray-500">
      Life-Log v0.1.0 · 数据本地存储于
      <code class="bg-gray-100 dark:bg-gray-800 px-1 rounded">~/Library/Application Support/com.crd.life-log/data</code>
    </p>
  </section>
</div>
