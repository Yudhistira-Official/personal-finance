<script lang="ts">
  import { onMount } from "svelte";
  import { api, type Account, type Category, type CategorySpend, type DashboardSummary, type SavingsPocket } from "$lib/api";
  import { fmtIDR, fmtShortDate, monthBounds } from "$lib/utils";
  import Icon from "$lib/components/Icon.svelte";
  import Progress from "$lib/components/Progress.svelte";
  import Modal from "$lib/components/Modal.svelte";
  import Fab from "$lib/components/Fab.svelte";

  let summary: DashboardSummary | null = $state(null);
  let catSpending: CategorySpend[] = $state([]);
  let pockets: SavingsPocket[] = $state([]);
  let accounts: Account[] = $state([]);
  let categories: Category[] = $state([]);
  let syncInfo: any = $state({ status: "synced", pending_count: 0 });
  let loading = $state(true);
  let err = $state("");

  let showTx = $state(false);
  let txType: "income" | "expense" | "transfer" = $state("expense");
  let fAmount = $state("");
  let fNote = $state("");
  let fAccount = $state("");
  let fDestAccount = $state("");
  let fCategory = $state("");
  let fDate = $state(new Date().toISOString().slice(0, 16));

  const totalSpend = $derived(catSpending.reduce((s, c) => s + c.amount, 0));

  const donutSegments = $derived.by(() => {
    if (!totalSpend) return [] as { cat: CategorySpend; pct: number; offset: number }[];
    let off = 0;
    return catSpending.slice(0, 6).map((cat) => {
      const pct = cat.amount / totalSpend;
      const seg = { cat, pct, offset: off };
      off += pct * 100;
      return seg;
    });
  });

  function cashflowMax() {
    return summary ? Math.max(summary.total_income, summary.total_expense, 1) : 1;
  }

  function greeting() {
    const h = new Date().getHours();
    if (h < 11) return "Selamat pagi";
    if (h < 15) return "Selamat siang";
    if (h < 19) return "Selamat sore";
    return "Selamat malam";
  }

  async function loadData() {
    loading = true; err = "";
    try {
      const { from, to } = monthBounds();
      const [s, cs, pkts, accts, cats, sync] = await Promise.all([
        api.dashboard_summary(from, to),
        api.expense_by_category(from, to),
        api.pockets_list(),
        api.accounts_list(),
        api.categories_list(),
        api.sync_status(),
      ]);
      summary = s; catSpending = cs; pockets = pkts; accounts = accts; categories = cats; syncInfo = sync;
      if (accts.length) {
        if (!fAccount) fAccount = accts[0].id;
        if (!fDestAccount && accts.length > 1) fDestAccount = accts[1].id;
      }
      if (cats.length && !fCategory) {
        const def = cats.find((c: Category) => c.category_type === txType);
        if (def) fCategory = def.id;
      }
    } catch (e: any) { err = String(e); }
    loading = false;
  }

  function filteredCats(typ: string) { return categories.filter((c: Category) => c.category_type === typ); }

  async function submitTx() {
    const amount = parseInt(fAmount.replace(/\./g, ""), 10);
    if (!amount || !fAccount || !fCategory) { err = "Lengkapi nominal, akun, dan kategori."; return; }
    if (txType === "transfer" && (!fDestAccount || fDestAccount === fAccount)) { err = "Pilih akun tujuan yang berbeda."; return; }
    try {
      await api.transactions_create({
        account_id: fAccount,
        destination_account_id: txType === "transfer" ? fDestAccount : null,
        category_id: fCategory,
        amount,
        transaction_type: txType,
        date: Math.floor(new Date(fDate).getTime() / 1000),
        note: fNote,
      });
      showTx = false; fAmount = ""; fNote = ""; err = "";
      await loadData();
    } catch (e: any) { err = String(e); }
  }

  function handleTxTypeChange() {
    const list = filteredCats(txType);
    if (list.length) fCategory = list[0].id;
  }

  function onAmountInput(e: Event) {
    const raw = (e.target as HTMLInputElement).value.replace(/\D/g, "");
    if (!raw) { fAmount = ""; return; }
    fAmount = parseInt(raw, 10).toString().replace(/\B(?=(\d{3})+(?!\d))/g, ".");
  }

  onMount(loadData);
</script>

<div class="page">
  {#if err}
    <div class="card card-pad-sm" style="background:var(--neg-soft);border-color:#f8c2cd;color:var(--neg-ink);display:flex;justify-content:space-between;align-items:center;margin-bottom:14px;font-size:13px;font-weight:600">
      <span>{err}</span>
      <button onclick={() => (err = "")} style="background:none;border:none;color:inherit;font-size:16px;line-height:1;cursor:pointer">✕</button>
    </div>
  {/if}

  {#if loading}
    <div class="card" style="text-align:center;color:var(--muted);padding:48px;font-weight:600">Memuat dashboard…</div>
  {:else}
    <!-- Greeting -->
    <div class="page-header" style="margin-bottom:16px">
      <div>
        <h1 class="page-title">{greeting()} 👋</h1>
        <p class="page-sub">Ini ringkasan keuanganmu bulan ini</p>
      </div>
      {#if syncInfo.pending_count > 0}
        <span class="badge badge-warn"><span class="badge-dot"></span>{syncInfo.pending_count} antre</span>
      {:else}
        <span class="badge badge-pos"><span class="badge-dot"></span>Synced</span>
      {/if}
    </div>

    <!-- Net worth hero -->
    <div class="card card-hero">
      <div class="hero-ring"></div>
      <div style="position:relative">
        <div style="display:flex;align-items:center;gap:8px;font-size:11px;font-weight:800;letter-spacing:0.1em;text-transform:uppercase;opacity:0.85">
          <Icon name="wallet" size={14} color="#fff" /> Total Kekayaan Bersih
        </div>
        <div class="num" style="font-size:34px;font-weight:800;letter-spacing:-0.02em;margin-top:10px;line-height:1.1">{fmtIDR(summary?.net_worth ?? 0)}</div>
        <div class="row" style="margin-top:16px;gap:8px;flex-wrap:wrap">
          <span class="hero-chip">
            <Icon name="piggy-bank" size={13} color="#fff" /> Tabungan {fmtIDR(summary?.total_savings ?? 0)}
          </span>
          <span class="hero-chip">
            <Icon name={summary && summary.net_cashflow >= 0 ? "trending-up" : "trending-down"} size={13} color="#fff" />
            Arus kas {fmtIDR(summary?.net_cashflow ?? 0)}
          </span>
        </div>
      </div>
    </div>

    <!-- Cashflow -->
    <div class="grid-2" style="margin-top:14px">
      <div class="card card-pad-sm">
        <div class="row" style="gap:9px;margin-bottom:8px">
          <div class="icon-tile" style="width:34px;height:34px;border-radius:11px;background:var(--pos-soft);color:var(--pos-ink)"><Icon name="arrow-down-left" size={16} /></div>
          <div class="label" style="margin:0">Pemasukan</div>
        </div>
        <div class="h-md num" style="color:var(--pos-ink)">{fmtIDR(summary?.total_income ?? 0)}</div>
        <div class="progress-track" style="height:6px;margin-top:10px;background:var(--pos-soft)">
          <div class="progress-fill" style="height:100%;width:{summary ? (summary.total_income / cashflowMax() * 100) : 0}%;background:var(--pos)"></div>
        </div>
      </div>
      <div class="card card-pad-sm">
        <div class="row" style="gap:9px;margin-bottom:8px">
          <div class="icon-tile" style="width:34px;height:34px;border-radius:11px;background:var(--neg-soft);color:var(--neg-ink)"><Icon name="arrow-up-right" size={16} /></div>
          <div class="label" style="margin:0">Pengeluaran</div>
        </div>
        <div class="h-md num" style="color:var(--neg-ink)">{fmtIDR(summary?.total_expense ?? 0)}</div>
        <div class="progress-track" style="height:6px;margin-top:10px;background:var(--neg-soft)">
          <div class="progress-fill" style="height:100%;width:{summary ? (summary.total_expense / cashflowMax() * 100) : 0}%;background:var(--neg)"></div>
        </div>
      </div>
    </div>

    <!-- Spending by category -->
    <div class="card" style="margin-top:14px">
      <div class="row" style="justify-content:space-between;margin-bottom:16px">
        <div class="h-sm">Pengeluaran per Kategori</div>
        <span class="badge badge-neutral">Bulan ini</span>
      </div>
      {#if !catSpending.length}
        <div class="empty" style="padding:20px 10px">
          <div class="empty-icon" style="width:52px;height:52px"><Icon name="pie-chart" size={24} /></div>
          <div class="h-sm" style="color:var(--ink)">Belum ada pengeluaran</div>
          <div style="font-size:12.5px;margin-top:4px">Catat transaksi untuk melihat grafiknya.</div>
        </div>
      {:else}
        <div class="row" style="gap:18px;align-items:center">
          <div style="position:relative;width:130px;height:130px;flex-shrink:0">
            <svg width="130" height="130" viewBox="0 0 100 100">
              {#each donutSegments as seg}
                {@const r = 40}
                {@const circ = 2 * Math.PI * r}
                {@const len = seg.pct * circ}
                {@const rot = seg.offset / 100 * 360 - 90}
                <circle cx="50" cy="50" r={r} fill="none" stroke={seg.cat.color_hex} stroke-width="13" stroke-dasharray="{len} {circ - len}" transform="rotate({rot} 50 50)" stroke-linecap="round" />
              {/each}
              <circle cx="50" cy="50" r="30" fill="var(--surface)" />
            </svg>
            <div style="position:absolute;inset:0;display:flex;flex-direction:column;align-items:center;justify-content:center;pointer-events:none">
              <div style="font-size:9.5px;color:var(--muted);font-weight:800;letter-spacing:0.08em">TOTAL</div>
              <div class="num" style="font-size:11px;font-weight:800">{fmtIDR(totalSpend)}</div>
            </div>
          </div>
          <div class="grow stack" style="gap:9px">
            {#each catSpending.slice(0, 5) as c}
              <div class="row" style="gap:9px;font-size:12.5px">
                <span style="width:9px;height:9px;border-radius:50%;background:{c.color_hex};flex-shrink:0"></span>
                <span class="grow" style="font-weight:600;white-space:nowrap;overflow:hidden;text-overflow:ellipsis">{c.category_name}</span>
                <span class="num" style="font-weight:800;white-space:nowrap">{fmtIDR(c.amount)}</span>
              </div>
            {/each}
          </div>
        </div>
      {/if}
    </div>

    <!-- Savings snapshot -->
    <div class="card" style="margin-top:14px">
      <div class="row" style="justify-content:space-between;margin-bottom:14px">
        <div class="h-sm">Pos Tabungan</div>
        <a href="/tabungan" style="font-size:12.5px;color:var(--brand);text-decoration:none;font-weight:700;display:inline-flex;align-items:center;gap:3px">Lihat semua <Icon name="chevron-right" size={13} /></a>
      </div>
      {#if !pockets.length}
        <div class="empty" style="padding:16px 10px">
          <div style="font-size:13px;color:var(--muted)">Belum ada pos tabungan. <a href="/tabungan" style="color:var(--brand);font-weight:700;text-decoration:none">Buat sekarang</a></div>
        </div>
      {:else}
        <div class="stack" style="gap:14px">
          {#each pockets.slice(0, 3) as p}
            {@const pct = p.target_amount > 0 ? Math.round((p.current_amount / p.target_amount) * 100) : 0}
            <div>
              <div class="row" style="justify-content:space-between;margin-bottom:7px;font-size:12.5px">
                <span class="row" style="gap:8px;font-weight:700">
                  <span style="width:9px;height:9px;border-radius:50%;background:{p.color_tag};display:inline-block"></span>{p.name}
                </span>
                <span class="badge" style="background:{p.color_tag}1a;color:{p.color_tag}">{pct}%</span>
              </div>
              <Progress progress={pct} color={p.color_tag} height={8} />
              <div class="row" style="justify-content:space-between;font-size:11px;color:var(--muted);margin-top:5px">
                <span class="num">{fmtIDR(p.current_amount)} / {fmtIDR(p.target_amount)}</span>
                {#if p.target_date}<span class="row" style="gap:4px"><Icon name="calendar" size={11} /> {fmtShortDate(p.target_date)}</span>{/if}
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </div>

  {/if}
</div>

{#if !loading}
  <Fab label="Catat Transaksi" onclick={() => (showTx = true)} />
{/if}

<!-- Quick add transaction modal -->
<Modal title="Catat Transaksi" open={showTx} onclose={() => (showTx = false)}>
  <div class="stack" style="gap:14px">
    <div class="segmented">
      {#each [["income", "Masuk", "arrow-down-left"], ["expense", "Keluar", "arrow-up-right"], ["transfer", "Transfer", "arrow-left-right"]] as [v, label, ic]}
        <button class="seg-item seg-{v}" class:active={txType === v} onclick={() => { txType = v as any; handleTxTypeChange(); }}>
          <Icon name={ic as string} size={14} /> {label}
        </button>
      {/each}
    </div>

    <div>
      <label class="label" for="tx-amount">Nominal</label>
      <div style="position:relative">
        <span style="position:absolute;left:14px;top:50%;transform:translateY(-50%);font-weight:800;color:var(--muted);font-size:15px">Rp</span>
        <input id="tx-amount" class="input input-amount" style="padding-left:44px" inputmode="numeric" placeholder="0" value={fAmount} oninput={onAmountInput} />
      </div>
    </div>

    <div class="grid-2">
      <div>
        <label class="label" for="tx-acct">Akun sumber</label>
        <select id="tx-acct" class="input" bind:value={fAccount}>
          {#each accounts as a}<option value={a.id}>{a.name}</option>{/each}
        </select>
      </div>
      {#if txType === "transfer"}
        <div>
          <label class="label" for="tx-dest">Akun tujuan</label>
          <select id="tx-dest" class="input" bind:value={fDestAccount}>
            {#each accounts.filter((a) => a.id !== fAccount) as a}<option value={a.id}>{a.name}</option>{/each}
          </select>
        </div>
      {:else}
        <div>
          <label class="label" for="tx-cat">Kategori</label>
          <select id="tx-cat" class="input" bind:value={fCategory}>
            {#each filteredCats(txType) as c}<option value={c.id}>{c.name}</option>{/each}
          </select>
        </div>
      {/if}
    </div>
    {#if txType === "transfer"}
      <div>
        <label class="label" for="tx-cat2">Kategori</label>
        <select id="tx-cat2" class="input" bind:value={fCategory}>
          {#each categories as c}<option value={c.id}>{c.name}</option>{/each}
        </select>
      </div>
    {/if}

    <div>
      <label class="label" for="tx-date">Tanggal</label>
      <input id="tx-date" class="input" type="datetime-local" bind:value={fDate} />
    </div>

    <div>
      <label class="label" for="tx-note">Catatan <span class="text-muted" style="font-weight:500">(opsional)</span></label>
      <input id="tx-note" class="input" placeholder="mis. Makan siang di warteg" bind:value={fNote} />
    </div>

    <button class="btn btn-primary btn-block btn-lg" onclick={submitTx}>Simpan Transaksi</button>
  </div>
</Modal>
