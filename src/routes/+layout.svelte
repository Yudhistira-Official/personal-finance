<script lang="ts">
  import { onMount } from "svelte";
  import { page } from "$app/stores";
  import "../app.css";
  import Icon from "$lib/components/Icon.svelte";
  import { initAutoSync } from "$lib/autosync";

  const tabs = [
    { id: "riwayat", label: "Riwayat", path: "/riwayat", icon: "clock" },
    { id: "tabungan", label: "Tabungan", path: "/tabungan", icon: "piggy-bank" },
    { id: "dompet", label: "Dompet", path: "/dompet", icon: "wallet" },
    { id: "dashboard", label: "Beranda", path: "/", icon: "home", center: true },
    { id: "portofolio", label: "Portofolio", path: "/portofolio", icon: "trending-up" },
    { id: "hutang", label: "Hutang", path: "/hutang", icon: "arrow-left-right" },
    { id: "setelan", label: "Setelan", path: "/setelan", icon: "settings" },
  ];
  $: current = $page.url.pathname;
  $: isActive = (p: string) => (p === "/" ? current === "/" : current.startsWith(p));

  onMount(() => {
    void initAutoSync();
  });
</script>

<div class="app-root">
  <main class="app-main">
    <slot />
  </main>
  <nav class="bottom-nav" aria-label="Navigasi utama">
    {#each tabs as t}
      {@const active = isActive(t.path)}
      {#if t.center}
        <a
          href={t.path}
          aria-label={t.label}
          aria-current={active ? "page" : undefined}
          class="center-btn"
        >
          <Icon name={t.icon} size={24} color="#fff" />
        </a>
      {:else}
        <a
          href={t.path}
          aria-current={active ? "page" : undefined}
          class="nav-tab"
          class:active
        >
          <Icon name={t.icon} size={20} color={active ? "var(--brand)" : "var(--muted)"} />
          <span class="nav-label">{t.label}</span>
        </a>
      {/if}
    {/each}
  </nav>
</div>

<style>
  .app-root { display:flex; flex-direction:column; min-height:100dvh; }
  .app-main { flex:1; overflow:auto; padding-bottom:96px; }

  .bottom-nav {
    position:fixed; bottom:0; left:50%; transform:translateX(-50%);
    width:100%; max-width:460px;
    display:flex; align-items:stretch; justify-content:space-around;
    background:rgba(255,255,255,0.9);
    backdrop-filter:blur(16px);
    -webkit-backdrop-filter:blur(16px);
    border-top:1px solid var(--border);
    padding:8px 8px calc(10px + env(safe-area-inset-bottom));
    z-index:40;
  }

  .nav-tab {
    flex:1; min-width:0; display:flex; flex-direction:column; align-items:center; justify-content:center;
    gap:3px; text-decoration:none; color:var(--muted);
    font-size:9px; font-weight:700; padding:6px 2px; border-radius:12px;
    transition:color var(--dur) var(--ease);
  }
  .nav-tab:active { transform:scale(0.94); }
  .nav-tab.active { color:var(--brand); }
  .nav-label {
    line-height:1; letter-spacing:0; text-align:center;
    max-width:100%; overflow:hidden; text-overflow:ellipsis; white-space:nowrap;
  }

  .center-btn {
    width:62px; height:62px; border-radius:22px; flex-shrink:0;
    background:var(--brand); color:#fff;
    display:flex; align-items:center; justify-content:center;
    text-decoration:none; margin-top:-32px;
    box-shadow:0 10px 24px -8px rgba(37,99,235,0.45);
    border:5px solid var(--bg);
    transition:transform 0.15s var(--ease), box-shadow var(--dur) var(--ease);
  }
  .center-btn:active { transform:scale(0.92); }
</style>
