<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { api, type Account, type MutualFundProduct, type PortfolioHolding, type PortfolioSnapshot } from "$lib/api";
  import { fmtIDR } from "$lib/utils";
  import Icon from "$lib/components/Icon.svelte";
  import Modal from "$lib/components/Modal.svelte";
  import MutualFundPicker from "$lib/components/MutualFundPicker.svelte";

  let accounts: Account[] = $state([]);
  let loading = $state(true);
  let err = $state("");

  let holdings: PortfolioHolding[] = $state([]);
  let snapshots: PortfolioSnapshot[] = $state([]);
  let refreshingNav = $state(false);
  let refreshUnlockTimer: ReturnType<typeof setTimeout> | null = null;

  let showInvestForm = $state(false);
  let investMode = $state<"BUY" | "SELL">("BUY");
  let investHolding = $state<PortfolioHolding | null>(null);
  let investProduct = $state<MutualFundProduct | null>(null);
  let investAccount = $state("");
  let investAmountMode: "rupiah" | "unit" = $state("rupiah");
  let investAmount = $state("");
  let investUnits = $state("");
  let investFee = $state("");
  let investDate = $state("");
  let investNote = $state("");
  let investSaving = $state(false);

  const FUND_TYPE_LABEL: Record<string, string> = {
    money_market: "Pasar Uang",
    fixed_income: "Pendapatan Tetap",
    balanced: "Campuran",
    equity: "Saham",
  };

  /** Return stable badge class for supported fund types, neutral for unknown values. */
  function fundTypeClass(t: string): string {
    return ["money_market", "fixed_income", "balanced", "equity"].includes(t) ? `badge-${t}` : "badge-neutral";
  }

  function fundTypeLabel(t: string): string {
    return FUND_TYPE_LABEL[t] ?? t;
  }

  function fmtNav(nav: number): string {
    return "Rp " + nav.toLocaleString("id-ID", { minimumFractionDigits: 2, maximumFractionDigits: 2 });
  }

  function fmtUnits(units: number): string {
    return units.toLocaleString("id-ID", { maximumFractionDigits: 4 });
  }

  function fmtDeltaIDR(n: number): string {
    const sign = n > 0 ? "+" : n < 0 ? "-" : "";
    return sign + fmtIDR(Math.abs(n));
  }

  /** Load holdings, snapshots, and accounts; record today's snapshot silently. */
  async function load() {
    loading = true;
    err = "";
    try {
      [holdings, snapshots, accounts] = await Promise.all([
        api.get_portfolio_holdings(),
        api.get_portfolio_snapshots(30),
        api.accounts_list(),
      ]);
      // Snapshot harian: rekam nilai hari ini senyap, lalu refresh 30 hari terakhir.
      try {
        await api.record_daily_snapshot();
        snapshots = await api.get_portfolio_snapshots(30);
      } catch {
        snapshots = [];
      }
    } catch (e: any) {
      err = String(e);
    }
    loading = false;
  }

  /** Sparkline SVG 7 hari terakhir dari total_value; kosong bila data kurang. */
  function sparklinePoints(points: PortfolioSnapshot[]): string {
    if (points.length < 2) return "";
    const width = 120;
    const height = 40;
    const padding = 4;
    const values = points.map((p) => p.total_value);
    const min = Math.min(...values);
    const max = Math.max(...values);
    const span = max - min || 1;
    return points
      .map((p, index) => {
        const x = padding + (index / (points.length - 1)) * (width - padding * 2);
        const y = height - padding - ((p.total_value - min) / span) * (height - padding * 2);
        return `${x},${y}`;
      })
      .join(" ");
  }

  const snapshotsAsc = $derived([...snapshots].sort((a, b) => a.day - b.day));
  const todaySnapshot = $derived(snapshots.length ? snapshots[0] : null);
  const yesterdaySnapshot = $derived(snapshots.length > 1 ? snapshots[1] : null);
  const sparklineSnapshots = $derived(snapshotsAsc.slice(-7));
  const sparklinePointsValue = $derived(sparklinePoints(sparklineSnapshots));

  /** Derived totals from holdings so hero stays consistent with the list. */
  const totalValue = $derived(holdings.reduce((sum, h) => sum + h.current_value, 0));
  const totalInvested = $derived(holdings.reduce((sum, h) => sum + h.total_invested, 0));
  const totalPnl = $derived(holdings.reduce((sum, h) => sum + h.unrealized_pnl, 0));
  const roiPct = $derived(totalInvested ? (totalPnl / totalInvested) * 100 : 0);

  function toLocalInputValue(d: Date): string {
    const pad = (n: number) => String(n).padStart(2, "0");
    return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
  }

  /** Refresh owned-product NAVs; unlocks button after 10s even if IPC hangs. */
  async function refreshNav() {
    if (refreshingNav) return;
    refreshingNav = true;
    // Visual safety net: backend request timeout is 12s, normal batch finishes 2-5s.
    const unlockTimer = setTimeout(() => {
      refreshingNav = false;
    }, 10_000);
    refreshUnlockTimer = unlockTimer;
    try {
      await api.refresh_portfolio_nav();
      await load();
    } catch (e) {
      err = `Gagal refresh NAB: ${String(e)}`;
    } finally {
      if (refreshUnlockTimer === unlockTimer) {
        clearTimeout(unlockTimer);
        refreshUnlockTimer = null;
        refreshingNav = false;
      }
    }
  }

  /** Buka modal transaksi investasi (mode beli produk baru atau beli/jual dari holding). */
  function openInvest(mode: "BUY" | "SELL", holding: PortfolioHolding | null = null) {
    investMode = mode;
    investHolding = holding;
    investProduct = null;
    investAccount = accounts[0]?.id ?? "";
    investAmountMode = "rupiah";
    investAmount = "";
    investUnits = "";
    investFee = "";
    investDate = toLocalInputValue(new Date());
    investNote = "";
    showInvestForm = true;
  }

  /** NAV aktif: produk pilihan saat beli, NAB holding saat jual. */
  const investNav = $derived(
    investMode === "SELL"
      ? (investHolding !== null ? investHolding.current_nav : 0)
      : (investProduct !== null ? investProduct.current_nav : 0)
  );

  const investAmountNum = $derived(parseInt(investAmount || "0", 10));
  const investUnitsNum = $derived(parseFloat((investUnits || "0").replace(",", ".")) || 0);

  /** Estimasi unit/nominal otomatis berdasarkan mode input aktif. */
  const investEstUnits = $derived(investNav > 0 && investAmountNum > 0 ? investAmountNum / investNav : 0);
  const investEstTotal = $derived(investUnitsNum > 0 && investNav > 0 ? Math.round(investUnitsNum * investNav) : 0);

  function onInvestAmountInput(event: Event) {
    investAmount = (event.target as HTMLInputElement).value.replace(/\D/g, "");
  }

  function onInvestFeeInput(event: Event) {
    investFee = (event.target as HTMLInputElement).value.replace(/\D/g, "");
  }

  /** Catat transaksi beli/jual reksa dana lalu muat ulang saldo & holdings. */
  async function submitInvest() {
    const fee = parseInt(investFee || "0", 10);
    const date = Math.floor(new Date(investDate).getTime() / 1000);
    let units = 0;
    let total = 0;

    if (!investAccount) {
      err = "Pilih akun kas terlebih dahulu.";
      return;
    }
    if (!investDate || isNaN(date)) {
      err = "Tanggal transaksi tidak valid.";
      return;
    }

    if (investMode === "BUY") {
      if (!investProduct) {
        err = "Pilih produk reksa dana terlebih dahulu.";
        return;
      }
      if (investAmountMode === "rupiah") {
        if (investAmountNum <= 0) {
          err = "Nominal pembelian harus lebih dari nol.";
          return;
        }
        units = investEstUnits;
        total = investAmountNum;
      } else {
        if (investUnitsNum <= 0) {
          err = "Jumlah unit harus lebih dari nol.";
          return;
        }
        units = investUnitsNum;
        total = investEstTotal;
      }
    } else {
      if (!investHolding) return;
      if (investUnitsNum <= 0) {
        err = "Jumlah unit jual harus lebih dari nol.";
        return;
      }
      if (investUnitsNum > investHolding.total_units + 1e-9) {
        err = `Unit melebihi kepemilikan (maks ${fmtUnits(investHolding.total_units)} unit).`;
        return;
      }
      units = investUnitsNum;
      total = Math.round(units * investHolding.current_nav);
    }

    const productId = investMode === "BUY" ? investProduct!.id : investHolding!.product_id;

    investSaving = true;
    try {
      await api.record_investment_tx({
        id: crypto.randomUUID(),
        product_id: productId,
        account_id: investAccount,
        tx_type: investMode,
        units,
        nav_per_unit: investNav,
        total_amount: total,
        fee,
        date,
        note: investNote.trim() || null,
      });
      showInvestForm = false;
      await load();
    } catch (e: any) {
      err = String(e);
    }
    investSaving = false;
  }

  onMount(load);
  onDestroy(() => {
    if (refreshUnlockTimer) clearTimeout(refreshUnlockTimer);
  });
</script>

<div class="page">
  <div class="page-header">
    <div>
      <h1 class="page-title">Portofolio</h1>
      <a class="back-link" href="/dompet">
        <Icon name="arrow-left" size={13} color="var(--muted)" /> Kembali ke Dompet
      </a>
    </div>
    <button class="btn btn-primary btn-sm" onclick={() => openInvest("BUY")}>
      <Icon name="plus" size={15} color="#fff" /> Beli
    </button>
  </div>

  {#if err}
    <div class="card card-pad-sm" style="margin-bottom:14px;background:var(--neg-soft);border-color:#fecdd3">
      <div class="row">
        <Icon name="alert" size={18} color="var(--neg-ink)" />
        <span class="grow" style="font-size:13px;color:var(--neg-ink)">{err}</span>
        <button class="btn btn-ghost btn-sm" style="padding:6px" onclick={() => err = ""} aria-label="Tutup pesan error">
          <Icon name="x" size={16} />
        </button>
      </div>
    </div>
  {/if}

  {#if loading}
    <div class="center" style="padding:60px 0"><span class="text-muted">Memuat…</span></div>
  {:else}
    <div class="stack">
      <div class="card card-hero portfolio-hero">
        <div class="hero-ring"></div>
        <div class="portfolio-hero-top">
          <div class="portfolio-label"><Icon name="trending-up" size={14} color="#fff" /> Total Nilai Portofolio</div>
          <button class="hero-refresh btn btn-sm" onclick={refreshNav} disabled={refreshingNav}>
            <Icon name="refresh" size={13} /> {refreshingNav ? "Memuat…" : "Refresh NAB"}
          </button>
        </div>
        <div class="num portfolio-total">{fmtIDR(totalValue)}</div>
        <div class="portfolio-pnl">
          Floating PnL {fmtDeltaIDR(totalPnl)} ({roiPct >= 0 ? "+" : ""}{roiPct.toFixed(2)}%)
        </div>
      </div>

      {#if holdings.length}
        {#if todaySnapshot}
          <div class="card card-pad-sm accumulation-card">
            <div class="row" style="justify-content:space-between;margin-bottom:8px">
              <span class="text-muted accumulation-label">Akumulasi harian</span>
              {#if yesterdaySnapshot}
                {@const delta = todaySnapshot.total_value - yesterdaySnapshot.total_value}
                <span class="pill" class:pill-pos={delta >= 0} class:pill-neg={delta < 0}>{fmtDeltaIDR(delta)} hari ini</span>
              {:else}
                <span class="pill pill-neutral">Belum ada pembanding</span>
              {/if}
            </div>
            <div class="row" style="justify-content:space-between;align-items:flex-end;gap:8px">
              <div style="flex:1;min-width:0">
                <div class="num h-sm">{fmtIDR(todaySnapshot.total_value)}</div>
                <div class="text-muted accumulation-note">
                  Pertumbuhan harian sejak modal awal {fmtIDR(todaySnapshot.total_invested)}
                </div>
              </div>
              {#if sparklinePointsValue}
                {@const lastPoint = sparklinePointsValue.split(" ").pop()?.split(",")}
                <svg class="portfolio-sparkline" width="120" height="40" viewBox="0 0 120 40" aria-label="Grafik nilai portofolio 7 hari terakhir" role="img">
                  <polygon points={`4,36 ${sparklinePointsValue} 116,36`} fill="rgba(37,99,235,0.08)" />
                  <polyline fill="none" stroke="var(--brand)" stroke-width="2" stroke-linejoin="round" stroke-linecap="round" points={sparklinePointsValue} />
                  {#if lastPoint}<circle cx={lastPoint[0]} cy={lastPoint[1]} r="3" fill="var(--brand)" />{/if}
                </svg>
              {/if}
            </div>
          </div>
        {/if}

        <div class="portfolio-heading">
          <h2 class="h-md" style="margin:0">Holdings</h2>
          <span class="text-muted" style="font-size:12px">{holdings.length} produk</span>
        </div>

        <div class="stack" style="gap:10px">
          {#each holdings as holding (holding.product_id)}
            <div class="holding-card">
              <div class="holding-header">
                <span class="icon-tile holding-icon"><Icon name="pie-chart" size={18} color="var(--brand)" /></span>
                <div class="grow holding-title">
                  <div class="h-sm holding-name">{holding.product_name || "Produk tidak tersedia"}</div>
                  <div class="holding-badges">
                    <span class="badge {fundTypeClass(holding.fund_type)}">{fundTypeLabel(holding.fund_type)}</span>
                    {#if (holding as PortfolioHolding & { is_syariah?: boolean }).is_syariah}<span class="badge badge-pos"><Icon name="shield" size={11} color="var(--pos-ink)" /> Syariah</span>{/if}
                  </div>
                </div>
              </div>
              <div class="hold-metrics">
                <div class="hold-metric"><span class="hold-metric-label"><Icon name="layers" size={11} /> Unit</span><span class="hold-metric-value num">{fmtUnits(holding.total_units)}</span></div>
                <div class="hold-metric"><span class="hold-metric-label"><Icon name="calendar" size={11} /> Avg NAB</span><span class="hold-metric-value num">{fmtNav(holding.avg_buy_nav)}</span></div>
                <div class="hold-metric"><span class="hold-metric-label"><Icon name="trending-up" size={11} /> NAB Kini</span><span class="hold-metric-value num">{fmtNav(holding.current_nav)}</span></div>
              </div>
              <div class="holding-summary">
                <div class="hold-metric"><span class="hold-metric-label"><Icon name="wallet" size={11} /> Nilai</span><span class="num h-md holding-value">{fmtIDR(holding.current_value)}</span></div>
                <div class="hold-metric"><span class="hold-metric-label">PnL</span><span class="num holding-pnl" class:text-pos={holding.unrealized_pnl >= 0} class:text-neg={holding.unrealized_pnl < 0}><Icon name={holding.unrealized_pnl >= 0 ? "trending-up" : "trending-down"} size={12} /> {fmtDeltaIDR(holding.unrealized_pnl)}</span></div>
                <div class="hold-metric holding-roi"><span class="hold-metric-label">ROI</span><span class="pill" class:pill-pos={holding.roi_percentage >= 0} class:pill-neg={holding.roi_percentage < 0}>{holding.roi_percentage >= 0 ? "+" : ""}{holding.roi_percentage.toFixed(2)}%</span></div>
              </div>
              <div class="hold-actions">
                <button class="btn btn-soft btn-sm" onclick={() => openInvest("BUY", holding)}><Icon name="plus" size={13} /> Beli Lagi</button>
                <button class="btn btn-secondary btn-sm" onclick={() => openInvest("SELL", holding)}><Icon name="minus" size={13} /> Jual</button>
              </div>
            </div>
          {/each}
        </div>
      {:else}
        <div class="card card-pad-sm portfolio-empty">
          <span class="icon-tile portfolio-empty-icon"><Icon name="pie-chart" size={26} color="var(--brand)" /></span>
          <div class="portfolio-empty-title">Belum ada investasi</div>
          <div class="portfolio-empty-sub">Mulai investasimu — pilih reksa dana di atas dan masukkan akun kas.</div>
          <button class="btn btn-primary btn-sm" onclick={() => openInvest("BUY")}><Icon name="plus" size={14} color="#fff" /> Pilih Reksa Dana</button>
        </div>
      {/if}
    </div>
  {/if}
</div>

<Modal title={investMode === "BUY" ? "Beli Reksa Dana" : "Jual Reksa Dana"} open={showInvestForm} onclose={() => showInvestForm = false}>
  <div class="stack" style="gap:14px">
    {#if investMode === "SELL" && investHolding}
      <div class="card card-pad-sm" style="background:var(--surface-2);box-shadow:none">
        <div class="h-sm">{investHolding.product_name || "Produk tidak tersedia"}</div>
        <div class="text-muted num" style="font-size:12px;margin-top:3px">
          Maks {fmtUnits(investHolding.total_units)} unit · NAB kini {fmtNav(investHolding.current_nav)}
        </div>
      </div>
    {:else}
      <div>
        <label class="label">Produk Reksa Dana</label>
        <MutualFundPicker value={investProduct} onselect={(p) => investProduct = p} />
      </div>
    {/if}

    <div>
      <label class="label" for="invest-account">Akun Kas</label>
      <select id="invest-account" class="input" bind:value={investAccount}>
        <option value="" disabled>Pilih akun kas</option>
        {#each accounts as account}<option value={account.id}>{account.name} — {fmtIDR(account.current_balance)}</option>{/each}
      </select>
    </div>

    {#if investMode === "BUY"}
      <div>
        <label class="label">Mode Input</label>
        <div class="segmented">
          <button type="button" class="seg-item" class:active={investAmountMode === "rupiah"} onclick={() => investAmountMode = "rupiah"}>Rupiah</button>
          <button type="button" class="seg-item" class:active={investAmountMode === "unit"} onclick={() => investAmountMode = "unit"}>Unit</button>
        </div>
      </div>

      {#if investAmountMode === "rupiah"}
        <div>
          <label class="label" for="invest-amount">Nominal Beli (IDR)</label>
          <input id="invest-amount" class="input input-amount" inputmode="numeric" placeholder="0" value={investAmount ? parseInt(investAmount, 10).toLocaleString("id-ID") : ""} oninput={onInvestAmountInput} />
          {#if investProduct && investAmountNum > 0}
            <p class="text-muted num" style="font-size:11.5px;margin:8px 0 0">≈ {fmtUnits(investEstUnits)} unit @ {fmtNav(investNav)}</p>
          {/if}
        </div>
      {:else}
        <div>
          <label class="label" for="invest-units">Jumlah Unit</label>
          <input id="invest-units" class="input input-amount" inputmode="decimal" placeholder="0" bind:value={investUnits} />
          {#if investProduct && investUnitsNum > 0}
            <p class="text-muted num" style="font-size:11.5px;margin:8px 0 0">≈ {fmtIDR(investEstTotal)} @ {fmtNav(investNav)}</p>
          {/if}
        </div>
      {/if}
    {:else}
      <div>
        <label class="label" for="invest-units-sell">Jumlah Unit Jual</label>
        <input id="invest-units-sell" class="input input-amount" inputmode="decimal" placeholder="0" bind:value={investUnits} />
        {#if investHolding && investUnitsNum > 0}
          <p class="text-muted num" style="font-size:11.5px;margin:8px 0 0">≈ {fmtIDR(Math.round(investUnitsNum * investHolding.current_nav))} @ {fmtNav(investHolding.current_nav)}</p>
        {/if}
      </div>
    {/if}

    <div>
      <label class="label" for="invest-fee">Fee <span class="text-muted">(opsional, IDR)</span></label>
      <input id="invest-fee" class="input" inputmode="numeric" placeholder="0" value={investFee ? parseInt(investFee, 10).toLocaleString("id-ID") : ""} oninput={onInvestFeeInput} />
    </div>

    <div>
      <label class="label" for="invest-date">Tanggal</label>
      <input id="invest-date" type="datetime-local" class="input" bind:value={investDate} />
    </div>

    <div>
      <label class="label" for="invest-note">Catatan <span class="text-muted">(opsional)</span></label>
      <input id="invest-note" class="input" placeholder="mis. Nabung rutin bulanan" bind:value={investNote} />
    </div>

    <button class="btn btn-primary btn-block" onclick={submitInvest} disabled={investSaving}>
      <Icon name="check" size={15} color="#fff" /> {investSaving ? "Menyimpan…" : investMode === "BUY" ? "Catat Pembelian" : "Catat Penjualan"}
    </button>
  </div>
</Modal>

<style>
  .back-link {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 12px;
    font-weight: 600;
    color: var(--muted);
    text-decoration: none;
    margin-top: 3px;
  }
  .back-link:hover { color: var(--brand); }

  /* Portfolio hero keeps total and PnL visually grouped. */
  .portfolio-hero { padding: 18px 18px 20px; }
  .portfolio-hero-top { position: relative; display: flex; align-items: flex-start; justify-content: space-between; gap: 10px; }
  .portfolio-label { display: inline-flex; align-items: center; gap: 8px; font-size: 11px; font-weight: 800; letter-spacing: 0.1em; text-transform: uppercase; opacity: 0.85; }
  .hero-refresh {
    position: relative; z-index: 1; flex: none;
    border: 1px solid rgba(255, 255, 255, 0.45); background: rgba(255, 255, 255, 0.12);
    color: #fff; font-size: 11.5px; font-weight: 700; padding: 6px 10px;
  }
  .hero-refresh:hover { background: rgba(255, 255, 255, 0.2); }
  .hero-refresh:disabled { opacity: 0.65; }
  .portfolio-total { font-size: 32px; font-weight: 800; letter-spacing: -0.02em; line-height: 1.1; margin-top: 12px; }
  .portfolio-pnl { margin-top: 10px; font-size: 12.5px; font-weight: 700; opacity: 0.92; }
  .portfolio-heading { display: flex; align-items: center; justify-content: space-between; margin: 0 2px; }

  /* Daily accumulation card with compact trend sparkline. */
  .accumulation-card { background: var(--surface-2); }
  .accumulation-label { font-size: 11.5px; font-weight: 600; letter-spacing: 0.04em; text-transform: uppercase; }
  .accumulation-note { font-size: 11.5px; margin-top: 2px; }
  .portfolio-sparkline { flex: none; }

  /* Holding card hierarchy: identity, metrics, valuation summary, actions. */
  .holding-card { background: var(--surface); border: 1px solid var(--border); border-radius: var(--radius); padding: 14px; box-shadow: var(--shadow-sm); }
  .holding-header { display: flex; align-items: flex-start; gap: 10px; }
  .holding-icon { width: 38px; height: 38px; background: var(--brand-soft); color: var(--brand); }
  .holding-title { min-width: 0; }
  .holding-name { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .holding-badges { display: flex; align-items: center; gap: 6px; margin-top: 5px; flex-wrap: wrap; }
  .holding-summary { display: grid; grid-template-columns: 1.25fr 1fr auto; gap: 10px; align-items: end; margin-top: 12px; padding-top: 10px; border-top: 1px solid var(--border); }
  .holding-value { color: var(--ink); }
  .holding-pnl { display: inline-flex; align-items: center; gap: 4px; font-size: 13px; font-weight: 700; }
  .holding-roi { justify-self: end; text-align: right; }

  /* Metric grid 3 kolom: Unit | Avg NAB | NAB Kini. */
  .hold-metrics {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 12px 10px;
    margin-top: 14px;
  }
  .hold-metric { display: flex; flex-direction: column; gap: 4px; min-width: 0; }
  .hold-metric-label {
    display: inline-flex; align-items: center; gap: 5px;
    font-size: 11px; color: var(--muted);
  }
  .hold-metric-value { font-size: 13px; font-weight: 700; color: var(--ink); }

  /* ROI pill for quick sentiment. */
  .pill {
    display: inline-flex; align-items: center;
    padding: 3px 9px; border-radius: 999px;
    font-size: 11.5px; font-weight: 700;
  }
  .pill-pos { background: var(--pos-soft); color: var(--pos-ink); }
  .pill-neg { background: var(--neg-soft); color: var(--neg-ink); }
  .pill-neutral { background: var(--surface); color: var(--muted); border: 1px solid var(--border); }

  /* Per-holding actions stay compact instead of stretching across the card. */
  .hold-actions {
    display: flex; justify-content: flex-start; gap: 8px; margin-top: 14px;
    padding-top: 12px; border-top: 1px dashed var(--border-strong);
  }

  /* Empty portfolio state points directly to the first investment action. */
  .portfolio-empty { display: flex; flex-direction: column; align-items: center; text-align: center; gap: 8px; padding: 24px 16px; }
  .portfolio-empty-icon { width: 58px; height: 58px; border-radius: 18px; background: var(--brand-soft); color: var(--brand); margin-bottom: 4px; }
  .portfolio-empty-title { font-size: 16px; font-weight: 700; color: var(--ink); }
  .portfolio-empty-sub { font-size: 12.5px; color: var(--muted); max-width: 32ch; margin-bottom: 6px; }
</style>
