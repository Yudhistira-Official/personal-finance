# Personal Finance

Aplikasi keuangan pribadi offline-first untuk desktop dan mobile.

## Fitur Utama

- Multi-akun: Bank, E-Wallet, Cash, Investasi
- Pencatatan transaksi: pemasukan, pengeluaran, transfer
- Pos tabungan (*envelope budgeting*) dengan target dan estimasi
- Dashboard: net worth, cashflow bulanan, donut pengeluaran per kategori
- Riwayat dengan filter, pencarian, dan ekspor CSV
- Sinkronisasi dua arah ke Google Spreadsheet
- Offline-first dengan SQLite lokal
- Nominal `i64` untuk mencegah error floating point

## Tech Stack

- Tauri v2
- Rust: `rusqlite`/SQLite, `reqwest` + `tokio`
- SvelteKit + Svelte 5
- Tailwind CSS
- Google Apps Script (`database-core`)

## Struktur Proyek

```text
src/          # Frontend
src-tauri/    # Rust core + IPC + sync
database-core/ # Backend Google Apps Script
```

## Menjalankan Aplikasi

### Prasyarat

- Rust dan Cargo
- Node.js serta npm

### Development

```bash
npm install
npm run tauri dev
```

### Build

```bash
npm run tauri build
```

## Tutorial Menggunakan Database (Sinkronisasi Google Spreadsheet)

1. **Siapkan backend** — gunakan folder `database-core/`. Ikuti detail loader dan deployment di [`database-core/README.md`](database-core/README.md).
2. **Jalankan `setup()` sekali** — fungsi membuat 4 sheet dan header standar.
3. **Deploy Web App** — pilih **Execute as: Me** dan **Who has access: Anyone**.
4. **Tempel URL** — buka aplikasi, masuk ke **Pengaturan**, lalu isi URL Web App pada integrasi Google Spreadsheet.
5. **Uji Koneksi** — pastikan endpoint merespons health check.
6. **Kirim Data (`push`) / Tarik Data (`fetch`)** — kirim transaksi lokal ke Sheets atau ambil data dari Sheets ke aplikasi.
7. **Aktifkan Auto-Sync** — biarkan aplikasi menjalankan sinkronisasi berkala sesuai kebutuhan.

Alur data:

- Transaksi baru disimpan ke SQLite lokal.
- `push` mengirim transaksi yang belum tersinkronisasi ke Google Sheets.
- `fetch` mengambil data dari Google Sheets lalu menggabungkannya ke database lokal menggunakan `INSERT OR IGNORE` berdasarkan `id`.

## Catatan Privasi

Data disimpan lokal di SQLite pada folder aplikasi. Tidak ada data dikirim ke pihak ketiga selain Google Spreadsheet milik user.
