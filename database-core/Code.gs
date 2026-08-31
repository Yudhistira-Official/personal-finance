/**
 * Kode inti diambil otomatis loader dari GitHub; jangan ditempel manual di Apps Script.
 * Menangani GET untuk health check dan pengambilan data sinkronisasi.
 * @param {GoogleAppsScript.Events.DoGet} e Event GET dari Web App.
 * @return {GoogleAppsScript.Content.TextOutput} Respons JSON.
 */
function handleGet_(e) {
  // Tanpa action, kembalikan health check ringan untuk sync_test.
  var action = e && e.parameter ? e.parameter.action : '';
  if (action === 'fetch') {
    return json_(buildFetchPayload_());
  }

  return json_({
    success: true,
    message: 'Database Core aktif',
    sheets: ['Transactions', 'Accounts', 'Savings', 'Categories']
  });
}

/**
 * Menangani POST push transaksi dari aplikasi desktop.
 * @param {GoogleAppsScript.Events.DoPost} e Event POST dari Web App.
 * @return {GoogleAppsScript.Content.TextOutput} Respons JSON.
 */
function handlePost_(e) {
  // Tolak request tanpa body atau action agar endpoint tidak menulis data ambigu.
  if (!e || !e.postData || !e.postData.contents) {
    return json_({ success: false, message: 'Body JSON wajib diisi', pushed: 0 });
  }

  try {
    var payload = JSON.parse(e.postData.contents);
    if (payload.action !== 'push' || !Array.isArray(payload.transactions)) {
      return json_({ success: false, message: 'Action push dan transactions wajib diisi', pushed: 0 });
    }

    var sheet = SpreadsheetApp.getActiveSpreadsheet().getSheetByName('Transactions');
    if (!sheet) {
      return json_({ success: false, message: 'Sheet Transactions belum dibuat', pushed: 0 });
    }

    var existingIds = readSheet_('Transactions').reduce(function(ids, row) {
      if (row.id) ids[row.id] = true;
      return ids;
    }, {});
    var pushed = 0;

    // Simpan hanya transaksi valid baru; ID menjadi kunci idempotensi push.
    payload.transactions.forEach(function(transaction) {
      var id = String(transaction.id || '').trim();
      if (!id || existingIds[id]) return;

      appendRow_('Transactions', [
        id,
        transaction.date || 0,
        transaction.type || 'expense',
        String(transaction.account_id || ''),
        String(transaction.category_id || ''),
        transaction.amount || 0,
        String(transaction.note || ''),
        new Date().toISOString()
      ]);
      existingIds[id] = true;
      pushed++;
    });

    return json_({ success: true, message: pushed + ' transaksi berhasil disimpan', pushed: pushed });
  } catch (error) {
    return json_({ success: false, message: 'JSON tidak valid: ' + error.message, pushed: 0 });
  }
}

/**
 * Membuat empat sheet standar beserta header tebal bila belum tersedia.
 * @return {void}
 */
function runSetup_() {
  var spreadsheet = SpreadsheetApp.getActiveSpreadsheet();
  var schemas = {
    Transactions: ['id', 'date', 'type', 'account_id', 'category_id', 'amount', 'note', 'updated_at'],
    Accounts: ['id', 'name', 'account_type', 'balance', 'is_active'],
    Savings: ['id', 'name', 'target_amount', 'current_amount', 'linked_account_id'],
    Categories: ['id', 'name', 'type', 'icon', 'color']
  };

  // Buat sheet yang hilang, lalu pastikan baris pertama selalu menjadi header standar.
  Object.keys(schemas).forEach(function(name) {
    var sheet = spreadsheet.getSheetByName(name) || spreadsheet.insertSheet(name);
    var headers = schemas[name];
    sheet.getRange(1, 1, 1, headers.length).setValues([headers]).setFontWeight('bold');
    sheet.setFrozenRows(1);
  });
}

/**
 * Membaca sheet menjadi array object berdasarkan header baris pertama.
 * @param {string} sheetName Nama sheet yang dibaca.
 * @return {Object[]} Baris data sebagai object.
 */
function readSheet_(sheetName) {
  var sheet = SpreadsheetApp.getActiveSpreadsheet().getSheetByName(sheetName);
  if (!sheet || sheet.getLastRow() < 2) return [];

  var values = sheet.getDataRange().getValues();
  var headers = values.shift().map(String);
  return values.filter(function(row) {
    return row.some(function(value) { return value !== ''; });
  }).map(function(row) {
    return headers.reduce(function(item, header, index) {
      item[header] = row[index];
      return item;
    }, {});
  });
}

/**
 * Menambahkan satu baris ke sheet berdasarkan nama sheet dan urutan kolomnya.
 * @param {string} sheetName Nama sheet tujuan.
 * @param {Array} values Nilai kolom sesuai header sheet.
 * @return {void}
 */
function appendRow_(sheetName, values) {
  var sheet = SpreadsheetApp.getActiveSpreadsheet().getSheetByName(sheetName);
  if (!sheet) throw new Error('Sheet ' + sheetName + ' belum dibuat');
  sheet.appendRow(values);
}

/**
 * Menggabungkan seluruh data sheet untuk respons fetch.
 * @return {Object} Payload transaksi dan master data.
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
 * Mengubah object menjadi output JSON ContentService.
 * @param {Object} payload Data yang dikirim ke client.
 * @return {GoogleAppsScript.Content.TextOutput} Output JSON UTF-8.
 */
function json_(payload) {
  return ContentService
    .createTextOutput(JSON.stringify(payload))
    .setMimeType(ContentService.MimeType.JSON);
}
