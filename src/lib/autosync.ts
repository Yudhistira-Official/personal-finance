import { api } from "./api";

/**
 * Auto-sync global: interval sinkronisasi yang hidup di LEVEL LAYOUT,
 * sehingga tetap berjalan saat user berpindah halaman.
 * Halaman Setelan hanya membaca/menulis state untuk UI toggle.
 */
export const autoSyncState = {
  enabled: false,
};

let syncInterval: ReturnType<typeof setInterval> | null = null;
const AUTO_SYNC_MS = 60_000;

async function runSilentSync() {
  try {
    await api.sync_push();
  } catch {
    // Senyap: gagal jaringan → antrean tetap pending, dicoba lagi tick berikutnya.
  }
}

function clearTimer() {
  if (syncInterval !== null) {
    clearInterval(syncInterval);
    syncInterval = null;
  }
}

/** Sinkronkan interval dengan nilai toggle (tanpa menyimpan ke backend). */
export function applyAutoSync(enabled: boolean): void {
  autoSyncState.enabled = enabled;
  clearTimer();
  if (enabled) {
    void runSilentSync();
    syncInterval = setInterval(runSilentSync, AUTO_SYNC_MS);
  }
}

/** Nyalakan auto-sync (simpan ke backend + mulai interval global). */
export async function enableAutoSync(savedUrl: string | null): Promise<void> {
  await api.settings_save(savedUrl, true);
  applyAutoSync(true);
}

/** Matikan auto-sync (simpan ke backend + hentikan interval global). */
export async function disableAutoSync(savedUrl: string | null): Promise<void> {
  await api.settings_save(savedUrl, false);
  applyAutoSync(false);
}

/** Sinkronkan interval dengan nilai tersimpan (dipanggil saat layout mount). */
export function startAutoSyncIfEnabled(): void {
  clearTimer();
  if (autoSyncState.enabled) {
    syncInterval = setInterval(runSilentSync, AUTO_SYNC_MS);
  }
}

/** Restore status dari backend lalu aktifkan interval bila perlu (layout onMount). */
export async function initAutoSync(): Promise<boolean> {
  try {
    const info = await api.sync_status();
    autoSyncState.enabled = info.auto_sync === true;
    startAutoSyncIfEnabled();
    return autoSyncState.enabled;
  } catch {
    return false;
  }
}
