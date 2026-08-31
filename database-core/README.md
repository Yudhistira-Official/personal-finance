# Database Core — Endpoint Sinkronisasi

Database Core adalah endpoint Google Apps Script untuk sinkronisasi dua arah aplikasi **Personal Finance** dengan Google Spreadsheet milik user.

## Cara Pasang (Loader)

1. Buat spreadsheet baru di [Google Sheets](https://sheets.google.com).
2. Buka **Extensions > Apps Script**.
3. Hapus isi editor.
4. Tempel **KODE LOADER** berikut ke editor:

```javascript
/**
 * Personal Finance — Database Core (LOADER)
 * Tempel kode ini di editor Google Apps Script. Kode inti (Code.gs) diambil
 * otomatis dari GitHub, jadi tidak perlu copy-paste manual dan selalu terbaru.
 */
const PF_RAW_BASE = 'https://raw.githubusercontent.com/Yudhistira-Official/personal-finance/main/database-core/';
const PF_CORE_FILE = 'Code.gs';
const PF_CACHE_KEY = 'pf_dbcore_v1';
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

// Titik masuk Web App — delegasikan ke fungsi di Code.gs hasil fetch.
function doGet(e) { eval(pfFetchCore_()); return handleGet_(e); }
function doPost(e) { eval(pfFetchCore_()); return handlePost_(e); }

// Jalankan sekali untuk membuat sheet + header.
function setup() { eval(pfFetchCore_()); return runSetup_(); }
```

5. Klik **Simpan**.

Loader mengambil `Code.gs` terbaru dari GitHub otomatis. Saat pertama dijalankan, Apps Script membutuhkan izin akses internet untuk `UrlFetchApp`.

## Menjalankan setup()

1. Pilih fungsi `setup` di toolbar Apps Script.
2. Klik **Run**.
3. Izinkan akses saat diminta.

`setup()` membuat 4 sheet beserta header bold:

- `Transactions`: `id`, `date`, `type`, `account_id`, `category_id`, `amount`, `note`, `updated_at`
- `Accounts`: `id`, `name`, `account_type`, `balance`, `is_active`
- `Savings`: `id`, `name`, `target_amount`, `current_amount`, `linked_account_id`
- `Categories`: `id`, `name`, `type`, `icon`, `color`

Sheet yang sudah ada tidak dihapus. Baris pertama dipastikan menjadi header standar dan dibekukan.

## Deploy sebagai Web App

1. Buka **Deploy > New deployment**.
2. Pilih **Web app**.
3. Atur **Execute as** menjadi `Me`.
4. Atur **Who has access** menjadi `Anyone`.
5. Klik **Deploy**.
6. Salin URL yang berakhiran `/exec`.

## Koneksi ke Aplikasi

1. Buka **Personal Finance > Pengaturan > Integrasi Google Spreadsheet**.
2. Tempel URL Web App `/exec`.
3. Klik **Uji Koneksi** untuk memeriksa health check endpoint.
4. Klik **Tarik Data** untuk menjalankan `fetch` dan menggabungkan data Spreadsheet ke SQLite lokal.
5. Klik **Kirim Data** untuk menjalankan `push` transaksi lokal yang belum tersinkronisasi ke Spreadsheet.

## Format Endpoint

| Operasi | Method | URL / Body | Fungsi |
| --- | --- | --- | --- |
| Health check (`sync_test`) | `GET` | `<url>` | Memeriksa endpoint aktif. |
| Fetch (`sync_fetch`) | `GET` | `<url>?action=fetch` | Mengambil transaksi dan data master. |
| Push (`sync_push`) | `POST` | JSON `{"action":"push","transactions":[...]}` | Mengirim transaksi baru ke sheet `Transactions`. |

Base URL tidak boleh berisi parameter tambahan.

## Memperbarui Kode

Cukup push perubahan `database-core/Code.gs` ke GitHub. Loader otomatis mengambil versi baru setelah cache 5 menit kedaluwarsa. Tidak perlu menempel ulang kode loader.
