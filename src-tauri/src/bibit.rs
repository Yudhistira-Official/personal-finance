use crate::models::MutualFundProduct;
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;

const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36";

pub struct BibitClient {
    client: reqwest::Client,
}

impl BibitClient {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(12))
            .user_agent(USER_AGENT)
            .build()
            .unwrap_or_default();
        Self { client }
    }

    /// Ambil satu halaman katalog: request + decrypt + extract array mentah.
    ///
    /// Err hanya untuk kegagalan transport/parse/decrypt. HTTP non-2xx dan
    /// shape tak dikenal dikembalikan sebagai `None` agar pemanggil bisa
    /// memutuskan (fallback page size atau hentikan pagination). `name_filter`
    /// dikirim sebagai param `name` untuk server-side search (string kosong =
    /// tanpa filter, dipakai fetch_catalog).
    async fn fetch_catalog_page(
        &self,
        page_size: i64,
        page: i64,
        name_filter: &str,
    ) -> Result<Option<Vec<Value>>, String> {
        let resp = self
            .client
            .get("https://api.bibit.id/products/list")
            .query(&[
                ("limit", page_size.to_string()),
                ("page", page.to_string()),
                ("name", name_filter.to_string()),
                ("sort_by", "7".to_string()),
            ])
            .header(reqwest::header::ACCEPT, "application/json")
            .header(reqwest::header::USER_AGENT, USER_AGENT)
            .header(reqwest::header::ORIGIN, "https://bibit.id")
            .send()
            .await
            .map_err(|e| format!("fetch_catalog request page {page}: {e}"))?;

        if !resp.status().is_success() {
            return Ok(None);
        }

        let body: Value = resp
            .json()
            .await
            .map_err(|e| format!("fetch_catalog parse json page {page}: {e}"))?;

        // Bibit returns catalog payload as encrypted string under data.data or data.
        let catalog_body = match body.get("data") {
            Some(Value::Object(data_obj)) => {
                if let Some(Value::String(encrypted)) = data_obj.get("data") {
                    decrypt_bibit(encrypted).map_err(|e| format!("fetch_catalog decrypt: {e}"))?
                } else {
                    body.clone()
                }
            }
            Some(Value::String(encrypted)) => {
                decrypt_bibit(encrypted).map_err(|e| format!("fetch_catalog decrypt: {e}"))?
            }
            _ => body.clone(),
        };

        Ok(extract_catalog_array(&catalog_body))
    }

    /// GET https://api.bibit.id/products/list — paginated, mengambil SEMUA produk.
    ///
    /// Fetches and decrypts live catalogue data before mapping products.
    /// Returns mapped products or an explicit HTTP, JSON, or decryption error.
    pub async fn fetch_catalog(&self) -> Result<Vec<MutualFundProduct>, String> {
        let mut all: Vec<Value> = Vec::new();
        // page_size 100; fallback 50 bila API menolak (di bawah request).
        let mut page_size: i64 = 100;
        let mut page: i64 = 1;
        let mut page1_ok = false;

        // Cap 40 halaman * 100 = 4000 produk. Verifikasi live (2026-09): katalog
        // berisi 2943 produk = 30 halaman @ limit=100, jadi cap 20 (2000) lama
        // memangkas ~943 produk. Cap 40 memberi ruang pertumbuhan katalog.
        // Selesai normal saat halaman terakhir mengembalikan < page_size item.
        while page <= 40 {
            let items = match self.fetch_catalog_page(page_size, page, "").await? {
                Some(items) => items,
                None => {
                    if page == 1 && page_size == 100 {
                        // Halaman pertama ditolak dengan limit 100 → coba limit 50.
                        page_size = 50;
                        continue;
                    }
                    break;
                }
            };

            if page == 1 {
                if page_size == 100 && (items.len() as i64) < page_size {
                    // Respons < request limit → API memotong di 50; ulangi dengan 50.
                    page_size = 50;
                    continue;
                }
                page1_ok = true;
            }

            if items.is_empty() {
                break;
            }
            let fetched = items.len() as i64;
            all.extend(items);
            if fetched < page_size {
                break; // halaman terakhir
            }
            page += 1;
        }

        if !page1_ok || all.is_empty() {
            return Err("fetch_catalog: katalog kosong / respons tak dikenali".to_string());
        }

        let now = chrono::Utc::now().timestamp();

        let mut out = Vec::with_capacity(all.len());
        for item in all {
            out.push(map_product(&item, now));
        }

        Ok(out)
    }

    /// Server-side pencarian via param `name` API Bibit (terverifikasi live:
    /// `name=danareksa` → 90 hasil). Satu halaman (page=1, limit=50) cukup
    /// untuk dropdown picker. Hasil boleh kosong tanpa error — kosong berarti
    /// API tidak punya kecocokan, bukan kegagalan.
    pub async fn search_remote(&self, query: &str) -> Result<Vec<MutualFundProduct>, String> {
        let items = match self.fetch_catalog_page(50, 1, query).await? {
            Some(items) => items,
            // Non-2xx / shape tak dikenal: perlakukan sebagai tanpa hasil.
            None => return Ok(Vec::new()),
        };
        let now = chrono::Utc::now().timestamp();
        let mut out: Vec<MutualFundProduct> =
            items.iter().map(|item| map_product(item, now)).collect();
        // Urutkan by name agar konsisten dengan pencarian lokal (ORDER BY name).
        out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        Ok(out)
    }

    /// Fetches current NAVs only for requested products via one-page name searches.
    ///
    /// Missing products are omitted so stale cache values remain untouched.
    pub async fn fetch_nav_batch(
        &self,
        products_by_id: &HashMap<String, String>,
    ) -> Result<HashMap<String, f64>, String> {
        let mut navs = HashMap::new();
        for (product_id, product_name) in products_by_id {
            // Bibit searches by name, not product ID; use a compact name fragment.
            let query = product_name
                .split_whitespace()
                .take(2)
                .collect::<Vec<_>>()
                .join(" ");
            let query = if query.is_empty() {
                product_name.chars().take(15).collect::<String>()
            } else {
                query.chars().take(15).collect::<String>()
            };
            if query.is_empty() {
                // No cached name → no usable query; keep the stale NAV untouched.
                continue;
            }
            if let Some(product) = self
                .search_remote(&query)
                .await?
                .into_iter()
                .find(|product| product.id == *product_id)
            {
                if product.current_nav > 0.0 {
                    navs.insert(product_id.clone(), product.current_nav);
                }
            }
        }
        Ok(navs)
    }
}

/// Mapping item→MutualFundProduct, dipakai fetch_catalog & search_remote.
///
/// Diverifikasi live: type/sharia/minbuy flat, sedangkan
/// investment_manager/nav/aum nested object, return di changesvalue.
fn map_product(item: &Value, now: i64) -> MutualFundProduct {
    MutualFundProduct {
        id: as_text(item, "id").unwrap_or_default(),
        name: as_text(item, "name").unwrap_or_default(),
        fund_type: as_text(item, "type")
            .or_else(|| as_text(item, "fund_type"))
            .unwrap_or_default(),
        manager_name: item
            .get("investment_manager")
            .and_then(|v| v.get("name"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| as_text(item, "investment_manager_name"))
            .or_else(|| as_text(item, "manager"))
            .unwrap_or_default(),
        is_syariah: as_bool(item, "sharia")
            .or_else(|| as_bool(item, "is_syariah"))
            .unwrap_or(false),
        current_nav: item
            .get("nav")
            .and_then(|v| v.get("value"))
            .and_then(|v| v.as_f64())
            .or_else(|| as_f64(item, "nav"))
            .unwrap_or(0.0),
        return_1d: item
            .get("changesvalue")
            .and_then(|v| v.get("1d"))
            .and_then(|v| v.as_f64())
            .or_else(|| as_f64(item, "return_1d")),
        return_1y: item
            .get("changesvalue")
            .and_then(|v| v.get("1y"))
            .and_then(|v| v.as_f64())
            .or_else(|| as_f64(item, "return_1y")),
        aum: item
            .get("aum")
            .and_then(|v| v.get("value"))
            .and_then(|v| v.as_f64())
            .or_else(|| as_f64(item, "aum")),
        min_buy: as_i64(item, "minbuy")
            .or_else(|| as_i64(item, "minimum_buy"))
            .unwrap_or(0),
        last_fetched_at: now,
    }
}

impl BibitClient {
    /// Membaca NAB terkini satu produk via katalog (endpoint detail 422).
    ///
    /// `GET /products/{id}` tidak valid di API live, jadi NAB dicari dari
    /// `fetch_catalog()` yang sudah ter-decrypt dan ter-map dengan benar.
    #[allow(dead_code)]
    pub async fn fetch_single_nav(&self, product_id: &str) -> Result<f64, String> {
        let catalog = self.fetch_catalog().await?;
        catalog
            .into_iter()
            .find(|p| p.id == product_id)
            .map(|p| p.current_nav)
            // NAV 0 berarti belum ada data — perlakukan sebagai gagal, bukan nilai valid.
            .filter(|v| *v > 0.0)
            .ok_or_else(|| format!("NAB untuk product {product_id} tidak ditemukan"))
    }
}

/// Decrypts Bibit's `[iv][ciphertext][key]` payload using AES-256-CBC and PKCS7.
fn decrypt_bibit(payload: &str) -> Result<Value, String> {
    // Validate fixed-width IV and key segments before decoding untrusted input.
    if payload.len() < 64 {
        return Err("payload terlalu pendek".into());
    }
    let iv_hex = &payload[..32];
    let key_str = &payload[payload.len() - 32..];
    let ct_hex = &payload[32..payload.len() - 32];
    let iv = hex::decode(iv_hex).map_err(|e| format!("iv hex: {e}"))?;
    let key = key_str.as_bytes();
    let ct = hex::decode(ct_hex).map_err(|e| format!("ct hex: {e}"))?;

    // Match CryptoJS AES.decrypt defaults: CBC mode with PKCS7 padding.
    use aes::cipher::{BlockDecryptMut, KeyIvInit};
    type Aes256CbcDec = cbc::Decryptor<aes::Aes256>;
    let decryptor =
        Aes256CbcDec::new_from_slices(key, &iv).map_err(|e| format!("cipher init: {e}"))?;
    let mut buf = ct;
    let pt = decryptor
        .decrypt_padded_mut::<cbc::cipher::block_padding::Pkcs7>(&mut buf)
        .map_err(|e| format!("decrypt: {e}"))?;
    let text = String::from_utf8(pt.to_vec()).map_err(|e| format!("utf8: {e}"))?;
    serde_json::from_str(&text).map_err(|e| format!("json: {e}"))
}

/// Extracts product arrays from plaintext, decrypted, and wrapped response shapes.
fn extract_catalog_array(body: &Value) -> Option<Vec<Value>> {
    // Decrypted responses may be arrays or objects with products/data fields.
    if let Some(arr) = body.as_array() {
        return Some(arr.clone());
    }
    if let Some(arr) = body.get("products").and_then(Value::as_array) {
        return Some(arr.clone());
    }
    if let Some(arr) = body.get("data").and_then(Value::as_array) {
        return Some(arr.clone());
    }

    // Plain API wrappers keep arrays below their data object or JSON string.
    if let Some(data) = body.get("data") {
        if let Some(arr) = data.get("products").and_then(Value::as_array) {
            return Some(arr.clone());
        }
        if let Some(arr) = data.get("data").and_then(Value::as_array) {
            return Some(arr.clone());
        }
        if let Some(s) = data.as_str() {
            if let Ok(v) = serde_json::from_str::<Value>(s) {
                return extract_catalog_array(&v);
            }
        }
    }
    None
}

// ── Helper ekstraksi longgar ─────────────────────────────────────────────────
// Field Bibit tidak konsisten tipenya antar endpoint: id/type kadang number,
// nav/return kadang string. Helper ini koersi ke tipe target tanpa panik.

fn as_text(item: &Value, key: &str) -> Option<String> {
    match item.get(key)? {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

fn as_f64(item: &Value, key: &str) -> Option<f64> {
    match item.get(key)? {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.parse().ok(),
        _ => None,
    }
}

fn as_i64(item: &Value, key: &str) -> Option<i64> {
    match item.get(key)? {
        Value::Number(n) => n.as_i64().or_else(|| n.as_f64().map(|f| f as i64)),
        Value::String(s) => s
            .parse()
            .ok()
            .or_else(|| s.parse::<f64>().ok().map(|f| f as i64)),
        _ => None,
    }
}

fn as_bool(item: &Value, key: &str) -> Option<bool> {
    match item.get(key)? {
        Value::Bool(b) => Some(*b),
        Value::Number(n) => n.as_i64().map(|i| i != 0),
        Value::String(s) => match s.as_str() {
            "1" | "true" | "TRUE" => Some(true),
            "0" | "false" | "FALSE" => Some(false),
            _ => None,
        },
        _ => None,
    }
}
