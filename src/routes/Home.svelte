<script lang="ts">
  import { api, type EventOut } from "../lib/api";

  let pingStatus = $state<string>("");
  let recent = $state<EventOut[]>([]);
  let pingError = $state<string | null>(null);
  let autoRefresh = $state(true);

  async function checkPing() {
    pingError = null;
    try {
      pingStatus = await api.ping();
    } catch (e) {
      pingError = String(e);
    }
  }

  async function loadRecent() {
    try {
      recent = await api.listRecentEvents(20);
    } catch (e) {
      pingError = String(e);
    }
  }

  $effect(() => {
    checkPing();
    loadRecent();
    if (!autoRefresh) return;
    // 每 3 秒刷新一次（这样 LLM 结构化完成后能很快看到）
    const t = setInterval(loadRecent, 3000);
    return () => clearInterval(t);
  });

  function structuredSummary(s: Record<string, unknown> | undefined): string {
    if (!s) return "";
    const parts: string[] = [];
    if (s.activity) parts.push(String(s.activity));
    if (s.mood) parts.push(`心情：${s.mood}`);
    if (s.energy) parts.push(`精力 ${s.energy}/10`);
    if (s.project) parts.push(`项目：${s.project}`);
    if (s.location) parts.push(`📍 ${s.location}`);
    if (Array.isArray(s.with_whom) && s.with_whom.length)
      parts.push(`与 ${s.with_whom.join("、")}`);
    return parts.join("  ·  ");
  }

  function timeAgo(ts: number): string {
    const diff = Date.now() - ts;
    if (diff < 60_000) return "刚刚";
    if (diff < 3600_000) return `${Math.floor(diff / 60_000)} 分钟前`;
    if (diff < 86400_000) return `${Math.floor(diff / 3600_000)} 小时前`;
    return new Date(ts).toLocaleString("zh-CN", {
      month: "2-digit",
      day: "2-digit",
      hour: "2-digit",
      minute: "2-digit",
    });
  }
</script>

<div class="p-6 max-w-2xl mx-auto">
  <h1 class="text-2xl font-semibold mb-1">Life-Log</h1>
  <p class="text-sm text-gray-500 mb-4">
    v0.1.0 · 开发期主窗口
  </p>

  <!-- 快捷入口（菜单栏菜单暂时不可用时用这里） -->
  <div class="flex gap-2 mb-6">
    <button
       onclick={() => (window.location.hash = "#/checkin")}
       class="px-3 py-1.5 text-sm bg-blue-500 text-white rounded hover:bg-blue-600">
      🎤 打卡
    </button>
    <button
       onclick={() => (window.location.hash = "#/settings")}
       class="px-3 py-1.5 text-sm bg-gray-200 dark:bg-gray-700 rounded hover:bg-gray-300">
      ⚙️ 设置
    </button>
    <button
       onclick={() => (window.location.hash = "#/")}
       class="px-3 py-1.5 text-sm bg-gray-200 dark:bg-gray-700 rounded hover:bg-gray-300">
      🏠 主页
    </button>
  </div>

  <section class="mb-6 p-4 rounded-lg bg-gray-50 dark:bg-gray-800">
    <h2 class="text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">后端状态</h2>
    {#if pingError}
      <p class="text-red-600 text-sm">❌ {pingError}</p>
    {:else if pingStatus}
      <p class="text-green-600 text-sm">✅ {pingStatus}</p>
    {:else}
      <p class="text-gray-400 text-sm">…</p>
    {/if}
    <div class="flex items-center gap-4 mt-2">
      <button
        class="px-3 py-1 text-xs bg-blue-500 text-white rounded hover:bg-blue-600"
        onclick={checkPing}>重新检查</button
      >
      <label class="text-xs text-gray-500 flex items-center gap-1">
        <input type="checkbox" bind:checked={autoRefresh} />
        自动刷新（3s）
      </label>
    </div>
  </section>

  <section>
    <div class="flex items-center justify-between mb-2">
      <h2 class="text-sm font-medium text-gray-700 dark:text-gray-300">
        最近事件（{recent.length}）
      </h2>
      <button
        class="px-2 py-0.5 text-xs bg-gray-200 dark:bg-gray-700 rounded hover:bg-gray-300"
        onclick={loadRecent}>手动刷新</button
      >
    </div>
    {#if recent.length === 0}
      <p class="text-gray-400 text-sm italic">
        还没有任何记录。点菜单栏的 "🎤 现在打卡" 试试。
      </p>
    {:else}
      <ul class="space-y-2">
        {#each recent as ev (ev.id)}
          {@const summary = structuredSummary(ev.structured)}
          <li class="text-sm p-3 bg-white dark:bg-gray-900 rounded border border-gray-200 dark:border-gray-700">
            <div class="flex items-center justify-between text-xs text-gray-400 mb-1">
              <span>
                <span class="inline-block px-1.5 py-0.5 mr-1 bg-gray-100 dark:bg-gray-800 rounded text-gray-600 dark:text-gray-400">
                  {ev.type}
                </span>
                {timeAgo(ev.ts)}
              </span>
              <span>#{ev.id}</span>
            </div>
            <div class="text-gray-800 dark:text-gray-200">
              {ev.raw_text ?? "(空)"}
            </div>
            {#if summary}
              <div class="mt-1.5 text-xs text-blue-600 dark:text-blue-400">
                🤖 {summary}
              </div>
            {:else if ev.type === "checkin"}
              <div class="mt-1.5 text-xs text-gray-400 italic">
                ⏳ 等待 LLM 提取…（确认设置页配好了 key？）
              </div>
            {/if}
            {#if ev.tags && ev.tags.length}
              <div class="mt-1.5 flex flex-wrap gap-1">
                {#each ev.tags as t}
                  <span class="px-1.5 py-0.5 text-xs bg-blue-100 dark:bg-blue-900 text-blue-700 dark:text-blue-300 rounded">
                    #{t}
                  </span>
                {/each}
              </div>
            {/if}
          </li>
        {/each}
      </ul>
    {/if}
  </section>
</div>
