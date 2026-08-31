<script lang="ts">
  import Icon from "./Icon.svelte";
  import type { Snippet } from "svelte";
  let { title = "", open = false, onclose = () => {}, children }: { title?: string; open?: boolean; onclose?: () => void; children?: Snippet } = $props();
</script>

{#if open}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="modal-backdrop" onclick={(e) => { if (e.target === e.currentTarget) onclose(); }}>
    <div class="modal-sheet" role="dialog" aria-modal="true" aria-label={title || "Dialog"}>
      <div class="sheet-handle"></div>
      {#if title}
        <div class="row" style="justify-content:space-between;margin-bottom:18px">
          <h3 class="h-md" style="margin:0">{title}</h3>
          <button class="btn btn-ghost btn-sm" style="padding:8px;border-radius:12px" onclick={onclose} aria-label="Tutup">
            <Icon name="x" size={17} />
          </button>
        </div>
      {/if}
      {#if children}{@render children()}{/if}
    </div>
  </div>
{/if}
