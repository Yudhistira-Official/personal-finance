import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

/**
 * Check for, download, and install a newer app version from GitHub Releases.
 *
 * @returns The newly installed version string, or `null` when no update is
 *          available (or the check fails silently).
 */
export async function checkForUpdate(): Promise<string | null> {
  try {
    const update = await check();
    if (update?.available) {
      // Download + install the update, then restart the app to apply it.
      await update.downloadAndInstall();
      await relaunch();
      return update.version;
    }
    return null;
  } catch {
    // Fail silently: an update check must never block normal app usage.
    return null;
  }
}
