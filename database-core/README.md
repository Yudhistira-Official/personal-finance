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
const PF_CACHE_KEY = 'pf_dbcore_v3';
const PF_CACHE_TTL = 300; // detik

// Referensi STATIS ke SpreadsheetApp. Kode SpreadsheetApp asli ada di dalam
// eval() (Code.gs dari GitHub) sehingga TIDAK terdeteksi analyzer scope Apps
// Script. Baris ini memaksa scope "spreadsheets" diminta saat otorisasi,
// tanpa itu setup()/push()/fetch() gagal dengan "permissions are not sufficient".
function pfScopeHint_() {
  return [SpreadsheetApp.getActiveSpreadsheet, UrlFetchApp.fetch, CacheService.getScriptCache, ContentService.createTextOutput];
}

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
  pfScopeHint_();
  try { eval(pfFetchCore_()); return globalThis.handleGet_(e); }
  catch (err) { return pfJson_({ success: false, message: 'Loader: ' + err.message }); }
}
function doPost(e) {
  pfScopeHint_();
  try { eval(pfFetchCore_()); return globalThis.handlePost_(e); }
  catch (err) { return pfJson_({ success: false, message: 'Loader: ' + err.message }); }
}

// Jalankan sekali untuk membuat sheet + header.
function setup() {
  pfScopeHint_();
  eval(pfFetchCore_());
  return globalThis.runSetup_();
}
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

## Troubleshooting

| Gejala | Penyebab umum | Solusi |
| --- | --- | --- |
| `Exception: Specified permissions are not sufficient ... spreadsheets` | Scope `spreadsheets` tidak diminta (karena kode SpreadsheetApp tersembunyi di `eval`) | Tempel loader **versi `pfScopeHint_`** terbaru, jalankan `setup()` lagi, lalu **Review permissions → Allow** untuk menyetujui scope baru. |
| `handleGet_ is not defined` / `runSetup_ is not defined` | Loader lama (sebelum fix `globalThis`) | Tempel ulang **KODE LOADER** terbaru di atas (versi `globalThis`), lalu Deploy ulang. |
| `Gagal mengambil Code.gs dari GitHub (HTTP 404)` | URL raw salah / file belum di-push | Pastikan `database-core/Code.gs` sudah ada di branch `main` repo. |
| Web App minta login / `401` | Akses deployment bukan "Anyone" | Deploy > Manage deployments > ubah *Who has access* ke **Anyone**. |
| `Exception: Authorization required` saat `UrlFetchApp` | Izin eksternal belum diberikan | Jalankan `setup()` dari editor Apps Script sekali, klik **Review permissions → Allow**. |
| Perubahan `Code.gs` belum terlihat | Cache loader 5 menit | Tunggu ~5 menit, atau naikkan `PF_CACHE_KEY` (mis. `pf_dbcore_v2`) untuk memaksa refresh. |
| `SpreadsheetApp.getActiveSpreadsheet()` null | Script tidak terikat ke spreadsheet | Buat script lewat **Extensions > Apps Script** dari dalam spreadsheet (bukan project standalone). |

> Setelah mengubah loader di editor, selalu klik **Deploy > Manage deployments > Edit > New version > Deploy**. Tanpa deploy ulang, Web App masih menjalankan versi lama.
