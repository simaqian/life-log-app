<script lang="ts">
  import Checkin from "./routes/Checkin.svelte";
  import Home from "./routes/Home.svelte";
  import Settings from "./routes/Settings.svelte";

  // 极简 hash 路由
  let hash = $state(window.location.hash || "#/");

  $effect(() => {
    const onChange = () => (hash = window.location.hash || "#/");
    window.addEventListener("hashchange", onChange);
    return () => window.removeEventListener("hashchange", onChange);
  });

  const route = $derived(hash.replace(/^#\//, ""));
</script>

{#if route === "checkin"}
  <Checkin />
{:else if route === "settings"}
  <Settings />
{:else}
  <Home />
{/if}
