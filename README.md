# Personal Finance

Aplikasi keuangan pribadi **offline-first** untuk desktop dan mobile — pencatatan arus kas, pemantauan saldo lintas bank/e-wallet, dan alokasi pos tabungan (*envelope budgeting*), dengan sinkronisasi opsional ke Google Spreadsheet.

## Fitur Utama

- **Multi-akun** — Bank, E-Wallet, Cash, Investasi (BNI, BCA, GoPay, Dana, dll.)
- **Transaksi** — pemasukan, pengeluaran, dan transfer antar-akun (transfer tidak dihitung sebagai arus kas)
- **Pos tabungan** — target nominal, progres, dan kalkulator estimasi setor per minggu
- **Dashboard** — net worth, cashflow bulanan, donut pengeluaran per kategori
- **Riwayat** — filter rentang/akun/kategori/tipe, pencarian, ekspor CSV
- **Sinkronisasi dua arah** ke Google Spreadsheet (fetch & push)
- **Offline-first** — semua data tersimpan di SQLite lokal; sync berjalan di latar belakang
- **Presisi uang** — nominal disimpan sebagai `i64` (Rupiah), bebas error floating point

## Tech Stack

| Lapisan | Teknologi |
| --- | --- |
| Shell & IPC | Tauri v2 (Windows, macOS, Linux, Android, iOS) |
| Backend core | Rust — `rusqlite`/SQLite, `reqwest` + `tokio` |
| Frontend | SvelteKit + Svelte 5, Tailwind CSS |
| Sync backend | Google Apps Script Web App (`database-core/`) |

## Struktur Proyek

```text
src/            # Frontend SvelteKit (5 halaman + komponen)
src-tauri/      # Rust core: model, SQLite, IPC commands, sync engine
database-core/  # Backend Google Apps Script (endpoint sync)
```

## Menjalankan Aplikasi

### Prasyarat

- Rust + Cargo
- Node.js + npm

### Development

```bash
npm install
npm run tauri dev
```

### Build desktop

```bash
npm run tauri build
```

### Build Android (APK)

```bash
npm run tauri android init
npm run tauri android build -- --apk --target aarch64
```

## Rilis & APK

APK rilis tersedia di halaman [Releases](https://github.com/Yudhistira-Official/personal-finance/releases). Versi **v0.1.0** berisi APK Android (arm64-v8a) yang sudah **ditandatangani** — siap dipasang langsung via *sideload* (aktifkan "Install unknown apps").

---

# Tutorial: Database Core (Sinkronisasi Google Spreadsheet)

Database Core adalah **endpoint Google Apps Script** yang menjadi "database" awan untuk sinkronisasi dua arah. Kode intinya (`Code.gs`) diambil otomatis dari GitHub lewat **loader** kecil, jadi kamu tidak perlu copy-paste kode besar dan selalu mendapat versi terbaru.

## 1. Pasang Loader

1. Buat spreadsheet baru di [Google Sheets](https://sheets.google.com).
2. Buka **Extensions > Apps Script**.
3. Hapus isi editor, lalu tempel **kode loader** berikut:

```javascript
/**
 * Personal Finance — Database Core (LOADER)
 * Kode inti (Code.gs) diambil otomatis dari GitHub.
 */
const PF_RAW_BASE = 'https://raw.githubusercontent.com/Yudhistira-Official/personal-finance/main/database-core/';
const PF_CORE_FILE = 'Code.gs';
const PF_CACHE_KEY = 'pf_dbcore_v2';
const PF_CACHE_TTL = 300; // detik

function pfFetchCore_() {
  var cache = CacheService.getScriptCache();
  var code = cache.get(PF_CACHE_KEY);
  if (!code) {
    var res = UrlFetchApp.fetch(PF_RAW_BASE + PF_CORE_FILE, { muteHttpExceptions: true });
    var status = res.getResponseCode();
    if (status !== 200) {
      throw new Error('Gagal mengambil ' + PF_CORE_FILE + ' dari GitHub (HTTP ' + status + ').');
    }
    code = res.getContentText();
    cache.put(PF_CACHE_KEY, code, PF_CACHE_TTL);
  }
  return code;
}

function pfJson_(obj) {
  return ContentService.createTextOutput(JSON.stringify(obj))
    .setMimeType(ContentService.MimeType.JSON);
}

// Titik masuk Web App — eval Code.gs lalu panggil fungsinya lewat globalThis.
function doGet(e) {
  try { eval(pfFetchCore_()); return globalThis.handleGet_(e); }
  catch (err) { return pfJson_({ success: false, message: 'Loader: ' + err.message }); }
}
function doPost(e) {
  try { eval(pfFetchCore_()); return globalThis.handlePost_(e); }
  catch (err) { return pfJson_({ success: false, message: 'Loader: ' + err.message }); }
}

// Jalankan sekali untuk membuat sheet + header.
function setup() {
  eval(pfFetchCore_());
  return globalThis.runSetup_();
}
```

4. Klik **Simpan** (Ctrl+S).

## 2. Jalankan `setup()` (sekali)

1. Pilih fungsi **`setup`** pada dropdown di toolbar.
2. Klik **Run** (▶).
3. Saat diminta izin, klik **Review permissions → Allow** (butuh akses internet untuk `UrlFetchApp`).

`setup()` membuat 4 sheet dengan header standar (baris pertama dibekukan):

| Sheet | Kolom |
| --- | --- |
| `Transactions` | id, date, type, account_id, category_id, amount, note, updated_at |
| `Accounts` | id, name, account_type, balance, is_active |
| `Savings` | id, name, target_amount, current_amount, linked_account_id |
| `Categories` | id, name, type, icon, color |

## 3. Deploy sebagai Web App

1. **Deploy > New deployment** → pilih **Web app**.
2. **Execute as**: `Me`.
3. **Who has access**: `Anyone`.
4. Klik **Deploy**, lalu **salin URL** yang berakhiran `/exec`.

## 4. Hubungkan ke Aplikasi

1. Buka **Personal Finance > Pengaturan > Integrasi Google Spreadsheet**.
2. Tempel URL `/exec` tadi.
3. **Uji Koneksi** → pastikan health check OK.
4. **Kirim Data** (`push`) → unggah transaksi lokal ke Sheets.
5. **Tarik Data** (`fetch`) → ambil data dari Sheets ke aplikasi.
6. (Opsional) aktifkan **Auto-Sync**.

## Alur Data

```
[Catat transaksi di UI]
        │  Tauri IPC
        ▼
[State Rust] ──► [SQLite lokal]  (offline-first, selalu tersimpan)
        │
        ▼  async worker (reqwest/tokio)
[Google Apps Script /exec]  ◄── push (POST) / fetch (GET)
        ▼
[Google Spreadsheet milikmu]
```

- **Push**: transaksi berstatus `pending`/`failed` dikirim; idempoten berdasarkan `id`.
- **Fetch**: data dari Sheets digabung ke SQLite lokal via `INSERT OR IGNORE` (baris `synced` tidak diduplikasi).

## Endpoint

| Operasi | Method | URL / Body |
| --- | --- | --- |
| Health check | `GET` | `<url>` |
| Fetch | `GET` | `<url>?action=fetch` |
| Push | `POST` | `{"action":"push","transactions":[...]}` |

## Memperbarui Kode

Ubah `database-core/Code.gs` lalu push ke GitHub. Loader mengambil versi baru otomatis setelah cache 5 menit kedaluwarsa (atau naikkan `PF_CACHE_KEY` untuk memaksa refresh). **Tidak perlu menempel ulang loader.**

> Troubleshooting lengkap (error `handleGet_ is not defined`, HTTP 404, izin akses, dsb.) ada di [`database-core/README.md`](database-core/README.md).

## Catatan Privasi

Data disimpan lokal di SQLite pada folder aplikasi. Tidak ada data finansial dikirim ke pihak ketiga selain Google Spreadsheet milikmu sendiri.
