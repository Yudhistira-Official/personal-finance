<script lang="ts">
  import { onMount } from "svelte";
  import { api, type Account, type Obligation, type ObligationSummary } from "$lib/api";
  import { fmtIDR, fmtShortDate, parseIDR } from "$lib/utils";
  import Icon from "$lib/components/Icon.svelte";
  import Modal from "$lib/components/Modal.svelte";
  import Progress from "$lib/components/Progress.svelte";
  import Fab from "$lib/components/Fab.svelte";

  let obligations: Obligation[] = $state([]);
  let summary: ObligationSummary = $state({ total_debt: 0, total_receivable: 0, overdue_count: 0 });
  let accounts: Account[] = $state([]);
  let loading = $state(true);
  let err = $state("");
  let filter = $state<"ALL" | "DEBT" | "RECEIVABLE">("ALL");

  let showForm = $state(false);
  let editing: Obligation | null = $state(null);
  let direction = $state<"DEBT" | "RECEIVABLE">("DEBT");
  let counterparty = $state("");
  let title = $state("");
  let originalAmount = $state("");
  let remainingAmount = $state("");
  let dueDate = $state("");
  let note = $state("");
  let saving = $state(false);

  let showPay = $state(false);
  let payTarget: Obligation | null = $state(null);
  let payAmount = $state("");
  let payAccount = $state("");
  let payDate = $state("");
  let paySaving = $state(false);

  /** Load obligations, summary, and cash accounts. */
  async function load() {
    loading = true;
    err = "";
    try {
      [obligations, summary, accounts] = await Promise.all([
        api.obligations_list(),
        api.obligations_summary(),
        api.accounts_list(),
      ]);
    } catch (e: any) {
      err = String(e);
    }
    loading = false;
  }

  const visible = $derived(
    obligations
      .filter((o) => filter === "ALL" || o.direction === filter)
      .sort((a, b) => {
        if (a.status !== b.status) return a.status === "OPEN" ? -1 : 1;
        return (a.due_date ?? Infinity) - (b.due_date ?? Infinity);
      })
  );

  function toLocalInputValue(d: Date): string {
    const pad = (n: number) => String(n).padStart(2, "0");
    return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
  }

  function openCreate() {
    editing = null;
    direction = "DEBT";
    counterparty = "";
    title = "";
    originalAmount = "";
    remainingAmount = "";
    dueDate = "";
    note = "";
    showForm = true;
  }

  function openEdit(o: Obligation) {
    editing = o;
    direction = o.direction;
    counterparty = o.counterparty;
    title = o.title;
    originalAmount = String(o.original_amount);
    remainingAmount = String(o.remaining_amount);
    dueDate = o.due_date ? toLocalInputValue(new Date(o.due_date * 1000)) : "";
    note = o.note ?? "";
    showForm = true;
  }

  function openPay(o: Obligation) {
    payTarget = o;
    payAmount = String(o.remaining_amount);
    payAccount = "";
    payDate = toLocalInputValue(new Date());
    showPay = true;
  }

  function parseDate(v: string): number | null {
    if (!v) return null;
    const t = new Date(v).getTime();
    return isNaN(t) ? null : Math.floor(t / 1000);
  }

  /** Create or update one obligation. */
  async function submit() {
    const orig = parseIDR(originalAmount);
    const remain = remainingAmount ? parseIDR(remainingAmount) : null;
    if (!counterparty.trim() || !title.trim() || orig <= 0) {
      err = "Wajib diisi: pihak, judul, nominal asli (> 0).";
      return;
    }
    const input = {
      direction,
      counterparty: counterparty.trim(),
      title: title.trim(),
      original_amount: orig,
      remaining_amount: remain,
      due_date: parseDate(dueDate),
      note: note.trim() || null,
    };
    saving = true;
    try {
      if (editing) await api.obligation_update(editing.id, input);
      else await api.obligation_create(input);
      showForm = false;
      await load();
    } catch (e: any) {
      err = String(e);
    }
    saving = false;
  }

  /** Record a payment/receipt against the selected obligation. */
  async function submitPay() {
    if (!payTarget) return;
    const amount = parseIDR(payAmount);
    if (amount <= 0 || amount > payTarget.remaining_amount) {
      err = `Nominal bayar harus 1–${payTarget.remaining_amount}.`;
      return;
    }
    paySaving = true;
    try {
      await api.obligation_pay({
        obligation_id: payTarget.id,
        amount,
        account_id: payAccount || null,
        date: parseDate(payDate),
      });
      showPay = false;
      await load();
    } catch (e: any) {
      err = String(e);
    }
    paySaving = false;
  }

  async function remove(o: Obligation) {
    if (!confirm(`Hapus "${o.title}"?`)) return;
    try {
      await api.obligation_delete(o.id);
      await load();
    } catch (e: any) {
      err = String(e);
    }
  }

  function isOverdue(o: Obligation): boolean {
    return o.status === "OPEN" && !!o.due_date && o.due_date < Math.floor(Date.now() / 1000);
  }

  function paidPct(o: Obligation): number {
    return o.original_amount ? ((o.original_amount - o.remaining_amount) / o.original_amount) * 100 : 0;
  }

  onMount(load);
</script>

<div class="page">
  <div class="page-header">
    <div>
      <h1 class="page-title">Hutang &amp; Piutang</h1>
      <p class="page-sub">Catat kewajiban dan tagihan, lacak pelunasannya.</p>
    </div>
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
      <div class="card card-hero">
        <div class="hero-ring"></div>
        <div class="obligation-summary">
          <div class="obligation-col">
            <div class="obligation-label"><Icon name="arrow-up-right" size={13} color="#fff" /> Total Hutang</div>
            <div class="num obligation-total">{fmtIDR(summary.total_debt)}</div>
          </div>
          <div class="obligation-col">
            <div class="obligation-label"><Icon name="arrow-down-left" size={13} color="#fff" /> Total Piutang</div>
            <div class="num obligation-total">{fmtIDR(summary.total_receivable)}</div>
          </div>
        </div>
        {#if summary.overdue_count > 0}
          <div class="obligation-overdue"><Icon name="alert" size={13} /> {summary.overdue_count} jatuh tempo</div>
        {/if}
      </div>

      <div class="segmented">
        <button type="button" class="seg-item" class:active={filter === "ALL"} onclick={() => filter = "ALL"}>Semua</button>
        <button type="button" class="seg-item" class:active={filter === "DEBT"} onclick={() => filter = "DEBT"}>Hutang</button>
        <button type="button" class="seg-item" class:active={filter === "RECEIVABLE"} onclick={() => filter = "RECEIVABLE"}>Piutang</button>
      </div>

      {#if visible.length}
        <div class="stack" style="gap:10px">
          {#each visible as o (o.id)}
            {@const debt = o.direction === "DEBT"}
            <div class="obligation-card">
              <div class="row" style="align-items:flex-start;gap:10px">
                <span class="icon-tile obligation-icon" class:obligation-icon-debt={debt} class:obligation-icon-receivable={!debt}>
                  <Icon name={debt ? "arrow-up-right" : "arrow-down-left"} size={18} color={debt ? "var(--neg-ink)" : "var(--pos-ink)"} />
                </span>
                <div class="grow">
                  <div class="h-sm obligation-title">{o.title}</div>
                  <div class="obligation-meta">{o.counterparty}{#if o.due_date}<span class="badge obligation-due" class:obligation-due-overdue={isOverdue(o)}>{isOverdue(o) ? "Jatuh tempo " : ""}{fmtShortDate(o.due_date)}</span>{/if}</div>
                  {#if o.note}<div class="obligation-note">{o.note}</div>{/if}
                </div>
                {#if o.status === "DONE"}<span class="badge badge-pos">Lunas</span>{/if}
              </div>
              <div class="obligation-progress">
                <Progress progress={paidPct(o)} color={debt ? "var(--brand)" : "var(--pos)"} height={6} />
                <div class="obligation-amounts num"><span>{fmtIDR(o.remaining_amount)}</span><span class="obligation-original">dari {fmtIDR(o.original_amount)}</span></div>
              </div>
              <div class="obligation-actions">
                {#if o.status === "OPEN"}
                  <button class="btn btn-soft btn-sm" onclick={() => openPay(o)}><Icon name="check" size={13} /> {debt ? "Bayar" : "Terima"}</button>
                {/if}
                <button class="btn btn-ghost btn-sm" onclick={() => openEdit(o)}>Edit</button>
                <button class="btn btn-ghost btn-sm" onclick={() => remove(o)} aria-label="Hapus"><Icon name="trash" size={13} /></button>
              </div>
            </div>
          {/each}
        </div>
      {:else}
        <div class="card card-pad-sm obligation-empty">
          <span class="icon-tile obligation-empty-icon"><Icon name="arrow-left-right" size={26} color="var(--brand)" /></span>
          <div class="obligation-empty-title">Belum ada catatan</div>
          <div class="obligation-empty-sub">Catat hutang atau piutang agar jatuh tempo tidak terlewat.</div>
          <button class="btn btn-primary btn-sm" onclick={openCreate}><Icon name="plus" size={14} color="#fff" /> Catat Hutang / Piutang</button>
        </div>
      {/if}
    </div>
  {/if}
</div>

<Fab label="Catat" onclick={openCreate} />

<Modal title={editing ? "Edit Catatan" : "Catat Hutang / Piutang"} open={showForm} onclose={() => showForm = false}>
  <div class="stack" style="gap:14px">
    <div>
      <label class="label">Arah</label>
      <div class="segmented">
        <button type="button" class="seg-item" class:active={direction === "DEBT"} onclick={() => direction = "DEBT"}>Hutang</button>
        <button type="button" class="seg-item" class:active={direction === "RECEIVABLE"} onclick={() => direction = "RECEIVABLE"}>Piutang</button>
      </div>
    </div>
    <div><label class="label" for="obligation-counterparty">Pihak</label><input id="obligation-counterparty" class="input" placeholder="Nama orang / toko" bind:value={counterparty} /></div>
    <div><label class="label" for="obligation-title">Judul</label><input id="obligation-title" class="input" placeholder="mis. Pinjaman darurat" bind:value={title} /></div>
    <div><label class="label" for="obligation-original">Nominal Asli (IDR)</label><input id="obligation-original" class="input input-amount" inputmode="numeric" placeholder="0" value={originalAmount ? parseIDR(originalAmount).toLocaleString("id-ID") : ""} oninput={(e) => originalAmount = e.currentTarget.value.replace(/\D/g, "")} /></div>
    <div><label class="label" for="obligation-remaining">Sisa <span class="text-muted">(opsional, default = asli)</span></label><input id="obligation-remaining" class="input input-amount" inputmode="numeric" placeholder={originalAmount || "0"} value={remainingAmount ? parseIDR(remainingAmount).toLocaleString("id-ID") : ""} oninput={(e) => remainingAmount = e.currentTarget.value.replace(/\D/g, "")} /></div>
    <div><label class="label" for="obligation-due">Jatuh Tempo</label><input id="obligation-due" type="datetime-local" class="input" bind:value={dueDate} /></div>
    <div><label class="label" for="obligation-note">Catatan <span class="text-muted">(opsional)</span></label><input id="obligation-note" class="input" placeholder="mis. tanpa bunga" bind:value={note} /></div>
    <button class="btn btn-primary btn-block" onclick={submit} disabled={saving}><Icon name="check" size={15} color="#fff" /> {saving ? "Menyimpan…" : "Simpan"}</button>
  </div>
</Modal>

<Modal title={payTarget?.direction === "DEBT" ? "Bayar Hutang" : "Terima Piutang"} open={showPay} onclose={() => showPay = false}>
  {#if payTarget}
    <div class="stack" style="gap:14px">
      <div class="card card-pad-sm" style="background:var(--surface-2);box-shadow:none">
        <div class="h-sm">{payTarget.title}</div>
        <div class="text-muted" style="font-size:12px;margin-top:3px">Sisa {fmtIDR(payTarget.remaining_amount)} dari {fmtIDR(payTarget.original_amount)}</div>
      </div>
      <div><label class="label" for="obligation-pay-amount">Nominal</label><input id="obligation-pay-amount" class="input input-amount" inputmode="numeric" placeholder="0" value={payAmount ? parseIDR(payAmount).toLocaleString("id-ID") : ""} oninput={(e) => payAmount = e.currentTarget.value.replace(/\D/g, "")} /></div>
      <div><label class="label" for="obligation-pay-account">Akun Kas <span class="text-muted">(opsional)</span></label><select id="obligation-pay-account" class="input" bind:value={payAccount}><option value="">Tanpa transaksi kas</option>{#each accounts as a}<option value={a.id}>{a.name} — {fmtIDR(a.current_balance)}</option>{/each}</select></div>
      <div><label class="label" for="obligation-pay-date">Tanggal</label><input id="obligation-pay-date" type="datetime-local" class="input" bind:value={payDate} /></div>
      <button class="btn btn-primary btn-block" onclick={submitPay} disabled={paySaving}><Icon name="check" size={15} color="#fff" /> {paySaving ? "Menyimpan…" : "Simpan Pembayaran"}</button>
    </div>
  {/if}
</Modal>

<style>
  .obligation-summary { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; }
  .obligation-col { min-width: 0; }
  .obligation-label { display: inline-flex; align-items: center; gap: 6px; font-size: 11px; font-weight: 800; letter-spacing: 0.08em; text-transform: uppercase; opacity: 0.85; }
  .obligation-total { font-size: 22px; font-weight: 800; margin-top: 8px; overflow-wrap: anywhere; }
  .obligation-overdue { display: inline-flex; align-items: center; gap: 6px; margin-top: 12px; padding: 5px 9px; border-radius: 999px; background: rgba(255,255,255,.16); font-size: 12px; font-weight: 700; }

  .obligation-card { background: var(--surface); border: 1px solid var(--border); border-radius: var(--radius); padding: 14px; box-shadow: var(--shadow-sm); }
  .obligation-icon { width: 38px; height: 38px; flex: none; }
  .obligation-icon-debt { background: var(--neg-soft); color: var(--neg-ink); }
  .obligation-icon-receivable { background: var(--pos-soft); color: var(--pos-ink); }
  .obligation-title { overflow-wrap: anywhere; }
  .obligation-meta { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; margin-top: 4px; font-size: 12px; color: var(--muted); }
  .obligation-due { font-size: 10.5px; }
  .obligation-due-overdue { background: var(--neg-soft); color: var(--neg-ink); }
  .obligation-note { margin-top: 6px; font-size: 12px; color: var(--muted); overflow-wrap: anywhere; }
  .obligation-progress { margin-top: 12px; }
  .obligation-amounts { display: flex; justify-content: space-between; gap: 8px; margin-top: 7px; font-size: 12px; font-weight: 700; }
  .obligation-original { color: var(--muted); font-weight: 500; }
  .obligation-actions { display: flex; justify-content: flex-end; gap: 6px; margin-top: 10px; padding-top: 10px; border-top: 1px dashed var(--border-strong); }

  .obligation-empty { display: flex; flex-direction: column; align-items: center; text-align: center; gap: 8px; padding: 24px 16px; }
  .obligation-empty-icon { width: 58px; height: 58px; border-radius: 18px; background: var(--brand-soft); color: var(--brand); margin-bottom: 4px; }
  .obligation-empty-title { font-size: 16px; font-weight: 700; }
  .obligation-empty-sub { font-size: 12.5px; color: var(--muted); max-width: 32ch; margin-bottom: 6px; }
</style>
