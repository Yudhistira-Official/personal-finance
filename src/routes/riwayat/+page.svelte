<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { api, type Account, type Category, type Transaction } from "$lib/api";
  import { fmtIDR, fmtDate, fmtShortDate, todayBounds, weekBounds, monthBounds } from "$lib/utils";
  import Icon from "$lib/components/Icon.svelte";
  import Modal from "$lib/components/Modal.svelte";
  import Fab from "$lib/components/Fab.svelte";

  type RangeKey = "today" | "week" | "month" | "all";

  let txs: Transaction[] = $state([]);
  let accounts: Account[] = $state([]);
  let categories: Category[] = $state([]);
  let loading = $state(true);
  let err = $state("");
  let search = $state("");
  let range: RangeKey = $state("month");
  let fAccount = $state("");
  let fCategory = $state("");
  let fType = $state("");

  let expandedId: string | null = $state(null);

  // Collapsible filter panel visibility.
  let showFilters = $state(false);

  // edit modal state
  let showEdit = $state(false);
  let editing: Transaction | null = $state(null);
  let eAmount = $state("");
  let eNote = $state("");
  let eType = $state("expense");
  let eAccount = $state("");
  let eDest = $state("");
  let eCategory = $state("");
  let eDate = $state("");

  // True when any of the three filters has a value — drives the filter button's active dot.
  let hasActiveFilters = $derived(!!(fAccount || fCategory || fType));

  function rangeBounds(): { from?: number; to?: number } {
    if (range === "today") return todayBounds();
    if (range === "week") return weekBounds();
    if (range === "month") return monthBounds();
    return {};
  }

  async function load() {
    loading = true;
    err = "";
    try {
      const b = rangeBounds();
      const [t, a, c] = await Promise.all([
        api.transactions_list({
          search: search || null,
          account_id: fAccount || null,
          category_id: fCategory || null,
          tx_type: fType || null,
          from: b.from ?? null,
          to: b.to ?? null,
        }),
        api.accounts_list(),
        api.categories_list(),
      ]);
      txs = t;
      accounts = a;
      categories = c;
    } catch (e: any) {
      err = String(e);
    }
    loading = false;
  }

  function acctName(id: string) {
    return accounts.find((a) => a.id === id)?.name ?? "-";
  }
  function catById(id: string) {
    return categories.find((c) => c.id === id);
  }

  async function doDelete(id: string) {
    if (!confirm("Hapus transaksi ini?")) return;
    try {
      await api.transactions_delete(id);
      await load();
    } catch (e: any) {
      err = String(e);
    }
  }

  function openEdit(t: Transaction) {
    editing = t;
    eAmount = t.amount.toString();
    eNote = t.note;
    eType = t.transaction_type;
    eAccount = t.account_id;
    eDest = t.destination_account_id ?? "";
    eCategory = t.category_id;
    eDate = new Date(t.date * 1000).toISOString().slice(0, 16);
    showEdit = true;
  }

  async function submitEdit() {
    if (!editing) return;
    const amount = parseInt(eAmount.replace(/\D/g, ""), 10);
    try {
      await api.transactions_update(editing.id, {
        account_id: eAccount,
        destination_account_id: eType === "transfer" ? eDest : null,
        category_id: eCategory,
        amount,
        transaction_type: eType,
        date: Math.floor(new Date(eDate).getTime() / 1000),
        note: eNote,
      });
      showEdit = false;
      await load();
    } catch (e: any) {
      err = String(e);
    }
  }

  function doExport() {
    const b = rangeBounds();
    api
      .export_csv(b.from, b.to)
      .then((csv) => {
        const blob = new Blob(["\ufeff" + csv], { type: "text/csv;charset=utf-8" });
        const url = URL.createObjectURL(blob);
        const a = document.createElement("a");
        a.href = url;
        a.download = "transaksi.csv";
        a.click();
        URL.revokeObjectURL(url);
      })
      .catch((e) => (err = String(e)));
  }

  function onAmountInput(e: Event) {
    const raw = (e.target as HTMLInputElement).value.replace(/\D/g, "");
    eAmount = raw ? parseInt(raw, 10).toString() : "";
  }

  // Compute a human-friendly label for a transaction's date group.
  function dateGroupLabel(ts: number): string {
    const d = new Date(ts * 1000);
    const now = new Date();
    const startOfDay = (x: Date) => new Date(x.getFullYear(), x.getMonth(), x.getDate()).getTime();
    const diffDays = Math.round((startOfDay(now) - startOfDay(d)) / 86400000);
    if (diffDays === 0) return "Hari ini";
    if (diffDays === 1) return "Kemarin";
    return fmtShortDate(ts);
  }

  // Group transactions (already sorted date DESC) into ordered day buckets.
  let groups = $derived.by(() => {
    const out: { label: string; items: Transaction[] }[] = [];
    for (const t of txs) {
      const label = dateGroupLabel(t.date);
      const last = out[out.length - 1];
      // Transactions are contiguous by day since they arrive sorted desc.
      if (last && last.label === label) {
        last.items.push(t);
      } else {
        out.push({ label, items: [t] });
      }
    }
    return out;
  });

  onMount(load);
</script>

<div class="page">
  <div class="page-header">
    <div>
      <h1 class="page-title">Riwayat</h1>
      <p class="page-sub">Semua catatan transaksimu</p>
    </div>
    <button class="btn btn-secondary btn-sm" onclick={doExport} title="Ekspor CSV">
      <Icon name="download" size={15} /> CSV
    </button>
  </div>

  {#if err}
    <div class="card card-pad-sm" style="margin-bottom:12px;background:var(--neg-soft);border-color:transparent;display:flex;justify-content:space-between;align-items:center">
      <span class="text-neg" style="font-size:13px;font-weight:600">{err}</span>
      <button class="btn btn-ghost btn-sm" style="padding:4px 8px" onclick={() => (err = "")} aria-label="Tutup">
        <Icon name="x" size={15} />
      </button>
    </div>
  {/if}

  <!-- Search bar -->
  <div class="card card-pad-sm" style="margin-bottom:12px">
    <div class="row">
      <Icon name="search" size={17} color="var(--muted)" />
      <input
        class="input"
        style="border:none;padding:0;box-shadow:none;background:transparent"
        placeholder="Cari catatan…"
        bind:value={search}
        oninput={load}
      />
    </div>
  </div>

  <!-- Range chips -->
  <div style="display:flex;gap:8px;overflow-x:auto;margin-bottom:12px;padding-bottom:2px">
    {#each [["today", "Hari Ini"], ["week", "7 Hari"], ["month", "Bulan Ini"], ["all", "Semua"]] as [v, label]}
      <button
        class="chip"
        class:active={range === v}
        onclick={() => {
          range = v as RangeKey;
          load();
        }}
      >
        {label}
      </button>
    {/each}
  </div>

  <!-- Filter toggle -->
  <div style="margin-bottom:12px">
    <button class="btn btn-secondary btn-sm" style="position:relative" onclick={() => (showFilters = !showFilters)}>
      <Icon name="filter" size={15} /> Filter
      {#if hasActiveFilters}
        <span
          style="position:absolute;top:6px;right:6px;width:7px;height:7px;border-radius:50%;background:var(--brand)"
        ></span>
      {/if}
    </button>
  </div>

  {#if showFilters}
    <div class="card card-pad-sm" style="margin-bottom:12px">
      <div class="grid-2">
        <div>
          <label class="label">Akun</label>
          <select class="input" bind:value={fAccount} onchange={load}>
            <option value="">Semua Akun</option>
            {#each accounts as a}<option value={a.id}>{a.name}</option>{/each}
          </select>
        </div>
        <div>
          <label class="label">Kategori</label>
          <select class="input" bind:value={fCategory} onchange={load}>
            <option value="">Semua Kategori</option>
            {#each categories as c}<option value={c.id}>{c.name}</option>{/each}
          </select>
        </div>
        <div style="grid-column:1 / -1">
          <label class="label">Tipe</label>
          <select class="input" bind:value={fType} onchange={load}>
            <option value="">Semua Tipe</option>
            <option value="income">Masuk</option>
            <option value="expense">Keluar</option>
            <option value="transfer">Transfer</option>
          </select>
        </div>
      </div>
    </div>
  {/if}

  {#if loading}
    <div class="text-muted" style="text-align:center;padding:32px">Memuat…</div>
  {:else if !txs.length}
    <div class="empty">
      <div class="empty-icon"><Icon name="receipt" size={28} /></div>
      <div class="h-md" style="margin:0 0 4px;color:var(--ink)">Belum ada transaksi</div>
      <div style="font-size:13px;margin-bottom:16px">Catat transaksi pertamamu untuk melihatnya di sini.</div>
      <a class="btn btn-primary btn-sm" href="/">Tambah Transaksi</a>
    </div>
  {:else}
    <div class="stack">
      {#each groups as g}
        <div>
          <div class="text-muted" style="font-size:12px;font-weight:700;letter-spacing:0.02em;margin:0 0 8px 4px">{g.label}</div>
          <div class="card card-pad-sm fade-item">
            <div style="display:flex;flex-direction:column">
              {#each g.items as t, i (t.id)}
                {@const cat = catById(t.category_id)}
                {@const isIncome = t.transaction_type === "income"}
                {@const isExpense = t.transaction_type === "expense"}
                {@const isTransfer = t.transaction_type === "transfer"}
                {@const color = cat?.color_hex ?? "#2563eb"}
                {#if i > 0}<div class="divider-dash" style="margin-top:0;padding-top:0"></div>{/if}
                <button
                  class="tx-row"
                  style="padding:10px 0"
                  onclick={() => (expandedId = expandedId === t.id ? null : t.id)}
                >
                  <div class="icon-tile" style="background:{color}1f;color:{color}">
                    <Icon name={cat?.icon ?? "wallet"} size={19} />
                  </div>
                  <div class="grow">
                    <div class="h-sm" style="white-space:nowrap;overflow:hidden;text-overflow:ellipsis">
                      {cat?.name ?? "Tanpa kategori"}
                    </div>
                    <div class="text-muted" style="font-size:11.5px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis">
                      {isTransfer
                        ? `${acctName(t.account_id)} → ${acctName(t.destination_account_id ?? "")}`
                        : acctName(t.account_id)} · {fmtDate(t.date)}
                    </div>
                  </div>
                  <div class="num" style="font-weight:800;font-size:13.5px;white-space:nowrap;text-align:right">
                    {#if isIncome}
                      <span class="text-pos">+{fmtIDR(t.amount)}</span>
                    {:else if isExpense}
                      <span class="text-neg">-{fmtIDR(t.amount)}</span>
                    {:else}
                      <span class="text-muted" style="display:inline-flex;align-items:center;gap:4px">
                        <Icon name="arrow-left-right" size={13} />{fmtIDR(t.amount)}
                      </span>
                    {/if}
                  </div>
                  <span class="text-muted"><Icon name={expandedId === t.id ? "chevron-up" : "chevron-down"} size={15} /></span>
                </button>

                {#if expandedId === t.id}
                  <div class="divider-dash" style="padding-top:10px">
                    {#if t.note}
                      <div style="margin-bottom:10px">
                        <div class="label" style="margin-bottom:2px">Catatan</div>
                        <div style="font-size:13px;color:var(--ink-2)">{t.note}</div>
                      </div>
                    {/if}
                    <div class="row" style="flex-wrap:wrap;gap:6px;margin-bottom:12px">
                      {#if t.sync_status === "synced"}
                        <span class="badge badge-pos"><span class="badge-dot"></span> Synced</span>
                      {:else if t.sync_status === "pending"}
                        <span class="badge badge-warn"><span class="badge-dot"></span> Pending</span>
                      {:else}
                        <span class="badge badge-neg"><span class="badge-dot"></span> Failed</span>
                      {/if}
                      {#if t.sheet_row_id != null}
                        <span class="badge badge-neutral">Baris #{t.sheet_row_id}</span>
                      {/if}
                    </div>
                    <div class="row">
                      <button class="btn btn-secondary btn-sm grow" onclick={() => openEdit(t)}>
                        <Icon name="edit" size={14} /> Edit
                      </button>
                      <button class="btn btn-danger btn-sm grow" onclick={() => doDelete(t.id)}>
                        <Icon name="trash" size={14} /> Hapus
                      </button>
                    </div>
                  </div>
                {/if}
              {/each}
            </div>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

{#if !loading}
  <Fab label="Catat Transaksi" onclick={() => goto("/")} />
{/if}

<Modal title="Edit Transaksi" open={showEdit} onclose={() => (showEdit = false)}>
  {#if editing}
    <div class="stack">
      <div class="segmented">
        {#each [["income", "Masuk"], ["expense", "Keluar"], ["transfer", "Transfer"]] as [v, label]}
          <button
            class="seg-item"
            class:active={eType === v}
            class:seg-income={v === "income"}
            class:seg-expense={v === "expense"}
            class:seg-transfer={v === "transfer"}
            onclick={() => (eType = v as string)}
          >
            {label}
          </button>
        {/each}
      </div>

      <div>
        <label class="label">Nominal (IDR)</label>
        <input class="input input-amount" inputmode="numeric" value={eAmount} oninput={onAmountInput} />
      </div>

      <div class="grid-2">
        <div>
          <label class="label">Akun sumber</label>
          <select class="input" bind:value={eAccount}>
            {#each accounts as a}<option value={a.id}>{a.name}</option>{/each}
          </select>
        </div>
        {#if eType === "transfer"}
          <div>
            <label class="label">Akun tujuan</label>
            <select class="input" bind:value={eDest}>
              {#each accounts.filter((a) => a.id !== eAccount) as a}<option value={a.id}>{a.name}</option>{/each}
            </select>
          </div>
        {:else}
          <div>
            <label class="label">Kategori</label>
            <select class="input" bind:value={eCategory}>
              {#each categories.filter((c) => c.category_type === eType) as c}<option value={c.id}>{c.name}</option>{/each}
            </select>
          </div>
        {/if}
      </div>

      {#if eType === "transfer"}
        <div>
          <label class="label">Kategori</label>
          <select class="input" bind:value={eCategory}>
            {#each categories as c}<option value={c.id}>{c.name}</option>{/each}
          </select>
        </div>
      {/if}

      <div>
        <label class="label">Tanggal</label>
        <input class="input" type="datetime-local" bind:value={eDate} />
      </div>

      <div>
        <label class="label">Catatan</label>
        <input class="input" bind:value={eNote} />
      </div>

      <button class="btn btn-primary btn-block" onclick={submitEdit}>Simpan</button>
    </div>
  {/if}
</Modal>
