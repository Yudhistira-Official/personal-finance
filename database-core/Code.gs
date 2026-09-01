/**
 * PERSONAL FINANCE - CORE ENGINE (Optimized)
 * Dijalankan via Bootstrapper Loader dari GitHub.
 */

var SCHEMAS = {
  Transactions: ['id', 'date', 'type', 'account_id', 'category_id', 'amount', 'note', 'updated_at'],
  Accounts: ['id', 'name', 'account_type', 'balance', 'is_active'],
  Savings: ['id', 'name', 'target_amount', 'current_amount', 'linked_account_id'],
  Categories: ['id', 'name', 'type', 'icon', 'color']
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
      var existingIds = readSheet_('Transactions').reduce(function(ids, row) {
        if (row.id) ids[String(row.id)] = true;
        return ids;
      }, {});

      var newRows = [];
      var nowIso = new Date().toISOString();

      payload.transactions.forEach(function(tx) {
        var id = String(tx.id || '').trim();
        if (!id || existingIds[id]) return;

        newRows.push([
          id,
          tx.date || 0,
          tx.type || 'expense',
          String(tx.account_id || ''),
          String(tx.category_id || ''),
          tx.amount || 0,
          String(tx.note || ''),
          nowIso
        ]);
        existingIds[id] = true;
      });

      // Tulis seluruh baris sekaligus dalam satu operasi I/O
      if (newRows.length > 0) {
        var startRow = sheet.getLastRow() + 1;
        sheet.getRange(startRow, 1, newRows.length, SCHEMAS.Transactions.length).setValues(newRows);
      }

      return json_({
        success: true,
        message: newRows.length + ' transaksi baru berhasil disimpan',
        pushed: newRows.length
      });
    }

    // 2. Action SYNC_ALL: Menimpa / sinkronisasi penuh seluruh data lokal ke cloud
    if (action === 'syncAll') {
      var data = payload.data || {};
      if (data.transactions) overwriteSheet_('Transactions', data.transactions);
      if (data.accounts) overwriteSheet_('Accounts', data.accounts);
      if (data.savings) overwriteSheet_('Savings', data.savings);
      if (data.categories) overwriteSheet_('Categories', data.categories);

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
    categories: readSheet_('Categories')
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
