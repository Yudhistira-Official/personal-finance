export function fmtIDR(n: number): string {
  const sign = n < 0 ? "-" : "";
  const abs = Math.abs(n);
  return sign + "Rp " + abs.toString().replace(/\B(?=(\d{3})+(?!\d))/g, ".");
}

export function fmtDate(ts: number): string {
  const d = new Date(ts * 1000);
  if (isNaN(d.getTime())) return "-";
  return d.toLocaleString("id-ID", { day: "2-digit", month: "short", year: "numeric", hour: "2-digit", minute: "2-digit" });
}

export function fmtShortDate(ts: number): string {
  const d = new Date(ts * 1000);
  if (isNaN(d.getTime())) return "-";
  return d.toLocaleDateString("id-ID", { day: "2-digit", month: "short", year: "numeric" });
}

export function monthBounds(d = new Date()): { from: number; to: number } {
  const from = Math.floor(new Date(d.getFullYear(), d.getMonth(), 1).getTime() / 1000);
  const to = Math.floor(new Date(d.getFullYear(), d.getMonth() + 1, 0, 23, 59, 59).getTime() / 1000);
  return { from, to };
}

export function weekBounds(d = new Date()): { from: number; to: number } {
  const day = d.getDay() === 0 ? 7 : d.getDay();
  const from = new Date(d); from.setDate(d.getDate() - day + 1); from.setHours(0, 0, 0, 0);
  const to = new Date(from); to.setDate(from.getDate() + 6); to.setHours(23, 59, 59, 999);
  return { from: Math.floor(from.getTime() / 1000), to: Math.floor(to.getTime() / 1000) };
}

export function todayBounds(): { from: number; to: number } {
  const d = new Date(); const s = new Date(d.getFullYear(), d.getMonth(), d.getDate(), 0, 0, 0); const e = new Date(d.getFullYear(), d.getMonth(), d.getDate(), 23, 59, 59);
  return { from: Math.floor(s.getTime() / 1000), to: Math.floor(e.getTime() / 1000) };
}

export function parseIDR(s: string): number {
  const cleaned = s.replace(/[^0-9]/g, "");
  return cleaned === "" ? 0 : parseInt(cleaned, 10);
}

export function formatInputIDR(n: number): string {
  if (!n) return "";
  return n.toString().replace(/\B(?=(\d{3})+(?!\d))/g, ".");
}
