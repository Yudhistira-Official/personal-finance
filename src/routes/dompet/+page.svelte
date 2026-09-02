<script lang="ts">
  import { onMount } from "svelte";
  import { api, type Account, type AccountType, type Category } from "$lib/api";
  import { fmtIDR } from "$lib/utils";
  import Icon from "$lib/components/Icon.svelte";
  import Modal from "$lib/components/Modal.svelte";
  import Fab from "$lib/components/Fab.svelte";

  let accounts: Account[] = $state([]);
  let categories: Category[] = $state([]);
  let loading = $state(true);
  let err = $state("");

  let showAccountForm = $state(false);
  let editing: Account | null = $state(null);
  let accountName = $state("");
  let accountType: AccountType = $state("bank");
  let accountNumber = $state("");
  let accountBalance = $state("");
  let accountActive = $state(true);

  let showBalanceForm = $state(false);
  let balanceAccount: Account | null = $state(null);
  let balanceValue = $state("");

  let showTransferForm = $state(false);
  let transferSource = $state("");
  let transferDestination = $state("");
  let transferAmount = $state("");
  let transferNote = $state("");
  let transferCategory = $state("");

  const groups: { type: AccountType; label: string; icon: string; color: string }[] = [
    { type: "bank", label: "Bank", icon: "landmark", color: "#2563eb" },
    { type: "ewallet", label: "E-Wallet", icon: "smartphone", color: "#7c3aed" },
    { type: "cash", label: "Cash & Lainnya", icon: "banknote", color: "#d97706" },
    { type: "investment", label: "Investasi", icon: "trending-up", color: "#059669" },
  ];

  /** Load account and category data needed by this page. */
  async function load() {
    loading = true;
    err = "";
    try {
      [accounts, categories] = await Promise.all([
        api.accounts_list(),
        api.categories_list(),
      ]);
    } catch (e: any) {
      err = String(e);
    }
    loading = false;
  }

  /** Reset account form and open it for creating a new account. */
  function openCreate() {
    editing = null;
    accountName = "";
    accountType = "bank";
    accountNumber = "";
    accountBalance = "";
    accountActive = true;
    showAccountForm = true;
  }

  /** Populate account form with selected account values for editing. */
  function openEdit(account: Account) {
    editing = account;
    accountName = account.name;
    accountType = account.account_type;
    accountNumber = account.account_number ?? "";
    accountBalance = account.current_balance.toString();
    accountActive = account.is_active;
    showAccountForm = true;
  }

  /** Keep monetary fields numeric while preserving simple editable text. */
  function onAmountInput(field: "account" | "balance" | "transfer", event: Event) {
    const value = (event.target as HTMLInputElement).value.replace(/\D/g, "");
    if (field === "account") accountBalance = value;
    if (field === "balance") balanceValue = value;
    if (field === "transfer") transferAmount = value;
  }

  /** Save account metadata and refresh visible balances. */
  async function submitAccount() {
    const balance = parseInt(accountBalance || "0", 10);
    if (!accountName.trim()) {
      err = "Nama akun wajib diisi.";
      return;
    }
    try {
      const input = {
        name: accountName.trim(),
        account_type: accountType,
        account_number: accountNumber.trim() || null,
        current_balance: balance,
        is_active: accountActive,
      };
      if (editing) await api.accounts_update(editing.id, input);
      else await api.accounts_create(input);
      showAccountForm = false;
      await load();
    } catch (e: any) {
      err = String(e);
    }
  }

  /** Open direct balance reconciliation for selected account. */
  function openBalance(account: Account) {
    balanceAccount = account;
    balanceValue = account.current_balance.toString();
    showBalanceForm = true;
  }

  /** Save reconciled balance without creating an income or expense. */
  async function submitBalance() {
    if (!balanceAccount) return;
    try {
      await api.accounts_update(balanceAccount.id, {
        name: balanceAccount.name,
        account_type: balanceAccount.account_type,
        account_number: balanceAccount.account_number ?? null,
        current_balance: parseInt(balanceValue || "0", 10),
        is_active: balanceAccount.is_active,
      });
      showBalanceForm = false;
      await load();
    } catch (e: any) {
      err = String(e);
    }
  }

  /** Remove account only after explicit user confirmation. */
  async function doDelete(id: string) {
    if (!confirm("Hapus akun ini? Riwayat transaksi terkait mungkin ikut terdampak.")) return;
    try {
      await api.accounts_delete(id);
      await load();
    } catch (e: any) {
      err = String(e);
    }
  }

  /** Initialize transfer form with distinct accounts and a transfer category. */
  function openTransfer() {
    transferSource = accounts[0]?.id ?? "";
    transferDestination = accounts.find((a) => a.id !== transferSource)?.id ?? "";
    transferAmount = "";
    transferNote = "";
    transferCategory = categories.find((c) => c.category_type === "transfer")?.id ?? categories[0]?.id ?? "";
    showTransferForm = true;
  }

  /** Record an account-to-account transfer using transfer transaction semantics. */
  async function submitTransfer() {
    const amount = parseInt(transferAmount || "0", 10);
    if (!transferSource || !transferDestination || transferSource === transferDestination || !amount || !transferCategory) {
      err = "Lengkapi akun sumber, tujuan, kategori, dan nominal transfer.";
      return;
    }
    try {
      await api.transactions_create({
        account_id: transferSource,
        destination_account_id: transferDestination,
        category_id: transferCategory,
        amount,
        transaction_type: "transfer",
        date: Math.floor(Date.now() / 1000),
        note: transferNote.trim(),
      });
      showTransferForm = false;
      await load();
    } catch (e: any) {
      err = String(e);
    }
  }

  /** Sum of balances across active accounts for the hero card. */
  const activeAccounts = $derived(accounts.filter((a) => a.is_active));
  const totalActiveBalance = $derived(
    activeAccounts.reduce((sum, a) => sum + a.current_balance, 0)
  );

  /** Group accounts while retaining stable product-defined ordering. */
  function accountsFor(type: AccountType) {
    return accounts.filter((account) => account.account_type === type);
  }

  onMount(load);
</script>

<div class="page">
  <div class="page-header">
    <div>
      <h1 class="page-title">Dompet</h1>
      <p class="page-sub">Kelola akun &amp; dompetmu</p>
    </div>
    <button class="btn btn-primary btn-sm" onclick={openCreate}>
      <Icon name="plus" size={15} color="#fff" /> Akun
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
  {:else if !accounts.length}
    <div class="empty">
      <div class="empty-icon"><Icon name="wallet" size={28} /></div>
      <h3 class="h-md" style="margin:0 0 4px;color:var(--ink)">Belum ada akun</h3>
      <p style="font-size:13px;margin:0 0 18px">Tambahkan rekening bank, e-wallet, atau dompet tunai.</p>
      <button class="btn btn-primary" onclick={openCreate}>
        <Icon name="plus" size={15} color="#fff" /> Tambah Akun Pertama
      </button>
    </div>
  {:else}
    <div class="stack">
      <button class="btn btn-secondary btn-block" onclick={openTransfer}>
        <Icon name="arrow-left-right" size={16} /> Transfer Antar-Akun
      </button>

      <div class="card card-hero">
        <div class="hero-ring"></div>
        <div style="position:relative">
          <div style="display:flex;align-items:center;gap:8px;font-size:11px;font-weight:800;letter-spacing:0.1em;text-transform:uppercase;opacity:0.85">
            <Icon name="wallet" size={14} color="#fff" /> Total Saldo Aktif
          </div>
          <div class="num" style="font-size:32px;font-weight:800;letter-spacing:-0.02em;margin-top:10px;line-height:1.1">{fmtIDR(totalActiveBalance)}</div>
          <div class="row" style="margin-top:16px">
            <span class="hero-chip">
              <Icon name="layers" size={13} color="#fff" /> {activeAccounts.length} akun aktif
            </span>
          </div>
        </div>
       </div>

       {#each groups as group}
        {@const grouped = accountsFor(group.type)}
        {#if grouped.length}
          <section>
            <div class="text-muted" style="font-size:11px;font-weight:700;text-transform:uppercase;letter-spacing:0.08em;margin:4px 2px 8px">
              {group.label} · {grouped.length}
            </div>
            <div class="stack" style="gap:10px">
              {#each grouped as account, i (i)}
                {@const isActive = account.is_active}
                <a
                  class="card card-pad-sm fade-item account-card"
                  style={`animation-delay:${i * 50}ms`}
                  href={`/riwayat?account=${encodeURIComponent(account.id)}`}
                >
                  <div class="row" style="align-items:flex-start">
                    <div
                      class="avatar"
                      style={`background:${group.color}1f;color:${group.color}`}
                    >
                      <Icon name={group.icon} size={18} color={group.color} />
                    </div>
                    <div class="grow">
                      <div class="row" style="gap:8px">
                        <span class="h-sm" style="overflow:hidden;text-overflow:ellipsis;white-space:nowrap">{account.name}</span>
                        <span class:badge-pos={isActive} class:badge-neutral={!isActive} class="badge">
                          {isActive ? "Aktif" : "Nonaktif"}
                        </span>
                      </div>
                      {#if account.account_number}
                        <div class="text-muted" style="font-size:11.5px;margin-top:2px;font-family:ui-monospace,SFMono-Regular,Menlo,monospace">
                          {account.account_number}
                        </div>
                      {/if}
                    </div>
                    <div class="h-sm num" style="text-align:right" class:text-pos={account.current_balance >= 0} class:text-neg={account.current_balance < 0}>
                      {fmtIDR(account.current_balance)}
                    </div>
                  </div>

                  <div class="divider-dash">
                    <div class="row" style="justify-content:flex-end;gap:6px" onclick={(event) => event.stopPropagation()}>
                      <button class="btn btn-ghost btn-sm" onclick={(event) => { event.preventDefault(); event.stopPropagation(); openBalance(account); }}>
                        <Icon name="refresh" size={14} /> Sesuaikan
                      </button>
                      <button class="btn btn-ghost btn-sm" onclick={(event) => { event.preventDefault(); event.stopPropagation(); openEdit(account); }} aria-label={`Edit ${account.name}`}>
                        <Icon name="edit" size={14} /> Edit
                      </button>
                      <button class="btn btn-ghost btn-sm" onclick={(event) => { event.preventDefault(); event.stopPropagation(); doDelete(account.id); }} aria-label={`Hapus ${account.name}`}>
                        <Icon name="trash" size={14} /> Hapus
                      </button>
                    </div>
                  </div>
                </a>
              {/each}
            </div>
          </section>
        {/if}
      {/each}
    </div>
  {/if}
</div>

{#if !loading && accounts.length}
  <Fab label="Tambah Akun" onclick={openCreate} />
{/if}

<Modal title={editing ? "Edit Akun" : "Tambah Akun"} open={showAccountForm} onclose={() => showAccountForm = false}>
  <div class="stack" style="gap:14px">
    <div>
      <label class="label" for="account-name">Nama Institusi</label>
      <input id="account-name" class="input" placeholder="mis. BCA Utama" bind:value={accountName} />
    </div>

    <div>
      <label class="label">Jenis Akun</label>
      <div class="segmented">
        {#each groups as group}
          <button
            type="button"
            class:active={accountType === group.type}
            class="seg-item"
            onclick={() => accountType = group.type}
          >
            <Icon name={group.icon} size={15} color={accountType === group.type ? group.color : undefined} />
            {group.type === "bank" ? "Bank" : group.type === "ewallet" ? "E-Wallet" : group.type === "cash" ? "Cash" : "Investasi"}
          </button>
        {/each}
      </div>
    </div>

    <div>
      <label class="label" for="account-number">Nomor Akun <span class="text-muted">(opsional)</span></label>
      <input id="account-number" class="input" placeholder="Nomor rekening / akun" bind:value={accountNumber} />
    </div>

    <div>
      <label class="label" for="account-balance">{editing ? "Saldo Saat Ini" : "Saldo Awal"} (IDR)</label>
      <input
        id="account-balance"
        class="input input-amount"
        inputmode="numeric"
        value={accountBalance ? parseInt(accountBalance, 10).toLocaleString("id-ID") : ""}
        oninput={(event) => onAmountInput("account", event)}
      />
    </div>

    <div class="row" style="justify-content:space-between">
      <span style="font-size:14px;font-weight:600">Akun Aktif</span>
      <button type="button" class:on={accountActive} class="switch" onclick={() => accountActive = !accountActive} aria-label="Akun aktif"></button>
    </div>

    <button class="btn btn-primary btn-block" onclick={submitAccount}>
      {editing ? "Simpan Perubahan" : "Tambah Akun"}
    </button>
  </div>
</Modal>

<Modal title="Sesuaikan Saldo" open={showBalanceForm} onclose={() => showBalanceForm = false}>
  {#if balanceAccount}
    <div class="stack" style="gap:14px">
      <div class="card card-pad-sm" style="background:var(--surface-2);box-shadow:none">
        <div class="h-sm">{balanceAccount.name}</div>
        <div class="text-muted" style="font-size:12px;margin-top:2px">Saldo tersimpan: {fmtIDR(balanceAccount.current_balance)}</div>
      </div>
      <div>
        <label class="label" for="balance-value">Saldo Aktual (IDR)</label>
        <input
          id="balance-value"
          class="input input-amount"
          inputmode="numeric"
          value={balanceValue ? parseInt(balanceValue, 10).toLocaleString("id-ID") : ""}
          oninput={(event) => onAmountInput("balance", event)}
        />
        <p class="text-muted" style="font-size:11.5px;margin:8px 0 0">Penyesuaian saldo tanpa membuat transaksi.</p>
      </div>
      <button class="btn btn-primary btn-block" onclick={submitBalance}>
        <Icon name="check" size={15} color="#fff" /> Simpan
      </button>
    </div>
  {/if}
</Modal>

<Modal title="Transfer Antar-Akun" open={showTransferForm} onclose={() => showTransferForm = false}>
  <div class="stack" style="gap:14px">
    <div>
      <label class="label" for="transfer-source">Akun Sumber</label>
      <select id="transfer-source" class="input" bind:value={transferSource}>
        <option value="" disabled>Pilih akun sumber</option>
        {#each accounts as account}<option value={account.id}>{account.name} — {fmtIDR(account.current_balance)}</option>{/each}
      </select>
    </div>
    <div>
      <label class="label" for="transfer-destination">Akun Tujuan</label>
      <select id="transfer-destination" class="input" bind:value={transferDestination}>
        <option value="" disabled>Pilih akun tujuan</option>
        {#each accounts.filter((account) => account.id !== transferSource) as account}<option value={account.id}>{account.name}</option>{/each}
      </select>
    </div>
    <div>
      <label class="label" for="transfer-amount">Nominal Transfer (IDR)</label>
      <input
        id="transfer-amount"
        class="input input-amount"
        inputmode="numeric"
        value={transferAmount ? parseInt(transferAmount, 10).toLocaleString("id-ID") : ""}
        oninput={(event) => onAmountInput("transfer", event)}
      />
    </div>
    <div>
      <label class="label" for="transfer-category">Kategori</label>
      <select id="transfer-category" class="input" bind:value={transferCategory}>
        <option value="" disabled>Pilih kategori</option>
        {#each categories as category}<option value={category.id}>{category.name}</option>{/each}
      </select>
    </div>
    <div>
      <label class="label" for="transfer-note">Catatan <span class="text-muted">(opsional)</span></label>
      <input id="transfer-note" class="input" placeholder="mis. Pindah dana bulanan" bind:value={transferNote} />
    </div>
    <p class="text-muted" style="font-size:11.5px;margin:0">Transfer tidak dihitung sebagai pemasukan/pengeluaran.</p>
    <button class="btn btn-primary btn-block" onclick={submitTransfer}>
      <Icon name="send" size={15} color="#fff" /> Catat Transfer
    </button>
  </div>
</Modal>



<style>
  .account-card {
    display: block;
    color: inherit;
    text-decoration: none;
    cursor: pointer;
  }
  .account-card:hover {
    border-color: var(--border-strong);
  }
</style>
