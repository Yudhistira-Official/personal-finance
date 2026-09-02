<script lang="ts">
  import { onMount } from "svelte";
  import { api, type MutualFundProduct } from "$lib/api";
  import Icon from "$lib/components/Icon.svelte";

  let {
    value = null,
    onselect = (_p: MutualFundProduct) => {},
  }: {
    value?: MutualFundProduct | null;
    onselect?: (p: MutualFundProduct) => void;
  } = $props();

  const FUND_TYPE_LABEL: Record<string, string> = {
    money_market: "Pasar Uang",
    fixed_income: "Pendapatan Tetap",
    balanced: "Campuran",
    equity: "Saham",
  };

  let query = $state("");
  let results: MutualFundProduct[] = $state([]);
  let searching = $state(false);
  let open = $state(false);
  let syncing = $state(false);
  let syncFailed = $state(false);
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;

  /** Return stable badge class for supported fund types, neutral for unknown values. */
  function typeClass(t: string): string {
    return ["money_market", "fixed_income", "balanced", "equity"].includes(t) ? `badge-${t}` : "badge-neutral";
  }

  function typeLabel(t: string): string {
    return FUND_TYPE_LABEL[t] ?? t;
  }

  function fmtNav(nav: number): string {
    return "Rp " + nav.toLocaleString("id-ID", { minimumFractionDigits: 2, maximumFractionDigits: 2 });
  }

  async function search(q: string) {
    searching = true;
    try {
      results = await api.search_mutual_funds(q);
    } catch {
      results = [];
    }
    searching = false;
  }

  function onInput() {
    if (debounceTimer) clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => search(query.trim()), 300);
  }

  function pick(p: MutualFundProduct) {
    onselect(p);
    query = "";
    results = [];
    open = false;
  }

  function clear() {
    query = "";
    results = [];
  }

  onMount(() => {
    // Tandai katalog sedang disiapkan agar dropdown menampilkan status sinkronisasi.
    syncing = true;
    syncFailed = false;
    api.sync_bibit_catalog()
      .catch(() => {
        // Kegagalan sync tidak mematikan pencarian karena fallback remote tetap aktif.
        syncFailed = true;
      })
      .finally(() => {
        syncing = false;
        // Ulangi pencarian terakhir agar hasil fresh selalu muncul tanpa race.
        search(query.trim());
      });
  });
</script>

{#if value}
  <div class="card card-pad-sm" style="display:flex;align-items:center;gap:12px">
    <div class="icon-tile" style="background:var(--brand-soft);color:var(--brand-strong)">
      <Icon name="trending-up" size={19} color="var(--brand-strong)" />
    </div>
    <div class="grow" style="min-width:0">
      <div class="h-sm" style="overflow:hidden;text-overflow:ellipsis;white-space:nowrap">{value.name}</div>
      <div class="text-muted num" style="font-size:11.5px;margin-top:2px">
        NAB {fmtNav(value.current_nav)} · {typeLabel(value.fund_type)}
      </div>
    </div>
    <button type="button" class="btn btn-ghost btn-sm" style="padding:8px" onclick={clear} aria-label="Hapus pilihan produk">
      <Icon name="x" size={15} />
    </button>
  </div>
{:else}
  <div style="position:relative">
    <input
      class="input"
      placeholder="Cari reksa dana (nama / manajer investasi)…"
      bind:value={query}
      oninput={onInput}
      onfocus={() => { open = true; search(query.trim()); }}
    />
    {#if open}
      <div
        class="card card-pad-sm"
        style="position:absolute;top:calc(100% + 8px);left:0;right:0;z-index:20;max-height:300px;overflow-y:auto;padding:8px;box-shadow:var(--shadow-md)"
      >
        {#if syncing}
          <div class="text-muted" style="padding:4px 8px 8px;font-size:11px">Menyiapkan katalog…</div>
        {/if}
        {#if searching}
          <div class="text-muted" style="padding:12px;font-size:13px;text-align:center">Mencari…</div>
        {:else if !results.length}
          <div class="text-muted" style="padding:12px;font-size:13px;text-align:center">
            Tidak ada produk ditemukan.
            {#if syncFailed && query.trim()}
              <div style="font-size:11px;margin-top:4px">Katalog belum tersinkron. Coba lagi saat ada internet.</div>
            {/if}
          </div>
        {:else}
          <div class="text-muted" style="padding:4px 8px 8px;font-size:11.5px;font-weight:600;letter-spacing:0.04em;text-transform:uppercase">
            {query.trim() ? "Hasil pencarian" : "5 rekomendasi teratas"}
          </div>
          <div class="stack" style="gap:8px">
            {#each results as p (p.id)}
              <button type="button" class="fund-option" onclick={() => pick(p)}>
                <div class="fund-option-main">
                  <div class="fund-option-name">{p.name}</div>
                  <div class="fund-option-badges">
                    <span class="badge fund-option-type {typeClass(p.fund_type)}">{typeLabel(p.fund_type)}</span>
                    {#if p.is_syariah}
                      <span class="badge badge-pos"><Icon name="shield" size={11} color="var(--pos-ink)" /> Syariah</span>
                    {/if}
                  </div>
                  <div class="fund-option-meta num">{p.manager_name} · NAB {fmtNav(p.current_nav)}</div>
                </div>
              </button>
            {/each}
          </div>
        {/if}
      </div>
    {/if}
  </div>
{/if}

<style>
  /* Dropdown result row: compact product identity with one-line fund metadata. */
  .fund-option {
    display: block; width: 100%; text-align: left;
    padding: 8px; border: 1px solid var(--border); border-radius: var(--radius-sm);
    background: var(--surface); color: inherit;
    transition: background var(--dur) var(--ease), border-color var(--dur) var(--ease);
  }
  .fund-option:hover { background: var(--surface-2); border-color: var(--border-strong); }
  .fund-option-main { min-width: 0; }
  .fund-option-name { font-size: 13.5px; font-weight: 600; line-height: 1.35; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .fund-option-badges { display: flex; align-items: center; gap: 6px; margin-top: 5px; flex-wrap: wrap; }
  .fund-option-type { font-size: 10px; padding: 2px 7px; }
  .fund-option-meta { font-size: 11px; color: var(--muted); margin-top: 5px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
</style>
