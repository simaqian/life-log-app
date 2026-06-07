<script lang="ts">
  import { api } from "../lib/api";

  /** 保存成功后返回主页面（改 hash 路由） */
  function goHome() {
    window.location.hash = "#/";
  }

  let text = $state("");
  let saving = $state(false);
  let errMsg = $state<string | null>(null);

  // 当前时间显示
  let now = $state(new Date());
  $effect(() => {
    const t = setInterval(() => (now = new Date()), 30_000);
    return () => clearInterval(t);
  });

  const hhmm = $derived(
    now.toLocaleTimeString("zh-CN", { hour: "2-digit", minute: "2-digit", hour12: false })
  );

  async function save() {
    if (!text.trim() && !saving) {
      await skip();
      return;
    }
    saving = true;
    errMsg = null;
    const rawText = text.trim();
    try {
      // 1) 立即入库，拿到 id
      const id = await api.createEvent({
        type: "checkin",
        raw_text: rawText,
        source: "manual",
      });
      text = "";
      // 2) 回主页
      goHome();
      // 3) 后台异步调 LLM 提取结构化
      //    错误不弹窗（用户已经看不到这个窗口了），只 console
      api.llmExtractCheckin(rawText)
        .then((res) =>
          api.updateEventStructured(
            id,
            res.structured,
            res.tags,
            res.provider,
            res.model
          )
        )
        .catch((e) => console.warn("LLM 提取失败：", e));
    } catch (e) {
      errMsg = String(e);
    } finally {
      saving = false;
    }
  }

  async function skip() {
    goHome();
  }

  // Enter 保存，Esc 跳过
  function onKey(e: KeyboardEvent) {
    if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
      e.preventDefault();
      save();
    } else if (e.key === "Escape") {
      e.preventDefault();
      skip();
    }
  }
</script>

<svelte:window onkeydown={onKey} />

<div class="h-screen flex flex-col p-4 bg-white dark:bg-gray-900">
  <header class="flex items-center justify-between mb-3">
    <div class="text-sm">
      <span class="font-medium">{hhmm}</span>
      <span class="text-gray-500 ml-2">你这小时在干嘛？</span>
    </div>
    <button
      class="text-gray-400 hover:text-gray-600 text-xs"
      onclick={skip}>×</button
    >
  </header>

  <textarea
    bind:value={text}
    placeholder="说点什么…（⌘↩ 保存，Esc 跳过）"
    class="flex-1 w-full p-2 text-sm border border-gray-200 dark:border-gray-700 rounded bg-gray-50 dark:bg-gray-800 resize-none focus:outline-none focus:border-blue-400"
    autofocus
  ></textarea>

  {#if errMsg}
    <p class="text-red-600 text-xs mt-2">{errMsg}</p>
  {/if}

  <footer class="flex items-center justify-between mt-3">
    <div class="text-xs text-gray-400">
      🎤 语音输入即将支持
    </div>
    <div class="flex gap-2">
      <button
        class="px-3 py-1 text-sm text-gray-600 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800 rounded"
        onclick={skip}>跳过</button
      >
      <button
        class="px-3 py-1 text-sm bg-blue-500 text-white rounded hover:bg-blue-600 disabled:opacity-50"
        disabled={saving}
        onclick={save}>{saving ? "保存中…" : "✓ 保存"}</button
      >
    </div>
  </footer>
</div>
