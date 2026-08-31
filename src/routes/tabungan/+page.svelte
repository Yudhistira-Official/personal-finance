<script lang="ts">
  import { onMount } from "svelte";
  import { api, type Account, type SavingsPocket } from "$lib/api";
  import { fmtIDR, fmtShortDate } from "$lib/utils";
  import Icon from "$lib/components/Icon.svelte";
  import Progress from "$lib/components/Progress.svelte";
  import Modal from "$lib/components/Modal.svelte";
  import Fab from "$lib/components/Fab.svelte";

  let pockets: SavingsPocket[] = $state([]);
  let accounts: Account[] = $state([]);
  let loading = $state(true);
  let err = $state("");
  let showForm = $state(false);
  let editing: SavingsPocket | null = $state(null);

  let fName = $state("");
  let fTarget = $state("");
  let fColor = $state("#10b981");
  let fAccount = $state("");
  let fTargetDate = $state("");
  let fLocked = $state(false);

  let actionPocket: SavingsPocket | null = $state(null);
  let showAction = $state(false);
  let actionType: "deposit" | "withdraw" = $state("deposit");
  let actionAmount = $state("");

  const cycleColors = ["#10b981", "#2563eb", "#f59e0b", "#ef4444", "#06b6d4", "#ec4899", "#8b5cf6", "#84cc16"];
  const totalTarget = $derived(pockets.reduce((sum, pocket) => sum + pocket.target_amount, 0));
  const totalCurrent = $derived(pockets.reduce((sum, pocket) => sum + pocket.current_amount, 0));
  const overallPct = $derived(totalTarget > 0 ? Math.min(100, Math.round((totalCurrent / totalTarget) * 100)) : 0);

  /** Calculate weekly contribution needed to reach pocket target by its deadline. */
  function calcEstimate(p: SavingsPocket): string {
    if (!p.target_date) return "-";
    const now = Date.now() / 1000;
    const remaining = p.target_amount - p.current_amount;
    if (remaining <= 0) return "Tercapai ✓";
    const secsLeft = p.target_date - now;
    if (secsLeft <= 0) return "Tenggat lewat";
    const weeks = secsLeft / (7 * 86400);
    const perWeek = Math.ceil(remaining / Math.max(1, weeks));
    return `${fmtIDR(perWeek)}/minggu`;
  }

  /** Load pockets and accounts together so linked-account labels stay current. */
  async function load() {
    loading = true;
    try {
      const [pkts, accts] = await Promise.all([api.pockets_list(), api.accounts_list()]);
      pockets = pkts;
      accounts = accts;
      if (accts.length && !fAccount) fAccount = accts[0].id;
    } catch (e: any) {
      err = String(e);
    }
    loading = false;
  }

  /** Reset form state for creating a new savings pocket. */
  function openCreate() {
    editing = null;
    fName = "";
    fTarget = "";
    fColor = "#10b981";
    fAccount = accounts[0]?.id ?? "";
    fTargetDate = "";
    fLocked = false;
    showForm = true;
  }

  /** Populate form state from an existing pocket for editing. */
  function openEdit(p: SavingsPocket) {
    editing = p;
    fName = p.name;
    fTarget = p.target_amount.toString();
    fColor = p.color_tag;
    fAccount = p.linked_account_id;
    fTargetDate = p.target_date ? new Date(p.target_date * 1000).toISOString().slice(0, 16) : "";
    fLocked = p.is_locked;
    showForm = true;
  }

  /** Validate and persist create/edit form data, then refresh displayed balances. */
  async function submitForm() {
    const target = parseInt(fTarget.replace(/\D/g, ""), 10);
    if (!fName.trim() || !target) {
      err = "Nama dan target nominal wajib diisi.";
      return;
    }
    const input: any = {
      name: fName.trim(),
      target_amount: target,
      linked_account_id: fAccount,
      target_date: fTargetDate ? Math.floor(new Date(fTargetDate).getTime() / 1000) : null,
      color_tag: fColor,
      is_locked: fLocked,
    };
    try {
      if (editing) await api.pockets_update(editing.id, input);
      else await api.pockets_create(input);
      showForm = false;
      await load();
    } catch (e: any) {
      err = String(e);
    }
  }

  /** Confirm destructive action before deleting a pocket. */
  async function doDelete(id: string) {
    if (!confirm("Hapus kantong ini?")) return;
    try {
      await api.pockets_delete(id);
      await load();
    } catch (e: any) {
      err = String(e);
    }
  }

  /** Open deposit or withdraw dialog for selected pocket. */
  function openAction(p: SavingsPocket, type: "deposit" | "withdraw") {
    actionPocket = p;
    actionType = type;
    actionAmount = "";
    showAction = true;
  }

  /** Validate and persist deposit or withdrawal, then reload pocket balances. */
  async function submitAction() {
    if (!actionPocket) return;
    const amt = parseInt(actionAmount.replace(/\D/g, ""), 10);
    if (!amt) {
      err = "Nominal wajib diisi.";
      return;
    }
    try {
      if (actionType === "deposit") await api.pockets_deposit(actionPocket.id, amt);
      else await api.pockets_withdraw(actionPocket.id, amt);
      showAction = false;
      await load();
    } catch (e: any) {
      err = String(e);
    }
  }

  /** Keep money fields numeric internally while displaying Indonesian separators. */
  function onAmtInput(field: "target" | "action", e: Event) {
    const raw = (e.target as HTMLInputElement).value.replace(/\D/g, "");
    const val = raw ? parseInt(raw, 10).toString() : "";
    if (field === "target") fTarget = val;
    else actionAmount = val;
  }

  onMount(load);
</script>

<div class="page">
  <div class="page-header">
    <div>
      <h1 class="page-title">Tabungan</h1>
      <p class="page-sub">Pos dana &amp; tujuan finansialmu</p>
    </div>
    <button class="btn btn-primary btn-sm" onclick={openCreate}><Icon name="plus" size={15} color="#fff" /> Baru</button>
  </div>

  {#if err}
    <div class="badge badge-neg" style="width:100%;justify-content:space-between;margin-bottom:14px;padding:10px 12px">
      <span class="row"><Icon name="alert" size={15} /> {err}</span>
      <button class="btn btn-ghost btn-sm" aria-label="Tutup pesan" onclick={() => err = ""}><Icon name="x" size={15} /></button>
    </div>
  {/if}

  {#if loading}
    <div class="empty text-muted">Memuat…</div>
  {:else if !pockets.length}
    <div class="empty card">
      <div class="empty-icon"><Icon name="piggy-bank" size={28} /></div>
      <div class="h-sm">Belum ada pos tabungan</div>
      <p>Mulai pisahkan dana untuk tujuan finansialmu.</p>
      <button class="btn btn-primary" onclick={openCreate}>Buat Kantong Pertama</button>
    </div>
  {:else}
    <div class="stack">
      <div class="card card-hero">
        <div class="hero-ring"></div>
        <div style="position:relative">
          <div style="display:flex;align-items:center;gap:8px;font-size:11px;font-weight:800;letter-spacing:0.1em;text-transform:uppercase;opacity:0.85">
            <Icon name="piggy-bank" size={14} color="#fff" /> Total Terkumpul
          </div>
          <div class="num" style="font-size:32px;font-weight:800;letter-spacing:-0.02em;margin-top:10px;line-height:1.1">{fmtIDR(totalCurrent)}</div>
          <div style="font-size:12px;opacity:0.85;margin-top:3px">dari total target <span class="num">{fmtIDR(totalTarget)}</span></div>
          <div class="progress-track" style="height:7px;background:rgba(255,255,255,.22);margin-top:16px"><Progress progress={overallPct} color="#ffffff" height={7} /></div>
          <div style="font-size:11px;opacity:0.85;text-align:right;margin-top:6px;font-weight:700">{overallPct}% tercapai</div>
        </div>
      </div>

      <div class="stack">
        {#each pockets as p, i}
          {@const pct = p.target_amount > 0 ? Math.min(100, Math.round((p.current_amount / p.target_amount) * 100)) : 0}
          {@const accName = accounts.find((a) => a.id === p.linked_account_id)?.name ?? "—"}
          <div class="card fade-item" style="animation-delay:{i * 40}ms">
            <div class="row" style="align-items:flex-start">
              <div class="icon-tile" style="background:{p.color_tag}22;color:{p.color_tag}"><Icon name="piggy-bank" size={20} /></div>
              <div class="grow">
                <div class="row" style="gap:6px;flex-wrap:wrap">
                  <div class="h-sm">{p.name}</div>
                  {#if p.is_locked}<span class="badge badge-neutral"><Icon name="lock" size={11} /> Terkunci</span>{/if}
                </div>
                <div class="text-muted" style="font-size:11px;margin-top:3px">{accName}</div>
              </div>
              <div class="row" style="gap:2px">
                <button class="btn btn-ghost btn-sm" style="padding:7px" aria-label="Edit {p.name}" onclick={() => openEdit(p)}><Icon name="edit" size={14} /></button>
                <button class="btn btn-ghost btn-sm" style="padding:7px" aria-label="Hapus {p.name}" onclick={() => doDelete(p.id)}><Icon name="trash" size={14} /></button>
              </div>
            </div>

            <div style="margin-top:18px">
              <div class="row" style="align-items:baseline;justify-content:space-between;margin-bottom:8px">
                <span class="h-md num" style="color:{p.color_tag};font-size:21px">{fmtIDR(p.current_amount)}</span>
                <span class="text-muted" style="font-size:11px">dari <span class="num">{fmtIDR(p.target_amount)}</span></span>
              </div>
              <Progress progress={pct} color={p.color_tag} height={8} />
              <div class="row" style="justify-content:space-between;margin-top:7px;font-size:11px">
                <span style="color:{p.color_tag};font-weight:800">{pct}%</span>
                {#if p.target_date}<span class="text-muted row" style="gap:4px"><Icon name="calendar" size={12} /> {fmtShortDate(p.target_date)}</span>{/if}
              </div>
            </div>

            {#if p.target_date && pct < 100}
              <div class="badge badge-pos" style="margin-top:13px;width:100%;padding:8px 10px"><Icon name="sparkles" size={14} /> Estimasi <b>{calcEstimate(p)}</b></div>
            {/if}

            <div class="row" style="margin-top:16px">
              <button class="btn btn-soft btn-sm grow" disabled={p.is_locked} onclick={() => openAction(p, "deposit")}><Icon name="arrow-down-left" size={15} /> Setor</button>
              <button class="btn btn-secondary btn-sm grow" disabled={p.is_locked || p.current_amount === 0} onclick={() => openAction(p, "withdraw")}><Icon name="arrow-up-right" size={15} /> Tarik</button>
            </div>
          </div>
        {/each}
      </div>
    </div>
  {/if}
</div>

{#if !loading && pockets.length}
  <Fab label="Buat Kantong Baru" onclick={openCreate} />
{/if}

<Modal title={editing ? "Edit Kantong" : "Kantong Baru"} open={showForm} onclose={() => showForm = false}>
  <div class="stack">
    <div><label class="label">Nama</label><input class="input" placeholder="mis. Dana Darurat" bind:value={fName} /></div>
    <div><label class="label">Target Nominal</label><input class="input input-amount" inputmode="numeric" placeholder="0" value={fTarget ? fmtIDR(parseInt(fTarget, 10)).replace("Rp ", "") : ""} oninput={(e) => onAmtInput("target", e)} /></div>
    <div><label class="label">Akun Penampung</label><select class="input" bind:value={fAccount}>{#each accounts as a}<option value={a.id}>{a.name} — {fmtIDR(a.current_balance)}</option>{/each}</select></div>
    <div><label class="label">Target Tanggal</label><input class="input" type="datetime-local" bind:value={fTargetDate} /></div>
    <div><label class="label">Warna</label><div class="swatches">{#each cycleColors as c}<button class:selected={fColor === c} class="swatch" style="background:{c}" aria-label="Pilih warna {c}" onclick={() => fColor = c}></button>{/each}</div></div>
    <div class="row" style="justify-content:space-between"><div><div class="h-sm">Kunci kantong</div><div class="text-muted" style="font-size:11px;margin-top:2px">Cegah setor dan tarik dana</div></div><button class:on={fLocked} class="switch" aria-label="Kunci kantong" aria-pressed={fLocked} onclick={() => fLocked = !fLocked}></button></div>
    <button class="btn btn-primary btn-block" onclick={submitForm}>{editing ? "Simpan" : "Buat Kantong"}</button>
  </div>
</Modal>

<Modal title={actionType === "deposit" ? "Setor" : "Tarik"} open={showAction} onclose={() => showAction = false}>
  {#if actionPocket}
    <div class="stack">
      <div class="card card-pad-sm" style="background:{actionPocket.color_tag}15;border-color:{actionPocket.color_tag}30">
        <div class="h-sm">{actionPocket.name}</div>
        <div class="text-muted" style="font-size:12px;margin-top:4px">Saldo saat ini <span class="num">{fmtIDR(actionPocket.current_amount)}</span></div>
      </div>
      <div><label class="label">Nominal</label><input class="input input-amount" inputmode="numeric" placeholder="0" value={actionAmount ? fmtIDR(parseInt(actionAmount, 10)).replace("Rp ", "") : ""} oninput={(e) => onAmtInput("action", e)} /></div>
      <button class="btn btn-primary btn-block" onclick={submitAction}>{actionType === "deposit" ? "Setor Sekarang" : "Tarik Sekarang"}</button>
    </div>
  {/if}
</Modal>
