<script lang="ts">
  let { progress = 0, color = "#2563eb", height = 8 }: { progress?: number; color?: string; height?: number } = $props();
  const pct = $derived(Math.max(0, Math.min(100, progress)));
</script>

<div class="progress-track" style="height:{height}px;width:100%">
  <div class="progress-fill" style="height:100%;width:{pct}%;background:{color}"></div>
</div>
