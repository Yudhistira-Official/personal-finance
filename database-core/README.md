# Database Core — Google Apps Script Web App

Database Core adalah endpoint sinkronisasi untuk aplikasi **Personal Finance** (Tauri v2 + Rust).
Endpoint ini menerima data dari aplikasi desktop (`sync_push`), mengirim balik data saat diminta
(`sync_fetch`), serta memberi sinyal hidup untuk `sync_test`.

## Persiapan Spreadsheet

1. Buka [Google Sheets](https://sheets.google.com) dan buat spreadsheet baru.
2. Buka menu **Extensions → Apps Script**.
3. Hapus isi editor, lalu tempel seluruh kode dari `Code.gs`.
4. Klik **Simpan** (Ctrl+S), beri nama proyek misalnya `Personal Finance DB Core`.

## Menjalankan setup()

1. Di editor Apps Script, pilih fungsi `setup` pada dropdown di toolbar.
2. Klik **Run** (▶). Saat diminta izin, klik **Review permissions** → pilih akun → **Allow**.
3. `setup()` akan membuat 4 sheet dengan header tebal (bold):
   - `Transactions`: id, date, type, account_id, category_id, amount, note, updated_at
   - `Accounts`: id, name, account_type, balance, is_active
   - `Savings`: id, name, target_amount, current_amount, linked_account_id
   - `Categories`: id, name, type, icon, color

> Sheet yang sudah ada tidak dihapus; hanya header baris pertama yang dipastikan sesuai.

## Deploy sebagai Web App

1. Klik **Deploy → New deployment**.
2. Pilih type **Web app**.
3. Isi description (opsional), lalu atur:
   - **Execute as**: `Me` (akun Anda sendiri)
   - **Who has access**: `Anyone`
4. Klik **Deploy**, lalu salin **Web app URL** yang muncul.
5. Setiap kali mengubah kode, gunakan **Deploy → Manage deployments → Edit → New version** agar
   perubahan aktif (versi lama tidak ikut berubah).

## Menghubungkan ke Aplikasi Personal Finance

1. Buka aplikasi **Personal Finance**.
2. Masuk ke **Setelan → Integrasi**.
3. Tempel **Web app URL** pada kolom URL Google Spreadsheet / Database.
4. Klik **Uji koneksi** (`sync_test`), lalu gunakan **Push** dan **Fetch** untuk sinkronisasi.

## Format Endpoint

Base URL yang ditempel tidak boleh berisi parameter tambahan.

### GET — Health check

```
GET <url>
```

Respons:

```json
{ "success": true, "message": "Database Core aktif", "sheets": ["Transactions", "Accounts", "Savings", "Categories"] }
```

### GET — Fetch data

```
GET <url>?action=fetch
```

Respons:

```json
{
  "transactions": [...],
  "accounts": [...],
  "savings": [...],
  "categories": [...]
}
```

### POST — Push transaksi

```
POST <url>
Content-Type: application/json

{
  "action": "push",
  "transactions": [
    {
      "id": "...",
      "date": 1700000000,
      "type": "expense",
      "account_id": "...",
      "destination_account_id": "...",
      "category_id": "...",
      "amount": 50000,
      "note": "...",
      "sync_status": "pending"
    }
  ]
}
```

Respons:

```json
{ "success": true, "message": "N transaksi berhasil disimpan", "pushed": N }
```

## Deploy via clasp (opsional)

`clasp` memungkinkan push kode langsung dari terminal tanpa menyalin manual:

1. Salin `.clasp.json.example` menjadi `.clasp.json`.
2. Ganti `scriptId` dengan ID script Anda (dapat dilihat di **Project Settings** pada editor Apps Script).
3. Login clasp: `clasp login`.
4. Push kode: `clasp push`.

> `.clasp.json` berisi ID pribadi dan sebaiknya tidak di-commit ke repository.
