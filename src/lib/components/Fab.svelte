<script lang="ts">
  import Icon from "./Icon.svelte";

  let {
    label = "Tambah",
    onclick = () => {},
  }: { label?: string; onclick?: () => void } = $props();
</script>

<button class="fab" aria-label={label} title={label} onclick={onclick}>
  <Icon name="plus" size={26} color="#fff" />
</button>

<style>
  .fab {
    position: fixed;
    right: 20px;
    bottom: 92px;
    width: 58px;
    height: 58px;
    border-radius: 50%;
    border: none;
    background: var(--brand);
    color: #fff;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    box-shadow: 0 12px 26px -8px rgba(37, 99, 235, 0.55);
    z-index: 45;
    transition: transform 0.15s var(--ease), box-shadow var(--dur) var(--ease), background var(--dur) var(--ease);
  }
  .fab:hover { background: var(--brand-strong); box-shadow: 0 14px 30px -8px rgba(37, 99, 235, 0.6); }
  .fab:active { transform: scale(0.9); }

  /* keep it inside the 460px app column on wide screens */
  @media (min-width: 520px) {
    .fab {
      right: calc(50% - 230px + 20px);
    }
  }
</style>
