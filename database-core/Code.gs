/**
 * PERSONAL FINANCE - CORE ENGINE (Optimized)
 * Dijalankan via Bootstrapper Loader dari GitHub.
 */

var SCHEMAS = {
  Transactions: ['id', 'date', 'type', 'account_id', 'destination_account_id', 'category_id', 'amount', 'note', 'updated_at'],
  Accounts: ['id', 'name', 'account_type', 'balance', 'is_active'],
  Savings: ['id', 'name', 'target_amount', 'current_amount', 'linked_account_id'],
  Categories: ['id', 'name', 'type', 'icon', 'color'],
  Investments: ['product_id', 'product_name', 'fund_type', 'manager_name', 'total_units', 'avg_buy_nav', 'current_nav', 'current_value', 'unrealized_pnl', 'roi_percentage', 'updated_at']
};

/**
 * Menangani GET untuk health check dan pengambilan data
 */
function handleGet_(e) {
  try {
    ensureSetup_();
    var action = e && e.parameter ? e.parameter.action : '';
    
    if (action === 'fetch') {
      return json_({
        success: true,
        message: 'Data berhasil diambil',
        data: buildFetchPayload_()
      });
    }

    return json_({
      success: true,
      message: 'Database Core aktif',
      sheets: Object.keys(SCHEMAS)
    });
  } catch (error) {
    return json_({ success: false, message: 'GET Error: ' + error.message });
  }
}

/**
 * Menangani POST untuk push transaksi cepat maupun full sync (Accounts, Savings, Categories)
 */
function handlePost_(e) {
  if (!e || !e.postData || !e.postData.contents) {
    return json_({ success: false, message: 'Body JSON wajib diisi', pushed: 0 });
  }

  try {
    ensureSetup_();
    var payload = JSON.parse(e.postData.contents);
    var action = payload.action || 'push';

    // 1. Action PUSH: Penambahan transaksi baru secara batch (High Performance)
    if (action === 'push') {
      if (!Array.isArray(payload.transactions)) {
        return json_({ success: false, message: 'Field transactions harus berupa array', pushed: 0 });
      }

      var sheet = getSs_().getSheetByName('Transactions');

      // Map id -> nomor baris fisik (1-indexed, header di baris 1) untuk UPSERT.
      var idToRow = {};
      var rawValues = sheet.getDataRange().getValues();
      for (var i = 1; i < rawValues.length; i++) {
        var rid = String(rawValues[i][0] || '').trim();
        if (rid) idToRow[rid] = i + 1;
      }

      var newRows = [];
      var nowIso = new Date().toISOString();
      var updated = 0;

      payload.transactions.forEach(function(tx) {
        var id = String(tx.id || '').trim();
        if (!id) return;

        var row = [
          id,
          tx.date || 0,
          tx.type || 'expense',
          String(tx.account_id || ''),
          // Ikuti urutan SCHEMAS.Transactions: tujuan transfer tepat setelah account_id.
          String(tx.destination_account_id || ''),
          String(tx.category_id || ''),
          tx.amount || 0,
          String(tx.note || ''),
          nowIso
        ];

        if (idToRow[id]) {
          // UPSERT: baris sudah ada -> perbarui seluruh kolom.
          sheet.getRange(idToRow[id], 1, 1, SCHEMAS.Transactions.length).setValues([row]);
          updated++;
        } else {
          newRows.push(row);
          idToRow[id] = true; // tandai agar id duplikat dalam payload hanya sekali
        }
      });

      // Tulis seluruh baris BARU sekaligus dalam satu operasi I/O.
      if (newRows.length > 0) {
        var startRow = sheet.getLastRow() + 1;
        sheet.getRange(startRow, 1, newRows.length, SCHEMAS.Transactions.length).setValues(newRows);
      }

      return json_({
        success: true,
        message: newRows.length + ' transaksi baru, ' + updated + ' diperbarui',
        pushed: newRows.length + updated
      });
    }

    // 2. Action SYNC_ALL: Menimpa / sinkronisasi penuh seluruh data lokal ke cloud
    if (action === 'syncAll') {
      var data = payload.data || {};
      if (data.transactions) overwriteSheet_('Transactions', data.transactions);
      if (data.accounts) overwriteSheet_('Accounts', data.accounts);
      if (data.savings) overwriteSheet_('Savings', data.savings);
      if (data.categories) overwriteSheet_('Categories', data.categories);
      if (data.investments) overwriteSheet_('Investments', data.investments);

      return json_({
        success: true,
        message: 'Full sync berhasil disinkronkan ke Spreadsheet',
        synced_at: new Date().toISOString()
      });
    }

    return json_({ success: false, message: 'Action tidak dikenali: ' + action });
  } catch (error) {
    return json_({ success: false, message: 'POST Error: ' + error.message });
  }
}

/**
 * Memastikan semua sheet dan header tersedia
 */
function ensureSetup_() {
  var spreadsheet = getSs_();
  Object.keys(SCHEMAS).forEach(function(name) {
    var sheet = spreadsheet.getSheetByName(name);
    if (!sheet) {
      sheet = spreadsheet.insertSheet(name);
      var headers = SCHEMAS[name];
      sheet.getRange(1, 1, 1, headers.length).setValues([headers]).setFontWeight('bold');
      sheet.setFrozenRows(1);
    }
  });
}

/**
 * Membaca sheet menjadi array of object
 */
function readSheet_(sheetName) {
  var sheet = getSs_().getSheetByName(sheetName);
  if (!sheet || sheet.getLastRow() < 2) return [];

  var values = sheet.getDataRange().getValues();
  var headers = values.shift().map(String);
  
  return values.filter(function(row) {
    return row.some(function(val) { return val !== ''; });
  }).map(function(row) {
    return headers.reduce(function(item, header, index) {
      item[header] = row[index];
      return item;
    }, {});
  });
}

/**
 * Menimpa isi sheet secara batch untuk Full Sync
 */
function overwriteSheet_(sheetName, items) {
  var sheet = getSs_().getSheetByName(sheetName);
  var headers = SCHEMAS[sheetName];
  
  sheet.clearContents();
  sheet.getRange(1, 1, 1, headers.length).setValues([headers]).setFontWeight('bold');
  sheet.setFrozenRows(1);

  if (!items || items.length === 0) return;

  var rows = items.map(function(item) {
    return headers.map(function(h) {
      return item[h] !== undefined ? item[h] : '';
    });
  });

  sheet.getRange(2, 1, rows.length, headers.length).setValues(rows);
}

/**
 * Mengumpulkan seluruh tabel untuk respons fetch
 */
function buildFetchPayload_() {
  return {
    transactions: readSheet_('Transactions'),
    accounts: readSheet_('Accounts'),
    savings: readSheet_('Savings'),
    categories: readSheet_('Categories'),
    investments: readSheet_('Investments')
  };
}

/**
 * Helper pembungkus output JSON
 */
function json_(payload) {
  return ContentService
    .createTextOutput(JSON.stringify(payload))
    .setMimeType(ContentService.MimeType.JSON);
}

/**
 * Ambil instance Spreadsheet aktif
 */
function getSs_() {
  var ss = SpreadsheetApp.getActiveSpreadsheet();
  if (!ss) {
    throw new Error('Script tidak terikat ke spreadsheet. Pastikan dibuka melalui Extensions > Apps Script.');
  }
  return ss;
}

/**
 * Ekspos fungsi ke globalThis untuk V8 runtime compatibility
 */
globalThis.handleGet_ = handleGet_;
globalThis.handlePost_ = handlePost_;
globalThis.runSetup_ = ensureSetup_;
