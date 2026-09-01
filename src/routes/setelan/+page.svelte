<script lang="ts">
  import { onMount } from "svelte";
  import { api, type Category, type SyncInfo } from "$lib/api";
  import { checkForUpdate } from "$lib/updater";
  import { enableAutoSync, disableAutoSync, applyAutoSync } from "$lib/autosync";
  import Icon from "$lib/components/Icon.svelte";
  import Modal from "$lib/components/Modal.svelte";
  import IconPicker from "$lib/components/IconPicker.svelte";

  // ---- state ----
  let loading = $state(true);
  let loadError = $state("");

  let sheetUrl = $state("");
  let autoSync = $state(false);
  let syncInfo = $state<SyncInfo | null>(null);
  let syncMsg = $state<{ ok: boolean; text: string } | null>(null);
  let busy = $state<"test" | "fetch" | "push" | "save" | null>(null);

  let categories = $state<Category[]>([]);
  let catError = $state("");

  // add category modal
  let showCatModal = $state(false);
  let newName = $state("");
  let newType = $state<"income" | "expense" | "transfer">("expense");
  let newIcon = $state("wallet");
  let newColor = $state("#4f8ef7");
  let savingCat = $state(false);

  let resetMsg = $state<{ ok: boolean; text: string } | null>(null);
  let checkingUpdate = $state(false);

  // Handle interval auto-sync — sekarang dihoist ke $lib/autosync.ts agar tetap hidup saat pindah halaman.
  // Simpan URL awal agar toggle tidak menghapus nilai tersimpan bila input kosong.
  // preset color swatches for the category picker
  const presetColors = [
    "#4f8ef7",
    "#10b981",
    "#f43f5e",
    "#f59e0b",
    "#8b5cf6",
    "#0ea5e9",
    "#ec4899",
    "#14b8a6",
  ];

  const groups = $derived([
    { key: "income", label: "Pemasukan", items: categories.filter((c) => c.category_type === "income") },
    { key: "expense", label: "Pengeluaran", items: categories.filter((c) => c.category_type === "expense") },
    { key: "transfer", label: "Transfer", items: categories.filter((c) => c.category_type === "transfer") },
  ]);

  onMount(async () => {
    loading = true;
    loadError = "";
    try {
      const [info, cats] = await Promise.all([api.sync_status(), api.categories_list()]);
      syncInfo = info;
      sheetUrl = info?.sheet_url ?? "";
      autoSync = info.auto_sync === true;
      categories = cats;
      // Check silently on entry; failures and no-update results stay invisible.
      void checkForUpdate();
    } catch (e) {
      loadError = String(e);
    } finally {
      loading = false;
    }
  });

  function flash(ok: boolean, text: string) {
    syncMsg = { ok, text };
  }

  // Toggle auto-sync: simpan ke backend lalu start/stop interval global sesuai nilai.
  async function toggleAutoSync() {
    const next = !autoSync;
    const savedUrl = syncInfo?.sheet_url ?? null;
    try {
      if (next) await enableAutoSync(savedUrl);
      else await disableAutoSync(savedUrl);
      autoSync = next;
      await refreshStatus();
    } catch (e) {
      flash(false, `Gagal menyimpan pengaturan: ${String(e)}`);
    }
  }

  async function refreshStatus() {
    try {
      syncInfo = await api.sync_status();
    } catch {
      /* keep old status */
    }
  }

  async function testConnection() {
    busy = "test";
    try {
      await api.sync_test(sheetUrl);
      flash(true, "Koneksi berhasil. Spreadsheet dapat diakses.");
    } catch (e) {
      flash(false, `Uji koneksi gagal: ${String(e)}`);
    } finally {
      busy = null;
    }
  }

  async function fetchData() {
    busy = "fetch";
    try {
      await api.sync_fetch();
      flash(true, "Data berhasil ditarik dari spreadsheet.");
      await refreshStatus();
    } catch (e) {
      flash(false, `Tarik data gagal: ${String(e)}`);
    } finally {
      busy = null;
    }
  }

  async function pushData() {
    busy = "push";
    try {
      await api.sync_push();
      flash(true, "Data berhasil dikirim ke spreadsheet.");
      await refreshStatus();
    } catch (e) {
      flash(false, `Kirim data gagal: ${String(e)}`);
    } finally {
      busy = null;
    }
  }

  async function saveSettings() {
    busy = "save";
    try {
      await api.settings_save(sheetUrl, autoSync);
      flash(true, "Pengaturan berhasil disimpan.");
      applyAutoSync(autoSync);
      await refreshStatus();
    } catch (e) {
      flash(false, `Gagal menyimpan pengaturan: ${String(e)}`);
    } finally {
      busy = null;
    }
  }

  async function createCategory() {
    if (!newName.trim()) return;
    savingCat = true;
    catError = "";
    try {
      await api.categories_create({
        name: newName.trim(),
        category_type: newType,
        icon: newIcon.trim() || "wallet",
        color_hex: newColor,
      });
      categories = await api.categories_list();
      showCatModal = false;
      newName = "";
      newType = "expense";
      newIcon = "wallet";
      newColor = "#4f8ef7";
    } catch (e) {
      catError = String(e);
    } finally {
      savingCat = false;
    }
  }

  async function deleteCategory(id: string, name: string) {
    if (!window.confirm(`Hapus kategori "${name}"?`)) return;
    catError = "";
    try {
      await api.categories_delete(id);
      categories = await api.categories_list();
    } catch (e) {
      catError = String(e);
    }
  }

  // Cek pembaruan manual; tampilkan hasil di feedback box (relaunch menutup app).
  async function runUpdateCheck() {
    if (checkingUpdate) return;
    checkingUpdate = true;
    try {
      const version = await checkForUpdate();
      if (version) {
        flash(true, `Versi baru v${version} terpasang, aplikasi dimulai ulang…`);
      } else {
        flash(true, "Sudah versi terbaru.");
      }
    } catch {
      flash(false, "Gagal memeriksa pembaruan.");
    } finally {
      checkingUpdate = false;
    }
  }

  async function resetData() {
    if (!window.confirm("Yakin ingin menghapus SEMUA data lokal? Tindakan ini tidak dapat dibatalkan.")) return;
    resetMsg = null;
    try {
      await api.reset_data();
      resetMsg = { ok: true, text: "Semua data lokal berhasil dihapus." };
      categories = await api.categories_list();
      await refreshStatus();
    } catch (e) {
      resetMsg = { ok: false, text: `Reset gagal: ${String(e)}` };
    }
  }

  // Timer auto-sync kini global ($lib/autosync.ts, dikelola layout) — tidak dibersihkan saat unmount.
</script>

<div class="page">
  <div class="page-header">
    <div>
      <h1 class="page-title">Pengaturan</h1>
      <p class="page-sub">Integrasi, kategori &amp; data</p>
    </div>
  </div>

  {#if loading}
    <div class="empty">
      <div class="empty-icon">
        <Icon name="settings" size={26} />
      </div>
      <p>Memuat pengaturan…</p>
    </div>
  {:else if loadError}
    <div class="card row">
      <div class="icon-tile" style="background:var(--neg-soft)">
        <Icon name="alert" size={22} color="var(--neg)" />
      </div>
      <div class="grow">
        <p class="h-sm" style="margin:0">Gagal memuat</p>
        <p class="text-muted" style="margin:2px 0 0;font-size:13px">{loadError}</p>
      </div>
    </div>
  {:else}
    <div class="stack">
      <!-- ============ 1. Integrasi Google Spreadsheet ============ -->
      <section>
        <h2 class="h-sm" style="margin:0 0 10px">Integrasi Google Spreadsheet</h2>
        <div class="card stack">
          <!-- status row -->
          <div class="row">
            <div class="icon-tile" style="background:var(--brand-soft)">
              <Icon name="cloud" size={22} color="var(--brand)" />
            </div>
            <div class="grow">
              <div class="row" style="gap:8px">
                <span class="h-sm" style="margin:0">Status Sinkronisasi</span>
                {#if syncInfo?.status === "synced"}
                  <span class="badge badge-pos"><span class="badge-dot"></span>Synced</span>
                {:else if (syncInfo?.pending_count ?? 0) > 0}
                  <span class="badge badge-warn"><span class="badge-dot"></span>{syncInfo?.pending_count} menunggu</span>
                {:else}
                  <span class="badge badge-neutral">Belum sinkron</span>
                {/if}
              </div>
              <p class="text-muted" style="margin:2px 0 0;font-size:12px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap">
                {syncInfo?.sheet_url ?? "Belum diatur"}
              </p>
            </div>
          </div>

          <div class="divider"></div>

          <!-- sheet url input -->
          <div>
            <label class="label" for="sheet-url">URL Spreadsheet</label>
            <div style="position:relative">
              <span style="position:absolute;left:12px;top:50%;transform:translateY(-50%);display:flex;color:var(--muted);pointer-events:none">
                <Icon name="link" size={17} />
              </span>
              <input
                id="sheet-url"
                class="input"
                type="text"
                placeholder="https://script.google.com/…"
                style="padding-left:38px"
                bind:value={sheetUrl}
              />
            </div>
          </div>

          <!-- action buttons -->
          <div class="grid-2">
            <button class="btn btn-secondary" onclick={testConnection} disabled={busy !== null || !sheetUrl}>
              <Icon name="link" size={16} /> {busy === "test" ? "Menguji…" : "Uji Koneksi"}
            </button>
            <button class="btn btn-secondary" onclick={fetchData} disabled={busy !== null || !sheetUrl}>
              <Icon name="download" size={16} /> {busy === "fetch" ? "Menarik…" : "Tarik Data"}
            </button>
            <button class="btn btn-secondary" onclick={pushData} disabled={busy !== null || !sheetUrl}>
              <Icon name="upload" size={16} /> {busy === "push" ? "Mengirim…" : "Kirim Data"}
            </button>
            <button class="btn btn-primary" onclick={saveSettings} disabled={busy !== null}>
              <Icon name="check" size={16} /> {busy === "save" ? "Menyimpan…" : "Simpan"}
            </button>
          </div>

          <!-- auto sync -->
          <div class="divider"></div>
          <div class="row">
            <div class="grow">
              <p class="h-sm" style="margin:0">Sinkronisasi Otomatis</p>
              <p class="text-muted" style="margin:2px 0 0;font-size:12px">Sinkronkan otomatis setelah transaksi baru</p>
            </div>
            <button
              class="switch"
              class:on={autoSync}
              role="switch"
              aria-checked={autoSync}
              aria-label="Sinkronisasi Otomatis"
              onclick={toggleAutoSync}
            ></button>
          </div>

          <!-- feedback -->
          {#if syncMsg}
            <div
              class="row"
              style="padding:10px 12px;border-radius:12px;gap:8px;font-size:13px;font-weight:600;background:{syncMsg.ok ? 'var(--pos-soft)' : 'var(--neg-soft)'};color:{syncMsg.ok ? 'var(--pos-ink)' : 'var(--neg-ink)'}"
            >
              <Icon name={syncMsg.ok ? "check" : "alert"} size={16} />
              <span class="grow">{syncMsg.text}</span>
            </div>
          {/if}
        </div>
      </section>

      <!-- ============ 2. Kategori ============ -->
      <section>
        <h2 class="h-sm" style="margin:0 0 10px">Kategori</h2>
        <div class="card">
          <div class="row" style="justify-content:space-between;margin-bottom:6px">
            <span class="h-sm" style="margin:0">Kelola Kategori</span>
            <button class="btn btn-soft btn-sm" onclick={() => (showCatModal = true)}>
              <Icon name="plus" size={15} /> Tambah
            </button>
          </div>

          {#if catError}
            <div
              class="row"
              style="padding:10px 12px;border-radius:12px;gap:8px;font-size:13px;font-weight:600;background:var(--neg-soft);color:var(--neg-ink)"
            >
              <Icon name="alert" size={16} />
              <span class="grow">{catError}</span>
            </div>
          {/if}

          {#if categories.length === 0}
            <p class="text-muted" style="margin:12px 0 4px;font-size:13px">Belum ada kategori</p>
          {:else}
            {#each groups as g (g.key)}
              {#if g.items.length > 0}
                <p class="text-muted" style="margin:14px 0 2px;font-size:11px;font-weight:700;letter-spacing:0.06em;text-transform:uppercase">
                  {g.label}
                </p>
                {#each g.items as c, i (c.id)}
                  {#if i > 0}<div class="divider"></div>{/if}
                  <div class="row" style="padding:10px 0">
                    <span class="badge-dot" style="background:{c.color_hex};width:12px;height:12px;flex-shrink:0"></span>
                    <Icon name={c.icon} size={18} color={c.color_hex} />
                    <span class="h-sm grow" style="margin:0">{c.name}</span>
                    <button
                      class="btn btn-ghost btn-sm"
                      style="padding:7px;color:var(--neg)"
                      title="Hapus kategori"
                      aria-label="Hapus {c.name}"
                      onclick={() => deleteCategory(c.id, c.name)}
                    >
                      <Icon name="trash" size={15} />
                    </button>
                  </div>
                {/each}
              {/if}
            {/each}
          {/if}
        </div>
      </section>

      <!-- ============ 3. Data & Keamanan ============ -->
      <section>
        <h2 class="h-sm" style="margin:0 0 10px">Data &amp; Keamanan</h2>
        <div class="card stack">
          <div class="row">
            <div class="icon-tile" style="background:var(--brand-soft)">
              <Icon name="shield" size={22} color="var(--brand)" />
            </div>
            <div class="grow">
              <p class="h-sm" style="margin:0">Backup Data</p>
              <p class="text-muted" style="margin:2px 0 0;font-size:12px">Ekspor CSV tersedia di halaman Riwayat.</p>
            </div>
          </div>

           <div class="divider"></div>

           <div class="row">
             <div class="icon-tile" style="background:var(--brand-soft)">
               <Icon name="refresh" size={22} color="var(--brand)" />
             </div>
             <div class="grow">
               <p class="h-sm" style="margin:0">Pembaruan Otomatis</p>
               <p class="text-muted" style="margin:2px 0 0;font-size:12px">Periksa versi terbaru aplikasi dari GitHub Releases.</p>
             </div>
             <button class="btn btn-secondary btn-sm" onclick={runUpdateCheck} disabled={checkingUpdate}>
               <Icon name="refresh" size={15} /> {checkingUpdate ? "Memeriksa…" : "Cek Pembaruan"}
             </button>
           </div>

           <div class="divider"></div>

           <div class="row">
             <div class="icon-tile" style="background:var(--neg-soft)">
              <Icon name="alert" size={22} color="var(--neg)" />
            </div>
            <div class="grow">
              <p class="h-sm" style="margin:0">Reset Semua Data</p>
              <p class="text-muted" style="margin:2px 0 0;font-size:12px">Hapus akun, transaksi &amp; tabungan</p>
            </div>
            <button class="btn btn-danger btn-sm" onclick={resetData}>Reset</button>
          </div>

          {#if resetMsg}
            <div
              class="row"
              style="padding:10px 12px;border-radius:12px;gap:8px;font-size:13px;font-weight:600;background:{resetMsg.ok ? 'var(--pos-soft)' : 'var(--neg-soft)'};color:{resetMsg.ok ? 'var(--pos-ink)' : 'var(--neg-ink)'}"
            >
              <Icon name={resetMsg.ok ? "check" : "alert"} size={16} />
              <span class="grow">{resetMsg.text}</span>
            </div>
          {/if}
        </div>
      </section>
    </div>
  {/if}
</div>

<!-- Modal tambah kategori -->
<Modal title="Tambah Kategori" open={showCatModal} onclose={() => (showCatModal = false)}>
  <div class="stack" style="gap:14px">
    <div>
      <label class="label" for="cat-name">Nama</label>
      <input id="cat-name" class="input" type="text" placeholder="cth. Gaji, Makan, Transfer" bind:value={newName} />
    </div>

    <div>
      <label class="label">Jenis</label>
      <div class="segmented">
        <button
          class="seg-item seg-income"
          class:active={newType === "income"}
          onclick={() => (newType = "income")}
        >Pemasukan</button>
        <button
          class="seg-item seg-expense"
          class:active={newType === "expense"}
          onclick={() => (newType = "expense")}
        >Pengeluaran</button>
        <button
          class="seg-item seg-transfer"
          class:active={newType === "transfer"}
          onclick={() => (newType = "transfer")}
        >Transfer</button>
      </div>
    </div>

    <div>
      <span class="label">Ikon</span>
      <IconPicker value={newIcon} color={newColor} onselect={(n) => (newIcon = n)} />
    </div>

    <div>
      <label class="label" for="cat-color">Warna</label>
      <div class="row" style="gap:10px;align-items:stretch">
        <input
          id="cat-color"
          class="input"
          type="color"
          style="width:48px;height:48px;padding:4px;flex-shrink:0;cursor:pointer"
          bind:value={newColor}
        />
        <div class="swatches grow">
          {#each presetColors as c (c)}
            <button
              class="swatch"
              class:selected={newColor === c}
              style="background:{c}"
              aria-label="Warna {c}"
              onclick={() => (newColor = c)}
            ></button>
          {/each}
        </div>
      </div>
    </div>

    {#if catError}
      <div
        class="row"
        style="padding:10px 12px;border-radius:12px;gap:8px;font-size:13px;font-weight:600;background:var(--neg-soft);color:var(--neg-ink)"
      >
        <Icon name="alert" size={16} />
        <span class="grow">{catError}</span>
      </div>
    {/if}

    <button class="btn btn-primary btn-block" onclick={createCategory} disabled={savingCat || !newName.trim()}>
      <Icon name="plus" size={16} /> {savingCat ? "Menyimpan…" : "Tambah Kategori"}
    </button>
  </div>
</Modal>
