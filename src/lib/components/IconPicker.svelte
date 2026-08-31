<script lang="ts">
  import Icon from "./Icon.svelte";

  let {
    value = "wallet",
    color = "#2563eb",
    onselect = (_: string) => {},
  }: { value?: string; color?: string; onselect?: (icon: string) => void } = $props();

  // Curated list of available icon names (must exist in Icon.svelte paths).
  export const ICON_OPTIONS: { name: string; label: string }[] = [
    { name: "wallet", label: "Dompet" },
    { name: "banknote", label: "Uang" },
    { name: "credit-card", label: "Kartu" },
    { name: "landmark", label: "Bank" },
    { name: "smartphone", label: "Ponsel" },
    { name: "piggy-bank", label: "Celengan" },
    { name: "utensils", label: "Makan" },
    { name: "bus", label: "Transport" },
    { name: "shopping-bag", label: "Belanja" },
    { name: "receipt", label: "Tagihan" },
    { name: "heart", label: "Kesehatan" },
    { name: "film", label: "Hiburan" },
    { name: "gift", label: "Hadiah" },
    { name: "trending-up", label: "Investasi" },
    { name: "arrow-left-right", label: "Transfer" },
    { name: "tag", label: "Label" },
    { name: "calendar", label: "Kalender" },
    { name: "clock", label: "Waktu" },
    { name: "layers", label: "Lapisan" },
    { name: "pie-chart", label: "Grafik" },
    { name: "sparkles", label: "Spesial" },
    { name: "shield", label: "Proteksi" },
    { name: "cloud", label: "Cloud" },
    { name: "database", label: "Data" },
  ];

  let open = $state(false);
  const current = $derived(ICON_OPTIONS.find((i) => i.name === value) ?? { name: value, label: value });

  function pick(name: string) {
    onselect(name);
    open = false;
  }

  function onBackdrop(e: MouseEvent) {
    if (!(e.target as HTMLElement).closest(".iconpicker")) open = false;
  }
</script>

<svelte:window onclick={onBackdrop} />

<div class="iconpicker">
  <button type="button" class="iconpicker-trigger" onclick={() => (open = !open)} aria-expanded={open}>
    <span class="iconpicker-preview" style="background:{color}1a;color:{color}">
      <Icon name={value} size={20} />
    </span>
    <span class="grow" style="text-align:left;font-weight:700">{current.label}</span>
    <Icon name="chevron-down" size={16} color="var(--muted)" />
  </button>

  {#if open}
    <div class="iconpicker-menu" role="listbox" aria-label="Pilih ikon">
      {#each ICON_OPTIONS as opt (opt.name)}
        <button
          type="button"
          class="iconpicker-option"
          class:selected={opt.name === value}
          role="option"
          aria-selected={opt.name === value}
          title={opt.label}
          onclick={() => pick(opt.name)}
        >
          <span class="iconpicker-tile" style="background:{color}1a;color:{color}">
            <Icon name={opt.name} size={18} />
          </span>
          <span class="iconpicker-label">{opt.label}</span>
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .iconpicker { position: relative; }
  .iconpicker-trigger {
    display: flex; align-items: center; gap: 10px; width: 100%;
    border: 1.5px solid var(--border); border-radius: 12px;
    padding: 9px 12px; background: var(--surface); cursor: pointer;
    transition: border-color var(--dur) var(--ease), box-shadow var(--dur) var(--ease);
  }
  .iconpicker-trigger:hover { border-color: var(--border-strong); }
  .iconpicker-trigger:focus-visible { outline: none; border-color: var(--brand); box-shadow: 0 0 0 4px rgba(37,99,235,0.12); }
  .iconpicker-preview {
    width: 36px; height: 36px; border-radius: 10px; flex-shrink: 0;
    display: flex; align-items: center; justify-content: center;
  }
  .iconpicker-menu {
    position: absolute; top: calc(100% + 6px); left: 0; right: 0; z-index: 30;
    background: var(--surface); border: 1px solid var(--border); border-radius: 14px;
    box-shadow: var(--shadow-md); padding: 8px;
    display: grid; grid-template-columns: repeat(4, 1fr); gap: 4px;
    max-height: 240px; overflow-y: auto;
    animation: pageIn 0.15s var(--ease);
  }
  .iconpicker-option {
    display: flex; flex-direction: column; align-items: center; gap: 5px;
    border: none; background: transparent; border-radius: 10px;
    padding: 8px 4px; cursor: pointer;
    transition: background var(--dur) var(--ease);
  }
  .iconpicker-option:hover { background: var(--surface-2); }
  .iconpicker-option.selected { background: var(--brand-soft); }
  .iconpicker-option.selected .iconpicker-label { color: var(--brand-strong); }
  .iconpicker-tile {
    width: 34px; height: 34px; border-radius: 10px;
    display: flex; align-items: center; justify-content: center;
  }
  .iconpicker-label { font-size: 10px; font-weight: 600; color: var(--muted); }
  @keyframes pageIn { from { opacity: 0; transform: translateY(-4px); } to { opacity: 1; transform: none; } }
</style>
