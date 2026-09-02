import { getVersion } from "@tauri-apps/api/app";

/** GitHub Releases endpoint of this app's repository. */
const RELEASES_LATEST_URL =
  "https://api.github.com/repos/Yudhistira-Official/personal-finance/releases/latest";

/** Result of a cross-platform update check (works on Android and desktop). */
export interface UpdateInfo {
  /** Latest published version, e.g. "0.2.0" (leading "v" stripped). */
  latest: string;
  /** Locally installed version, e.g. "0.1.0". */
  current: string;
  /** True when `latest` is strictly newer than `current`. */
  updateAvailable: boolean;
  /** Browser URL of the latest release page ("" when unknown). */
  url: string;
}

/**
 * Check for a newer app version via the GitHub Releases API.
 *
 * Cross-platform replacement for `@tauri-apps/plugin-updater`, which only
 * works on desktop. Never installs anything: callers open `url` in the
 * browser so the user downloads the update themselves.
 *
 * @returns Update info; on any failure `updateAvailable` is false so the
 *          check stays silent and never blocks normal app usage.
 */
export async function checkForUpdate(): Promise<UpdateInfo> {
  const current = await getVersion();
  try {
    const res = await fetch(RELEASES_LATEST_URL, {
      headers: { Accept: "application/vnd.github+json" },
    });
    if (!res.ok) return { latest: current, current, updateAvailable: false, url: "" };

    // tag_name is "vX.Y.Z"; html_url points at the release page for download.
    const data = await res.json();
    const latest = data.tag_name?.replace(/^v/, "") ?? current;
    const url = data.html_url ?? "https://github.com/Yudhistira-Official/personal-finance/releases/latest";
    return { latest, current, updateAvailable: compareVersions(latest, current) > 0, url };
  } catch {
    // Network/API failure: report "no update" silently.
    return { latest: current, current, updateAvailable: false, url: "" };
  }
}

/**
 * Compare dotted numeric versions ("0.2.1" vs "0.10.0").
 *
 * @returns 1 if a > b, -1 if a < b, 0 when equal. Missing segments count as 0.
 */
function compareVersions(a: string, b: string): number {
  const pa = a.split(".").map(Number);
  const pb = b.split(".").map(Number);
  for (let i = 0; i < Math.max(pa.length, pb.length); i++) {
    const x = pa[i] ?? 0;
    const y = pb[i] ?? 0;
    if (x > y) return 1;
    if (x < y) return -1;
  }
  return 0;
}
