use argon2::{
    password_hash::{
        rand_core::{OsRng, RngCore},
        PasswordHash, PasswordHasher, PasswordVerifier, SaltString
    },
    Argon2
};
use rusqlite::params;
use tauri::{State, Manager};
use crate::db::DbState;
use base64::prelude::*;
use std::process::Command;

// Database Structures
#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct Drug {
    pub id: i64,
    pub barcode: String,
    pub name: String,
    pub requires_prescription: bool,
    pub price_per_item_da: f64,
    pub cost_price_da: f64,
    pub items_per_package: i32,
    pub tva: f64,
    pub mg: f64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct StockBatch {
    pub id: i64,
    pub drug_id: i64,
    pub batch_number: String,
    pub expiry_date: String,
    pub packages_received: i32,
    pub items_remaining: i32,
    pub price_per_item_da: f64,
    pub cost_price_da: f64,
    pub tva: f64,
    pub mg: f64,
    pub received_at: String,
    pub invoice_id: Option<i64>,
    pub invoice_number: Option<String>,
    pub supplier_name: Option<String>,
    pub invoice_date: Option<String>,
}

pub fn generate_unique_6digit_barcode(tx: &rusqlite::Transaction) -> Result<String, String> {
    for _ in 0..10_000 {
        let code_num = 100_000 + (OsRng.next_u32() % 900_000);
        let code = format!("{:06}", code_num);
        let exists: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM drugs WHERE barcode = ?1);",
            params![code],
            |row| row.get(0),
        ).unwrap_or(false);
        if !exists {
            return Ok(code);
        }
    }
    Err("Impossible de générer un code-barres unique à 6 chiffres".into())
}

pub fn generate_unique_6digit_barcode_conn(conn: &rusqlite::Connection) -> Result<String, String> {
    for _ in 0..10_000 {
        let code_num = 100_000 + (OsRng.next_u32() % 900_000);
        let code = format!("{:06}", code_num);
        let exists: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM drugs WHERE barcode = ?1);",
            params![code],
            |row| row.get(0),
        ).unwrap_or(false);
        if !exists {
            return Ok(code);
        }
    }
    Err("Impossible de générer un code-barres unique à 6 chiffres".into())
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct AppSettings {
    pub expiry_warning_days: i32,
    pub default_language: String,
    pub last_backup_at: Option<String>,
    #[serde(default)]
    pub cashier_permissions: Option<String>,
    #[serde(default)]
    pub pharmacy_name: Option<String>,
    #[serde(default)]
    pub pharmacy_address: Option<String>,
    #[serde(default)]
    pub direct_ai_invoice: Option<bool>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct Customer {
    pub id: i64,
    pub name: String,
    pub birth_date: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct InvoiceItem {
    pub drug_name: String,
    pub quantity_items: i32,
    pub unit_price_da: f64,
    pub line_total_da: f64,
    pub cost_price_da: Option<f64>, // Hidden for Cashiers
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct LastInvoice {
    pub id: i64,
    pub prescribing_doctor_name: Option<String>,
    pub patient_name: Option<String>,
    pub patient_birth_date: Option<String>,
    pub treatment_period_days: Option<i32>,
    pub total_da: f64,
    pub created_at: String,
    pub items: Vec<InvoiceItem>,
    #[serde(default)]
    pub is_loan: Option<bool>,
    #[serde(default)]
    pub loan_status: Option<String>,
    #[serde(default)]
    pub loan_settled_at: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct CartItemInput {
    pub drug_id: i64,
    pub quantity_items: i32,
    pub unit_price_da: f64,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct ActiveLoanItem {
    pub sale_id: i64,
    pub sale_date: String,
    pub drug_id: i64,
    pub drug_name: String,
    pub quantity: i32,
    pub unit_price_da: f64,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct LoanDeductionInput {
    pub loan_sale_id: i64,
    pub drug_id: i64,
    pub quantity: i32,
}

// PIN Security Commands
#[tauri::command]
pub fn is_pin_configured(state: State<'_, DbState>) -> Result<bool, String> {
    let conn = state.conn.lock().map_err(|_| "Failed to lock database connection")?;
    
    let hash: Option<String> = conn.query_row(
        "SELECT admin_pin_hash FROM settings WHERE id = 1;",
        [],
        |row| row.get(0),
    ).map_err(|e| format!("Database query error: {}", e))?;

    Ok(hash.is_some() && !hash.unwrap().trim().is_empty())
}

#[tauri::command]
pub fn configure_pin(state: State<'_, DbState>, pin: String) -> Result<(), String> {
    if pin.len() < 4 {
        return Err("PIN must be at least 4 digits".into());
    }

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(pin.as_bytes(), &salt)
        .map_err(|e| format!("Hashing failed: {}", e))?
        .to_string();

    let conn = state.conn.lock().map_err(|_| "Failed to lock database connection")?;
    
    conn.execute(
        "UPDATE settings SET admin_pin_hash = ?1 WHERE id = 1;",
        params![password_hash],
    ).map_err(|e| format!("Failed to save PIN: {}", e))?;

    Ok(())
}

#[tauri::command]
pub fn verify_pin(state: State<'_, DbState>, pin: String) -> Result<bool, String> {
    let conn = state.conn.lock().map_err(|_| "Failed to lock database connection")?;
    
    let hash_opt: Option<String> = conn.query_row(
        "SELECT admin_pin_hash FROM settings WHERE id = 1;",
        [],
        |row| row.get(0),
    ).map_err(|e| format!("Database query error: {}", e))?;

    let hash_str = match hash_opt {
        Some(h) if !h.trim().is_empty() => h,
        _ => return Err("No Admin PIN configured yet".into()),
    };

    let parsed_hash = PasswordHash::new(&hash_str)
        .map_err(|e| format!("Invalid hash format: {}", e))?;

    let is_valid = Argon2::default()
        .verify_password(pin.as_bytes(), &parsed_hash)
        .is_ok();

    Ok(is_valid)
}

// App Settings Commands
#[tauri::command]
pub fn get_settings(state: State<'_, DbState>) -> Result<AppSettings, String> {
    let conn = state.conn.lock().map_err(|_| "Failed to lock database connection")?;
    
    let settings = conn.query_row(
        "SELECT expiry_warning_days, default_language, last_backup_at, COALESCE(cashier_permissions, '{}'), COALESCE(pharmacy_name, ''), COALESCE(pharmacy_address, ''), COALESCE(direct_ai_invoice, 0) FROM settings WHERE id = 1;",
        [],
        |row| Ok(AppSettings {
            expiry_warning_days: row.get(0)?,
            default_language: row.get(1)?,
            last_backup_at: row.get(2)?,
            cashier_permissions: Some(row.get(3)?),
            pharmacy_name: Some(row.get(4)?),
            pharmacy_address: Some(row.get(5)?),
            direct_ai_invoice: Some(row.get::<_, i32>(6)? == 1),
        })
    ).map_err(|e| format!("Failed to fetch settings: {}", e))?;
    
    Ok(settings)
}

#[tauri::command]
pub fn update_settings(
    state: State<'_, DbState>,
    expiry_warning_days: i32,
    default_language: String,
    cashier_permissions: Option<String>,
    pharmacy_name: Option<String>,
    pharmacy_address: Option<String>,
    direct_ai_invoice: Option<bool>,
) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|_| "Failed to lock database connection")?;
    
    if let Some(ref perms) = cashier_permissions {
        conn.execute(
            "UPDATE settings SET cashier_permissions = ?1 WHERE id = 1;",
            params![perms],
        ).map_err(|e| format!("Failed to update permissions: {}", e))?;
    }

    if let Some(ref name) = pharmacy_name {
        conn.execute(
            "UPDATE settings SET pharmacy_name = ?1 WHERE id = 1;",
            params![name],
        ).map_err(|e| format!("Failed to update pharmacy name: {}", e))?;
    }

    if let Some(ref addr) = pharmacy_address {
        conn.execute(
            "UPDATE settings SET pharmacy_address = ?1 WHERE id = 1;",
            params![addr],
        ).map_err(|e| format!("Failed to update pharmacy address: {}", e))?;
    }

    if let Some(direct_ai) = direct_ai_invoice {
        conn.execute(
            "UPDATE settings SET direct_ai_invoice = ?1 WHERE id = 1;",
            params![if direct_ai { 1 } else { 0 }],
        ).map_err(|e| format!("Failed to update direct AI invoice setting: {}", e))?;
    }

    conn.execute(
        "UPDATE settings SET expiry_warning_days = ?1, default_language = ?2 WHERE id = 1;",
        params![expiry_warning_days, default_language],
    ).map_err(|e| format!("Failed to update settings: {}", e))?;

    Ok(())
}

// Stock Intake Commands
#[tauri::command]
pub fn get_drug_by_barcode(state: State<'_, DbState>, barcode: String) -> Result<Option<Drug>, String> {
    let conn = state.conn.lock().map_err(|_| "Failed to lock database connection")?;
    
    let res = conn.query_row(
        "SELECT id, barcode, name, requires_prescription, price_per_item_da, cost_price_da, items_per_package, tva, mg, created_at, updated_at 
         FROM drugs WHERE barcode = ?1;",
        params![barcode],
        |row| {
            Ok(Drug {
                id: row.get(0)?,
                barcode: row.get(1)?,
                name: row.get(2)?,
                requires_prescription: row.get::<usize, i32>(3)? != 0,
                price_per_item_da: row.get(4)?,
                cost_price_da: row.get(5)?,
                items_per_package: row.get(6)?,
                tva: row.get(7)?,
                mg: row.get(8)?,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
            })
        }
    );

    match res {
        Ok(drug) => Ok(Some(drug)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub fn generate_random_barcode(state: State<'_, DbState>) -> Result<String, String> {
    let conn = state.conn.lock().map_err(|_| "Failed to lock database connection")?;
    generate_unique_6digit_barcode_conn(&conn)
}

#[tauri::command]
pub fn get_stock_batches(state: State<'_, DbState>, drug_id: i64) -> Result<Vec<StockBatch>, String> {
    let conn = state.conn.lock().map_err(|_| "Failed to lock database connection")?;
    
    let mut stmt = conn.prepare(
        "SELECT 
            sb.id, sb.drug_id, sb.batch_number, sb.expiry_date, sb.packages_received, 
            sb.items_remaining, sb.price_per_item_da, sb.cost_price_da, sb.tva, sb.mg, sb.received_at,
            sb.invoice_id,
            inv.invoice_number,
            inv.supplier_name,
            inv.invoice_date
         FROM stock_batches sb
         LEFT JOIN imported_invoices inv ON inv.id = sb.invoice_id
         WHERE sb.drug_id = ?1 
         ORDER BY sb.expiry_date ASC;"
    ).map_err(|e| e.to_string())?;
    
    let rows = stmt.query_map(params![drug_id], |row| {
        Ok(StockBatch {
            id: row.get(0)?,
            drug_id: row.get(1)?,
            batch_number: row.get(2)?,
            expiry_date: row.get(3)?,
            packages_received: row.get(4)?,
            items_remaining: row.get(5)?,
            price_per_item_da: row.get(6)?,
            cost_price_da: row.get(7)?,
            tva: row.get(8)?,
            mg: row.get(9)?,
            received_at: row.get(10)?,
            invoice_id: row.get(11)?,
            invoice_number: row.get(12)?,
            supplier_name: row.get(13)?,
            invoice_date: row.get(14)?,
        })
    }).map_err(|e| e.to_string())?;

    let mut list = Vec::new();
    for r in rows {
        list.push(r.map_err(|e| e.to_string())?);
    }
    Ok(list)
}

#[tauri::command]
pub fn add_drug_with_batch(
    state: State<'_, DbState>,
    barcode: Option<String>,
    name: String,
    requires_prescription: bool,
    price_per_item_da: f64,
    cost_price_da: f64,
    items_per_package: i32,
    expiry_date: String,
    packages_received: i32,
    batch_number: Option<String>,
    tva: Option<f64>,
    mg: Option<f64>,
) -> Result<(), String> {
    if packages_received > 0 {
        let today = chrono::Local::now().naive_local().date();
        let tomorrow = today + chrono::Duration::days(1);
        if let Ok(parsed_expiry) = chrono::NaiveDate::parse_from_str(&expiry_date, "%Y-%m-%d") {
            if parsed_expiry < tomorrow {
                return Err("Expiration date must be at least tomorrow".into());
            }
        } else {
            return Err("Invalid expiration date format".into());
        }
    }

    let final_tva = tva.unwrap_or(9.0);
    let final_mg = mg.unwrap_or_else(|| {
        if cost_price_da > 0.0 {
            let cost_ttc = cost_price_da * (1.0 + final_tva / 100.0);
            if cost_ttc > 0.0 {
                ((price_per_item_da - cost_ttc) / cost_ttc) * 100.0
            } else {
                0.0
            }
        } else {
            0.0
        }
    });

    let final_batch_num = match batch_number {
        Some(b) if !b.trim().is_empty() => b.trim().to_string(),
        _ => "LOT-01".to_string(),
    };

    let mut conn = state.conn.lock().map_err(|_| "Failed to lock database connection")?;
    
    let tx = conn.transaction().map_err(|e| format!("Failed to start transaction: {}", e))?;

    // Check if drug already exists
    let barcode_clean = barcode.unwrap_or_default().trim().to_string();
    
    let existing_drug_id: Option<i64> = if !barcode_clean.is_empty() {
        tx.query_row(
            "SELECT id FROM drugs WHERE barcode = ?1;",
            params![barcode_clean],
            |row| row.get(0)
        ).ok()
    } else {
        tx.query_row(
            "SELECT id FROM drugs WHERE LOWER(TRIM(name)) = LOWER(TRIM(?1));",
            params![name],
            |row| row.get(0)
        ).ok()
    };

    let actual_drug_id = match existing_drug_id {
        Some(id) => {
            // Update drug details
            tx.execute(
                "UPDATE drugs SET name = ?1, requires_prescription = ?2, price_per_item_da = ?3, cost_price_da = ?4, items_per_package = ?5, tva = ?6, mg = ?7, updated_at = datetime('now', 'localtime') WHERE id = ?8;",
                params![name.trim(), if requires_prescription { 1 } else { 0 }, price_per_item_da, cost_price_da, items_per_package, final_tva, final_mg, id]
            ).map_err(|e| format!("Failed to update drug: {}", e))?;
            id
        }
        None => {
            // Insert new drug: generate random unique 6-digit barcode if none provided
            let final_barcode = if !barcode_clean.is_empty() {
                barcode_clean
            } else {
                generate_unique_6digit_barcode(&tx)?
            };

            tx.execute(
                "INSERT INTO drugs (barcode, name, requires_prescription, price_per_item_da, cost_price_da, items_per_package, tva, mg) 
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8);",
                params![final_barcode, name.trim(), if requires_prescription { 1 } else { 0 }, price_per_item_da, cost_price_da, items_per_package, final_tva, final_mg]
            ).map_err(|e| format!("Failed to insert drug: {}", e))?;
            tx.last_insert_rowid()
        }
    };

    // Calculate items_remaining
    let items_remaining = packages_received * items_per_package;

    // Insert stock batch if packages_received > 0
    if packages_received > 0 {
        tx.execute(
            "INSERT INTO stock_batches (drug_id, batch_number, expiry_date, packages_received, items_remaining, price_per_item_da, cost_price_da, tva, mg) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9);",
            params![actual_drug_id, final_batch_num, expiry_date, packages_received, items_remaining, price_per_item_da, cost_price_da, final_tva, final_mg]
        ).map_err(|e| format!("Failed to insert stock batch: {}", e))?;
    }

    tx.commit().map_err(|e| format!("Transaction commit failed: {}", e))?;
    Ok(())
}

// POS Sales Commands
#[tauri::command]
pub fn search_customers(state: State<'_, DbState>, query: String) -> Result<Vec<Customer>, String> {
    let conn = state.conn.lock().map_err(|_| "Failed to lock database connection")?;
    
    let mut stmt = conn.prepare(
        "SELECT id, name, birth_date, created_at, updated_at FROM customers WHERE name LIKE ?1 ORDER BY name ASC LIMIT 10;"
    ).map_err(|e| e.to_string())?;
    
    let search_pattern = format!("%{}%", query);
    let rows = stmt.query_map(params![search_pattern], |row| {
        Ok(Customer {
            id: row.get(0)?,
            name: row.get(1)?,
            birth_date: row.get(2)?,
            created_at: row.get(3)?,
            updated_at: row.get(4)?,
        })
    }).map_err(|e| e.to_string())?;

    let mut list = Vec::new();
    for r in rows {
        list.push(r.map_err(|e| e.to_string())?);
    }
    Ok(list)
}

#[tauri::command]
pub fn add_customer(
    state: State<'_, DbState>,
    name: String,
    birth_date: String,
) -> Result<Customer, String> {
    let conn = state.conn.lock().map_err(|_| "Failed to lock database connection")?;
    
    let name_trimmed = name.trim();
    let birth_date_trimmed = birth_date.trim();

    if name_trimmed.is_empty() {
        return Err("NAME_REQUIRED".into());
    }
    if birth_date_trimmed.is_empty() {
        return Err("BIRTH_DATE_REQUIRED".into());
    }

    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM customers WHERE LOWER(TRIM(name)) = LOWER(TRIM(?1)) AND TRIM(birth_date) = TRIM(?2);",
        params![name_trimmed, birth_date_trimmed],
        |row| row.get(0),
    ).map_err(|e| format!("Database query error: {}", e))?;

    if count > 0 {
        return Err("PATIENT_ALREADY_EXISTS".into());
    }

    conn.execute(
        "INSERT INTO customers (name, birth_date) VALUES (?1, ?2);",
        params![name_trimmed, birth_date_trimmed],
    ).map_err(|e| format!("Failed to insert customer: {}", e))?;

    let last_id = conn.last_insert_rowid();
    
    let customer = conn.query_row(
        "SELECT id, name, birth_date, created_at, updated_at FROM customers WHERE id = ?1;",
        params![last_id],
        |row| Ok(Customer {
            id: row.get(0)?,
            name: row.get(1)?,
            birth_date: row.get(2)?,
            created_at: row.get(3)?,
            updated_at: row.get(4)?,
        })
    ).map_err(|e| format!("Failed to retrieve newly created customer: {}", e))?;

    Ok(customer)
}

#[tauri::command]
pub fn delete_customer(state: State<'_, DbState>, id: i64) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|_| "Failed to lock database connection")?;
    conn.execute("DELETE FROM customers WHERE id = ?1;", params![id])
        .map_err(|e| format!("Failed to delete customer: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn get_last_invoice(state: State<'_, DbState>, customer_id: i64) -> Result<Option<LastInvoice>, String> {
    let conn = state.conn.lock().map_err(|_| "Failed to lock database connection")?;
    
    // Find latest sale ID
    let sale_res: Result<(i64, Option<String>, Option<String>, Option<String>, Option<String>, Option<i32>, f64, String, Option<i32>, Option<String>, Option<String>), rusqlite::Error> = conn.query_row(
        "SELECT s.id, s.prescribing_doctor_name, s.patient_name, c.name, c.birth_date, s.treatment_period_days, s.total_da, s.created_at, s.is_loan, s.loan_status, s.loan_settled_at 
         FROM sales s
         LEFT JOIN customers c ON s.customer_id = c.id
         WHERE s.customer_id = ?1 ORDER BY s.id DESC LIMIT 1;",
        params![customer_id],
        |row| Ok((
            row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?,
            row.get(5)?, row.get(6)?, row.get(7)?, row.get(8)?, row.get(9)?, row.get(10)?
        ))
    );

    let (sale_id, doctor, patient, cust_name, birth_date, treatment_days, total, date, is_loan_raw, loan_status, loan_settled_at) = match sale_res {
        Ok(data) => data,
        Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(None),
        Err(e) => return Err(e.to_string()),
    };

    // Load items (Always cashier mode since this sidebar shows only names and prices)
    let mut stmt = conn.prepare(
        "SELECT COALESCE(NULLIF(si.drug_name, ''), d.name, 'Article archivé'), si.quantity_items, si.unit_price_da, si.line_total_da 
         FROM sale_items si 
         LEFT JOIN drugs d ON si.drug_id = d.id 
         WHERE si.sale_id = ?1;"
    ).map_err(|e| e.to_string())?;

    let rows = stmt.query_map(params![sale_id], |row| {
        Ok(InvoiceItem {
            drug_name: row.get(0)?,
            quantity_items: row.get(1)?,
            unit_price_da: row.get(2)?,
            line_total_da: row.get(3)?,
            cost_price_da: None,
        })
    }).map_err(|e| e.to_string())?;

    let mut items = Vec::new();
    for r in rows {
        items.push(r.map_err(|e| e.to_string())?);
    }

    let patient_display = match patient {
        Some(ref p) if !p.trim().is_empty() => Some(p.clone()),
        _ => cust_name,
    };

    Ok(Some(LastInvoice {
        id: sale_id,
        prescribing_doctor_name: doctor,
        patient_name: patient_display,
        patient_birth_date: birth_date,
        treatment_period_days: treatment_days,
        total_da: total,
        created_at: date,
        items,
        is_loan: Some(is_loan_raw.unwrap_or(0) == 1),
        loan_status,
        loan_settled_at,
    }))
}

#[tauri::command]
pub fn get_sale_by_id(state: State<'_, DbState>, sale_id: i64, admin: bool) -> Result<LastInvoice, String> {
    let conn = state.conn.lock().map_err(|_| "Failed to lock database connection")?;
    
    let sale_res: Result<(Option<String>, Option<String>, Option<String>, Option<String>, Option<i32>, f64, String, Option<i32>, Option<String>, Option<String>), rusqlite::Error> = conn.query_row(
        "SELECT s.prescribing_doctor_name, s.patient_name, c.name, c.birth_date, s.treatment_period_days, s.total_da, s.created_at, s.is_loan, s.loan_status, s.loan_settled_at 
         FROM sales s
         LEFT JOIN customers c ON s.customer_id = c.id
         WHERE s.id = ?1;",
        params![sale_id],
        |row| Ok((
            row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?,
            row.get(5)?, row.get(6)?, row.get(7)?, row.get(8)?, row.get(9)?
        ))
    );

    let (doctor, patient, cust_name, birth_date, treatment_days, total, date, is_loan_raw, loan_status, loan_settled_at) = sale_res.map_err(|e| e.to_string())?;

    // Load items
    let mut stmt = conn.prepare(
        "SELECT COALESCE(NULLIF(si.drug_name, ''), d.name, 'Article archivé'), si.quantity_items, si.unit_price_da, si.line_total_da, COALESCE(NULLIF(si.cost_price_da, 0.0), d.cost_price_da, 0.0) 
         FROM sale_items si 
         LEFT JOIN drugs d ON si.drug_id = d.id 
         WHERE si.sale_id = ?1;"
    ).map_err(|e| e.to_string())?;

    let rows = stmt.query_map(params![sale_id], |row| {
        let name: String = row.get(0)?;
        let qty: i32 = row.get(1)?;
        let unit_price: f64 = row.get(2)?;
        let line_total: f64 = row.get(3)?;
        let cost_price: f64 = row.get(4)?;
        Ok(InvoiceItem {
            drug_name: name,
            quantity_items: qty,
            unit_price_da: unit_price,
            line_total_da: line_total,
            cost_price_da: if admin { Some(cost_price) } else { None },
        })
    }).map_err(|e| e.to_string())?;

    let mut items = Vec::new();
    for r in rows {
        items.push(r.map_err(|e| e.to_string())?);
    }

    let patient_display = match patient {
        Some(ref p) if !p.trim().is_empty() => Some(p.clone()),
        _ => cust_name,
    };

    Ok(LastInvoice {
        id: sale_id,
        prescribing_doctor_name: doctor,
        patient_name: patient_display,
        patient_birth_date: birth_date,
        treatment_period_days: treatment_days,
        total_da: total,
        created_at: date,
        items,
        is_loan: Some(is_loan_raw.unwrap_or(0) == 1),
        loan_status,
        loan_settled_at,
    })
}

// Extracted checkout core logic for transaction handling and testing
pub fn run_checkout_transaction_logic(
    tx: &rusqlite::Transaction,
    customer_id: Option<i64>,
    customer_name: Option<String>,
    prescribing_doctor_name: Option<String>,
    patient_name: Option<String>,
    treatment_period_days: Option<i32>,
    cart_items: Vec<CartItemInput>,
    is_loan: Option<bool>,
    loan_deductions: Option<Vec<LoanDeductionInput>>,
) -> Result<i64, String> {
    // 1. Resolve or create customer id
    let mut resolved_customer_id: Option<i64> = customer_id;
    if resolved_customer_id.is_none() {
        if let Some(name) = customer_name {
            let name_trimmed = name.trim();
            if !name_trimmed.is_empty() {
                // Find existing customer
                let cust_res: Result<i64, rusqlite::Error> = tx.query_row(
                    "SELECT id FROM customers WHERE name = ?1;",
                    params![name_trimmed],
                    |row| row.get(0)
                );
                
                resolved_customer_id = match cust_res {
                    Ok(id) => Some(id),
                    Err(rusqlite::Error::QueryReturnedNoRows) => {
                        // Create new customer
                        tx.execute(
                            "INSERT INTO customers (name) VALUES (?1);",
                            params![name_trimmed]
                        ).map_err(|e| format!("Failed to create customer: {}", e))?;
                        Some(tx.last_insert_rowid())
                    }
                    Err(e) => return Err(e.to_string()),
                };
            }
        }
    }

    // 2. Calculate grand total
    let grand_total: f64 = cart_items.iter().map(|item| item.unit_price_da * (item.quantity_items as f64)).sum();

    // 3. Insert sales record
    let loan_flag = if is_loan.unwrap_or(false) { 1 } else { 0 };
    let loan_status = if loan_flag == 1 { "active" } else { "completed" };

    tx.execute(
        "INSERT INTO sales (customer_id, prescribing_doctor_name, patient_name, total_da, treatment_period_days, is_loan, loan_status) 
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7);",
        params![resolved_customer_id, prescribing_doctor_name, patient_name, grand_total, treatment_period_days, loan_flag, loan_status]
    ).map_err(|e| format!("Failed to record sale: {}", e))?;
    
    let sale_id = tx.last_insert_rowid();

    // 4. Deduct stock using FEFO and record sale items
    for item in cart_items {
        let loan_qty_covered: i32 = loan_deductions.as_ref().map(|deductions| {
            deductions.iter().filter(|d| d.drug_id == item.drug_id).map(|d| d.quantity).sum()
        }).unwrap_or(0);

        let mut remaining_to_deduct = (item.quantity_items - loan_qty_covered).max(0);

        let (drug_name, cost_price_da): (String, f64) = tx.query_row(
            "SELECT name, cost_price_da FROM drugs WHERE id = ?1;",
            params![item.drug_id],
            |row| Ok((row.get(0)?, row.get(1)?))
        ).unwrap_or_else(|_| ("Médicament".to_string(), 0.0));

        if remaining_to_deduct > 0 {
            // Fetch batches for this drug sorted by expiry date
            let mut stmt = tx.prepare(
                "SELECT id, items_remaining, expiry_date FROM stock_batches 
                 WHERE drug_id = ?1 AND items_remaining > 0 
                 ORDER BY expiry_date ASC;"
            ).map_err(|e| e.to_string())?;

            let batch_rows = stmt.query_map(params![item.drug_id], |row| {
                Ok((row.get::<usize, i64>(0)?, row.get::<usize, i32>(1)?, row.get::<usize, String>(2)?))
            }).map_err(|e| e.to_string())?;

            let mut batches = Vec::new();
            for r in batch_rows {
                batches.push(r.map_err(|e| e.to_string())?);
            }

            // Check if total stock is sufficient
            let total_available: i32 = batches.iter().map(|b| b.1).sum();
            if total_available < remaining_to_deduct {
                return Err(format!("Stock insuffisant pour '{}' (Disponible: {} unit, Demandé: {} unit)", drug_name, total_available, remaining_to_deduct));
            }

            for (batch_id, items_remaining, _expiry_date) in batches {
                if remaining_to_deduct <= 0 {
                    break;
                }

                let deduction = std::cmp::min(remaining_to_deduct, items_remaining);
                let new_remaining = items_remaining - deduction;

                // Update batch remaining quantity
                tx.execute(
                    "UPDATE stock_batches SET items_remaining = ?1 WHERE id = ?2;",
                    params![new_remaining, batch_id]
                ).map_err(|e| format!("Failed to update stock batch: {}", e))?;

                // Record sale item linked to this batch
                let line_total = (deduction as f64) * item.unit_price_da;
                tx.execute(
                    "INSERT INTO sale_items (sale_id, drug_id, batch_id, drug_name, quantity_items, unit_price_da, cost_price_da, line_total_da) 
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8);",
                    params![sale_id, item.drug_id, batch_id, drug_name, deduction, item.unit_price_da, cost_price_da, line_total]
                ).map_err(|e| format!("Failed to insert sale item: {}", e))?;

                remaining_to_deduct -= deduction;
            }
        }

        // If part or all of this item was covered by a previous loan, record the covered portion in sale_items (batch_id = NULL)
        if loan_qty_covered > 0 {
            let covered_qty = std::cmp::min(loan_qty_covered, item.quantity_items);
            let line_total = (covered_qty as f64) * item.unit_price_da;
            tx.execute(
                "INSERT INTO sale_items (sale_id, drug_id, batch_id, drug_name, quantity_items, unit_price_da, cost_price_da, line_total_da) 
                 VALUES (?1, ?2, NULL, ?3, ?4, ?5, ?6, ?7);",
                params![sale_id, item.drug_id, drug_name, covered_qty, item.unit_price_da, cost_price_da, line_total]
            ).map_err(|e| format!("Failed to insert covered loan sale item: {}", e))?;
        }
    }

    // 5. Settle or reduce the loans that were deducted
    if let Some(ref deductions) = loan_deductions {
        for d in deductions {
            let loan_item_qty: Result<i32, rusqlite::Error> = tx.query_row(
                "SELECT quantity_items FROM sale_items WHERE sale_id = ?1 AND drug_id = ?2 LIMIT 1;",
                params![d.loan_sale_id, d.drug_id],
                |r| r.get(0),
            );
            if let Ok(qty) = loan_item_qty {
                if qty <= d.quantity {
                    tx.execute(
                        "UPDATE sales SET loan_status = 'settled', loan_settled_at = datetime('now', 'localtime') WHERE id = ?1 AND is_loan = 1;",
                        params![d.loan_sale_id],
                    ).map_err(|e| format!("Failed to settle loan {}: {}", d.loan_sale_id, e))?;
                } else {
                    let new_qty = qty - d.quantity;
                    tx.execute(
                        "UPDATE sale_items SET quantity_items = ?1, line_total_da = ?1 * unit_price_da WHERE sale_id = ?2 AND drug_id = ?3;",
                        params![new_qty, d.loan_sale_id, d.drug_id],
                    ).map_err(|e| format!("Failed to update partial loan item: {}", e))?;

                    tx.execute(
                        "UPDATE sales SET total_da = (SELECT COALESCE(SUM(line_total_da), 0.0) FROM sale_items WHERE sale_id = ?1) WHERE id = ?1;",
                        params![d.loan_sale_id],
                    ).map_err(|e| format!("Failed to update partial loan total: {}", e))?;
                }
            } else {
                tx.execute(
                    "UPDATE sales SET loan_status = 'settled', loan_settled_at = datetime('now', 'localtime') WHERE id = ?1 AND is_loan = 1;",
                    params![d.loan_sale_id],
                ).map_err(|e| format!("Failed to settle loan {}: {}", d.loan_sale_id, e))?;
            }
        }
    }

    Ok(sale_id)
}

#[tauri::command]
pub fn checkout_sale(
    state: State<'_, DbState>,
    customer_id: Option<i64>,
    customer_name: Option<String>,
    prescribing_doctor_name: Option<String>,
    patient_name: Option<String>,
    treatment_period_days: Option<i32>,
    cart_items: Vec<CartItemInput>,
    is_loan: Option<bool>,
    loan_deductions: Option<Vec<LoanDeductionInput>>,
) -> Result<i64, String> {
    if cart_items.is_empty() {
        return Err("Cart is empty".into());
    }

    let mut conn = state.conn.lock().map_err(|_| "Failed to lock database connection")?;
    let tx = conn.transaction().map_err(|e| format!("Failed to start transaction: {}", e))?;

    let sale_id = run_checkout_transaction_logic(
        &tx,
        customer_id,
        customer_name,
        prescribing_doctor_name,
        patient_name,
        treatment_period_days,
        cart_items,
        is_loan,
        loan_deductions,
    )?;

    tx.commit().map_err(|e| format!("Transaction commit failed: {}", e))?;
    Ok(sale_id)
}

#[tauri::command]
pub fn get_patient_active_loans(
    state: State<'_, DbState>,
    customer_id: i64,
) -> Result<Vec<ActiveLoanItem>, String> {
    let conn = state.conn.lock().map_err(|_| "Failed to lock database connection")?;
    let mut stmt = conn.prepare(
        "SELECT s.id, s.created_at, si.drug_id, COALESCE(NULLIF(si.drug_name, ''), d.name, 'Médicament'), si.quantity_items, si.unit_price_da
         FROM sales s
         JOIN sale_items si ON s.id = si.sale_id
         LEFT JOIN drugs d ON si.drug_id = d.id
         WHERE s.customer_id = ?1 AND s.is_loan = 1 AND s.loan_status = 'active'
         ORDER BY s.id ASC;"
    ).map_err(|e| e.to_string())?;

    let rows = stmt.query_map(params![customer_id], |row| {
        Ok(ActiveLoanItem {
            sale_id: row.get(0)?,
            sale_date: row.get(1)?,
            drug_id: row.get(2)?,
            drug_name: row.get(3)?,
            quantity: row.get(4)?,
            unit_price_da: row.get(5)?,
        })
    }).map_err(|e| e.to_string())?;

    let mut items = Vec::new();
    for r in rows {
        items.push(r.map_err(|e| e.to_string())?);
    }
    Ok(items)
}

#[tauri::command]
pub fn get_sales_history(state: State<'_, DbState>, admin: bool) -> Result<Vec<LastInvoice>, String> {
    let conn = state.conn.lock().map_err(|_| "Failed to lock database connection")?;
    
    let mut stmt = conn.prepare(
        "SELECT s.id, s.prescribing_doctor_name, s.patient_name, c.name, c.birth_date, s.treatment_period_days, s.total_da, s.created_at, s.is_loan, s.loan_status, s.loan_settled_at 
         FROM sales s
         LEFT JOIN customers c ON s.customer_id = c.id
         ORDER BY s.id DESC;"
    ).map_err(|e| e.to_string())?;

    let rows = stmt.query_map([], |row| {
        let is_loan_val: i32 = row.get(8).unwrap_or(0);
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, Option<String>>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, Option<String>>(4)?,
            row.get::<_, Option<i32>>(5)?,
            row.get::<_, f64>(6)?,
            row.get::<_, String>(7)?,
            is_loan_val == 1,
            row.get::<_, Option<String>>(9)?,
            row.get::<_, Option<String>>(10)?,
        ))
    }).map_err(|e| e.to_string())?;

    // Collect into a vector first to release the database connection borrow before making nested queries!
    let mut sales_records = Vec::new();
    for r in rows {
        sales_records.push(r.map_err(|e| e.to_string())?);
    }
    
    std::mem::drop(stmt);

    let mut sales = Vec::new();
    for (id, doctor, patient, cust_name, birth_date, treatment_days, total, date, is_loan, loan_status, loan_settled_at) in sales_records {
        // Query items
        let mut item_stmt = conn.prepare(
            "SELECT COALESCE(NULLIF(si.drug_name, ''), d.name, 'Article archivé'), si.quantity_items, si.unit_price_da, si.line_total_da, COALESCE(NULLIF(si.cost_price_da, 0.0), d.cost_price_da, 0.0) 
             FROM sale_items si 
             LEFT JOIN drugs d ON si.drug_id = d.id 
             WHERE si.sale_id = ?1;"
        ).map_err(|e| e.to_string())?;

        let item_rows = item_stmt.query_map(params![id], |item_row| {
            let name: String = item_row.get(0)?;
            let qty: i32 = item_row.get(1)?;
            let unit_price: f64 = item_row.get(2)?;
            let line_total: f64 = item_row.get(3)?;
            let cost_price: f64 = item_row.get(4)?;
            Ok(InvoiceItem {
                drug_name: name,
                quantity_items: qty,
                unit_price_da: unit_price,
                line_total_da: line_total,
                cost_price_da: if admin { Some(cost_price) } else { None },
            })
        }).map_err(|e| e.to_string())?;

        let mut items = Vec::new();
        for ir in item_rows {
            items.push(ir.map_err(|e| e.to_string())?);
        }

        let patient_display = match patient {
            Some(ref p) if !p.trim().is_empty() => Some(p.clone()),
            _ => cust_name,
        };

        sales.push(LastInvoice {
            id,
            prescribing_doctor_name: doctor,
            patient_name: patient_display,
            patient_birth_date: birth_date,
            treatment_period_days: treatment_days,
            total_da: total,
            created_at: date,
            items,
            is_loan: Some(is_loan),
            loan_status,
            loan_settled_at,
        });
    }

    Ok(sales)
}

#[tauri::command]
pub fn get_loans(state: State<'_, DbState>, admin: bool) -> Result<Vec<LastInvoice>, String> {
    let conn = state.conn.lock().map_err(|_| "Failed to lock database connection")?;
    
    let mut stmt = conn.prepare(
        "SELECT s.id, s.prescribing_doctor_name, s.patient_name, c.name, c.birth_date, s.treatment_period_days, s.total_da, s.created_at, s.is_loan, s.loan_status, s.loan_settled_at 
         FROM sales s
         LEFT JOIN customers c ON s.customer_id = c.id
         WHERE s.is_loan = 1
         ORDER BY s.id DESC;"
    ).map_err(|e| e.to_string())?;

    let rows = stmt.query_map([], |row| {
        let is_loan_val: i32 = row.get(8).unwrap_or(0);
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, Option<String>>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, Option<String>>(4)?,
            row.get::<_, Option<i32>>(5)?,
            row.get::<_, f64>(6)?,
            row.get::<_, String>(7)?,
            is_loan_val == 1,
            row.get::<_, Option<String>>(9)?,
            row.get::<_, Option<String>>(10)?,
        ))
    }).map_err(|e| e.to_string())?;

    let mut sales_records = Vec::new();
    for r in rows {
        sales_records.push(r.map_err(|e| e.to_string())?);
    }
    
    std::mem::drop(stmt);

    let mut loans = Vec::new();
    for (id, doctor, patient, cust_name, birth_date, treatment_days, total, date, is_loan, loan_status, loan_settled_at) in sales_records {
        let mut item_stmt = conn.prepare(
            "SELECT COALESCE(NULLIF(si.drug_name, ''), d.name, 'Article archivé'), si.quantity_items, si.unit_price_da, si.line_total_da, COALESCE(NULLIF(si.cost_price_da, 0.0), d.cost_price_da, 0.0) 
             FROM sale_items si 
             LEFT JOIN drugs d ON si.drug_id = d.id 
             WHERE si.sale_id = ?1;"
        ).map_err(|e| e.to_string())?;

        let item_rows = item_stmt.query_map(params![id], |item_row| {
            let name: String = item_row.get(0)?;
            let qty: i32 = item_row.get(1)?;
            let unit_price: f64 = item_row.get(2)?;
            let line_total: f64 = item_row.get(3)?;
            let cost_price: f64 = item_row.get(4)?;
            Ok(InvoiceItem {
                drug_name: name,
                quantity_items: qty,
                unit_price_da: unit_price,
                line_total_da: line_total,
                cost_price_da: if admin { Some(cost_price) } else { None },
            })
        }).map_err(|e| e.to_string())?;

        let mut items = Vec::new();
        for ir in item_rows {
            items.push(ir.map_err(|e| e.to_string())?);
        }

        let patient_display = match patient {
            Some(ref p) if !p.trim().is_empty() => Some(p.clone()),
            _ => cust_name,
        };

        loans.push(LastInvoice {
            id,
            prescribing_doctor_name: doctor,
            patient_name: patient_display,
            patient_birth_date: birth_date,
            treatment_period_days: treatment_days,
            total_da: total,
            created_at: date,
            items,
            is_loan: Some(is_loan),
            loan_status,
            loan_settled_at,
        });
    }

    Ok(loans)
}

#[tauri::command]
pub fn settle_loan(state: State<'_, DbState>, sale_id: i64) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|_| "Failed to lock database connection")?;
    let rows_affected = conn.execute(
        "UPDATE sales SET loan_status = 'settled', loan_settled_at = datetime('now', 'localtime') WHERE id = ?1 AND is_loan = 1;",
        params![sale_id],
    ).map_err(|e| format!("Failed to settle loan: {}", e))?;

    if rows_affected == 0 {
        return Err("No loan found with the specified ID".to_string());
    }

    Ok(())
}


// USB Backup & Restore Commands
pub fn get_removable_drives() -> Result<Vec<String>, String> {
    let output = Command::new("powershell")
        .args(&[
            "-NoProfile",
            "-Command",
            "Get-Volume | Where-Object {$_.DriveType -eq 'Removable' -and $_.DriveLetter -ne $null} | Select-Object -ExpandProperty DriveLetter"
        ])
        .output()
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut drives = Vec::new();
    for line in stdout.lines() {
        let drive = line.trim();
        if !drive.is_empty() {
            drives.push(format!("{}:\\", drive));
        }
    }

    Ok(drives)
}

#[tauri::command]
pub fn backup_database(state: State<'_, DbState>, app_handle: tauri::AppHandle) -> Result<String, String> {
    let drives = get_removable_drives()?;
    if drives.is_empty() {
        return Err("No removable USB drive detected. Please insert a USB key.".into());
    }

    let drive_path = &drives[0];
    let backup_dir = std::path::Path::new(drive_path).join("PharmacyPOS_Backup");
    
    std::fs::create_dir_all(&backup_dir)
        .map_err(|e| format!("Failed to create directory on USB: {}", e))?;
    
    let backup_file = backup_dir.join("pharmacy_backup.db");

    let app_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    let db_path = app_dir.join("pharmacy.db");

    if !db_path.exists() {
        return Err("Source database file does not exist.".into());
    }

    std::fs::copy(&db_path, &backup_file).map_err(|e| format!("Copy failed: {}", e))?;

    let conn = state.conn.lock().map_err(|_| "Failed to lock database connection")?;
    let now_str = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    
    conn.execute(
        "UPDATE settings SET last_backup_at = ?1 WHERE id = 1;",
        params![now_str],
    ).map_err(|e| format!("Failed to update backup timestamp: {}", e))?;

    Ok(format!("Sauvegarde réussie sur {}.", backup_file.to_string_lossy()))
}

#[tauri::command]
pub fn restore_database(state: State<'_, DbState>, app_handle: tauri::AppHandle) -> Result<String, String> {
    let drives = get_removable_drives()?;
    if drives.is_empty() {
        return Err("No removable USB drive detected. Please insert your backup USB key.".into());
    }

    let drive_path = &drives[0];
    let backup_file = std::path::Path::new(drive_path).join("PharmacyPOS_Backup").join("pharmacy_backup.db");

    if !backup_file.exists() {
        return Err("No backup file found at PharmacyPOS_Backup\\pharmacy_backup.db on the USB drive.".into());
    }

    let app_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    let db_path = app_dir.join("pharmacy.db");

    let mut conn_guard = state.conn.lock().map_err(|_| "Failed to lock database connection")?;
    
    let temp_conn = rusqlite::Connection::open_in_memory().map_err(|e| e.to_string())?;
    let old_conn = std::mem::replace(&mut *conn_guard, temp_conn);
    old_conn.close().map_err(|(_, e)| format!("Failed to close old connection: {}", e))?;

    std::fs::copy(&backup_file, &db_path).map_err(|e| format!("Copy failed: {}", e))?;

    let new_conn = rusqlite::Connection::open(&db_path).map_err(|e| format!("Failed to reopen database: {}", e))?;
    let _ = std::mem::replace(&mut *conn_guard, new_conn);

    Ok("Base de données restaurée avec succès ! L'application va se recharger.".into())
}
#[derive(serde::Serialize, Clone)]
pub struct DrugStockItem {
    pub id: i64,
    pub barcode: String,
    pub name: String,
    pub price_per_item_da: f64,
    pub cost_price_da: f64,
    pub items_per_package: i32,
    pub requires_prescription: bool,
    pub tva: f64,
    pub mg: f64,
    pub total_stock_pcs: i64,
    pub nearest_expiry_date: Option<String>,
    pub batch_number: String,
    pub batches_count: i64,
    pub invoice_id: Option<i64>,
    pub invoice_number: Option<String>,
    pub supplier_name: Option<String>,
    pub invoice_date: Option<String>,
}

#[tauri::command]
pub fn get_drugs_stock_list(state: State<'_, DbState>) -> Result<Vec<DrugStockItem>, String> {
    let conn = state.conn.lock().map_err(|_| "Failed to lock database connection")?;
    let mut stmt = conn.prepare(
        "SELECT 
            d.id,
            d.barcode,
            d.name,
            COALESCE(
                (SELECT price_per_item_da FROM stock_batches WHERE drug_id = d.id AND items_remaining > 0 ORDER BY expiry_date ASC LIMIT 1),
                (SELECT price_per_item_da FROM stock_batches WHERE drug_id = d.id ORDER BY id DESC LIMIT 1),
                d.price_per_item_da
            ) AS price_per_item_da,
            COALESCE(
                (SELECT cost_price_da FROM stock_batches WHERE drug_id = d.id AND items_remaining > 0 ORDER BY expiry_date ASC LIMIT 1),
                (SELECT cost_price_da FROM stock_batches WHERE drug_id = d.id ORDER BY id DESC LIMIT 1),
                d.cost_price_da
            ) AS cost_price_da,
            d.items_per_package,
            d.requires_prescription,
            COALESCE(
                (SELECT tva FROM stock_batches WHERE drug_id = d.id AND items_remaining > 0 ORDER BY expiry_date ASC LIMIT 1),
                (SELECT tva FROM stock_batches WHERE drug_id = d.id ORDER BY id DESC LIMIT 1),
                d.tva
            ) AS tva,
            COALESCE(
                (SELECT mg FROM stock_batches WHERE drug_id = d.id AND items_remaining > 0 ORDER BY expiry_date ASC LIMIT 1),
                (SELECT mg FROM stock_batches WHERE drug_id = d.id ORDER BY id DESC LIMIT 1),
                d.mg
            ) AS mg,
            COALESCE(SUM(sb.items_remaining), 0) AS total_stock_pcs,
            MIN(sb.expiry_date) AS nearest_expiry_date,
            COALESCE((SELECT batch_number FROM stock_batches WHERE drug_id = d.id AND items_remaining > 0 ORDER BY expiry_date ASC LIMIT 1), (SELECT batch_number FROM stock_batches WHERE drug_id = d.id ORDER BY id DESC LIMIT 1), '') AS batch_number,
            COALESCE((SELECT COUNT(*) FROM stock_batches WHERE drug_id = d.id AND items_remaining > 0), 0) AS batches_count,
            COALESCE(
                (SELECT sb2.invoice_id FROM stock_batches sb2 WHERE sb2.drug_id = d.id AND sb2.items_remaining > 0 ORDER BY sb2.expiry_date ASC LIMIT 1),
                (SELECT sb2.invoice_id FROM stock_batches sb2 WHERE sb2.drug_id = d.id ORDER BY sb2.id DESC LIMIT 1)
            ) AS invoice_id,
            COALESCE(
                (SELECT inv.invoice_number FROM stock_batches sb2 JOIN imported_invoices inv ON inv.id = sb2.invoice_id WHERE sb2.drug_id = d.id AND sb2.items_remaining > 0 ORDER BY sb2.expiry_date ASC LIMIT 1),
                (SELECT inv.invoice_number FROM stock_batches sb2 JOIN imported_invoices inv ON inv.id = sb2.invoice_id WHERE sb2.drug_id = d.id ORDER BY sb2.id DESC LIMIT 1)
            ) AS invoice_number,
            COALESCE(
                (SELECT inv.supplier_name FROM stock_batches sb2 JOIN imported_invoices inv ON inv.id = sb2.invoice_id WHERE sb2.drug_id = d.id AND sb2.items_remaining > 0 ORDER BY sb2.expiry_date ASC LIMIT 1),
                (SELECT inv.supplier_name FROM stock_batches sb2 JOIN imported_invoices inv ON inv.id = sb2.invoice_id WHERE sb2.drug_id = d.id ORDER BY sb2.id DESC LIMIT 1)
            ) AS supplier_name,
            COALESCE(
                (SELECT inv.invoice_date FROM stock_batches sb2 JOIN imported_invoices inv ON inv.id = sb2.invoice_id WHERE sb2.drug_id = d.id AND sb2.items_remaining > 0 ORDER BY sb2.expiry_date ASC LIMIT 1),
                (SELECT inv.invoice_date FROM stock_batches sb2 JOIN imported_invoices inv ON inv.id = sb2.invoice_id WHERE sb2.drug_id = d.id ORDER BY sb2.id DESC LIMIT 1)
            ) AS invoice_date
        FROM drugs d
        LEFT JOIN stock_batches sb ON d.id = sb.drug_id AND sb.items_remaining > 0
        GROUP BY d.id
        ORDER BY d.name ASC;"
    ).map_err(|e| e.to_string())?;

    let rows = stmt.query_map([], |row| {
        let req_presc: i64 = row.get(6)?;
        Ok(DrugStockItem {
            id: row.get(0)?,
            barcode: row.get(1)?,
            name: row.get(2)?,
            price_per_item_da: row.get(3)?,
            cost_price_da: row.get(4)?,
            items_per_package: row.get(5)?,
            requires_prescription: req_presc != 0,
            tva: row.get(7)?,
            mg: row.get(8)?,
            total_stock_pcs: row.get(9)?,
            nearest_expiry_date: row.get(10)?,
            batch_number: row.get(11)?,
            batches_count: row.get(12)?,
            invoice_id: row.get(13)?,
            invoice_number: row.get(14)?,
            supplier_name: row.get(15)?,
            invoice_date: row.get(16)?,
        })
    }).map_err(|e| e.to_string())?;

    let mut list = Vec::new();
    for r in rows {
        list.push(r.map_err(|e| e.to_string())?);
    }
    Ok(list)
}

#[tauri::command]
pub fn search_drugs(state: State<'_, DbState>, query: String) -> Result<Vec<DrugStockItem>, String> {
    let conn = state.conn.lock().map_err(|_| "Failed to lock database connection")?;
    let search_pattern = format!("%{}%", query.trim());
    let mut stmt = conn.prepare(
        "SELECT 
            d.id,
            d.barcode,
            d.name,
            COALESCE(
                (SELECT price_per_item_da FROM stock_batches WHERE drug_id = d.id AND items_remaining > 0 ORDER BY expiry_date ASC LIMIT 1),
                (SELECT price_per_item_da FROM stock_batches WHERE drug_id = d.id ORDER BY id DESC LIMIT 1),
                d.price_per_item_da
            ) AS price_per_item_da,
            COALESCE(
                (SELECT cost_price_da FROM stock_batches WHERE drug_id = d.id AND items_remaining > 0 ORDER BY expiry_date ASC LIMIT 1),
                (SELECT cost_price_da FROM stock_batches WHERE drug_id = d.id ORDER BY id DESC LIMIT 1),
                d.cost_price_da
            ) AS cost_price_da,
            d.items_per_package,
            d.requires_prescription,
            COALESCE(
                (SELECT tva FROM stock_batches WHERE drug_id = d.id AND items_remaining > 0 ORDER BY expiry_date ASC LIMIT 1),
                (SELECT tva FROM stock_batches WHERE drug_id = d.id ORDER BY id DESC LIMIT 1),
                d.tva
            ) AS tva,
            COALESCE(
                (SELECT mg FROM stock_batches WHERE drug_id = d.id AND items_remaining > 0 ORDER BY expiry_date ASC LIMIT 1),
                (SELECT mg FROM stock_batches WHERE drug_id = d.id ORDER BY id DESC LIMIT 1),
                d.mg
            ) AS mg,
            COALESCE(SUM(sb.items_remaining), 0) AS total_stock_pcs,
            MIN(sb.expiry_date) AS nearest_expiry_date,
            COALESCE((SELECT batch_number FROM stock_batches WHERE drug_id = d.id AND items_remaining > 0 ORDER BY expiry_date ASC LIMIT 1), '') AS batch_number,
            COALESCE((SELECT COUNT(*) FROM stock_batches WHERE drug_id = d.id AND items_remaining > 0), 0) AS batches_count,
            COALESCE(
                (SELECT sb2.invoice_id FROM stock_batches sb2 WHERE sb2.drug_id = d.id AND sb2.items_remaining > 0 ORDER BY sb2.expiry_date ASC LIMIT 1),
                (SELECT sb2.invoice_id FROM stock_batches sb2 WHERE sb2.drug_id = d.id ORDER BY sb2.id DESC LIMIT 1)
            ) AS invoice_id,
            COALESCE(
                (SELECT inv.invoice_number FROM stock_batches sb2 JOIN imported_invoices inv ON inv.id = sb2.invoice_id WHERE sb2.drug_id = d.id AND sb2.items_remaining > 0 ORDER BY sb2.expiry_date ASC LIMIT 1),
                (SELECT inv.invoice_number FROM stock_batches sb2 JOIN imported_invoices inv ON inv.id = sb2.invoice_id WHERE sb2.drug_id = d.id ORDER BY sb2.id DESC LIMIT 1)
            ) AS invoice_number,
            COALESCE(
                (SELECT inv.supplier_name FROM stock_batches sb2 JOIN imported_invoices inv ON inv.id = sb2.invoice_id WHERE sb2.drug_id = d.id AND sb2.items_remaining > 0 ORDER BY sb2.expiry_date ASC LIMIT 1),
                (SELECT inv.supplier_name FROM stock_batches sb2 JOIN imported_invoices inv ON inv.id = sb2.invoice_id WHERE sb2.drug_id = d.id ORDER BY sb2.id DESC LIMIT 1)
            ) AS supplier_name,
            COALESCE(
                (SELECT inv.invoice_date FROM stock_batches sb2 JOIN imported_invoices inv ON inv.id = sb2.invoice_id WHERE sb2.drug_id = d.id AND sb2.items_remaining > 0 ORDER BY sb2.expiry_date ASC LIMIT 1),
                (SELECT inv.invoice_date FROM stock_batches sb2 JOIN imported_invoices inv ON inv.id = sb2.invoice_id WHERE sb2.drug_id = d.id ORDER BY sb2.id DESC LIMIT 1)
            ) AS invoice_date
        FROM drugs d
        LEFT JOIN stock_batches sb ON d.id = sb.drug_id AND sb.items_remaining > 0
        WHERE d.name LIKE ?1 OR d.barcode LIKE ?1
        GROUP BY d.id
        ORDER BY d.name ASC
        LIMIT 20;"
    ).map_err(|e| e.to_string())?;

    let rows = stmt.query_map(params![search_pattern], |row| {
        let req_presc: i64 = row.get(6)?;
        Ok(DrugStockItem {
            id: row.get(0)?,
            barcode: row.get(1)?,
            name: row.get(2)?,
            price_per_item_da: row.get(3)?,
            cost_price_da: row.get(4)?,
            items_per_package: row.get(5)?,
            requires_prescription: req_presc != 0,
            tva: row.get(7)?,
            mg: row.get(8)?,
            total_stock_pcs: row.get(9)?,
            nearest_expiry_date: row.get(10)?,
            batch_number: row.get(11)?,
            batches_count: row.get(12)?,
            invoice_id: row.get(13)?,
            invoice_number: row.get(14)?,
            supplier_name: row.get(15)?,
            invoice_date: row.get(16)?,
        })
    }).map_err(|e| e.to_string())?;

    let mut list = Vec::new();
    for r in rows {
        list.push(r.map_err(|e| e.to_string())?);
    }
    Ok(list)
}

#[tauri::command]
pub fn delete_stock_batch(state: State<'_, DbState>, batch_id: i64) -> Result<(), String> {
    let mut conn = state.conn.lock().map_err(|_| "Failed to lock database connection")?;

    conn.execute("PRAGMA foreign_keys = OFF;", [])
        .map_err(|e| format!("Failed to disable foreign keys: {}", e))?;

    let res = (|| -> Result<(), rusqlite::Error> {
        let tx = conn.transaction()?;
        tx.execute("UPDATE sale_items SET batch_id = NULL WHERE batch_id = ?1;", params![batch_id])?;
        tx.execute("DELETE FROM stock_batches WHERE id = ?1;", params![batch_id])?;
        tx.commit()?;
        Ok(())
    })();

    let _ = conn.execute("PRAGMA foreign_keys = ON;", []);

    res.map_err(|e| format!("Failed to delete stock batch: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn delete_drug(state: State<'_, DbState>, id: i64) -> Result<(), String> {
    let mut conn = state.conn.lock().map_err(|_| "Failed to lock database connection")?;

    // Backfill drug_name and cost_price_da in sale_items before deletion to preserve historical sales
    let drug_info: Option<(String, f64)> = conn.query_row(
        "SELECT name, cost_price_da FROM drugs WHERE id = ?1;",
        params![id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    ).ok();

    if let Some((name, cost)) = drug_info {
        let _ = conn.execute(
            "UPDATE sale_items SET 
                drug_name = CASE WHEN drug_name IS NULL OR drug_name = '' THEN ?1 ELSE drug_name END,
                cost_price_da = CASE WHEN cost_price_da = 0.0 THEN ?2 ELSE cost_price_da END
             WHERE drug_id = ?3;",
            params![name, cost, id],
        );
    }

    // Temporarily turn foreign_keys OFF to ensure clean removal of drug, batches, and aliases
    conn.execute("PRAGMA foreign_keys = OFF;", [])
        .map_err(|e| format!("Failed to disable foreign keys: {}", e))?;

    let res = (|| -> Result<(), rusqlite::Error> {
        let tx = conn.transaction()?;
        tx.execute("UPDATE sale_items SET drug_id = NULL, batch_id = NULL WHERE drug_id = ?1;", params![id])?;
        tx.execute("DELETE FROM stock_batches WHERE drug_id = ?1;", params![id])?;
        tx.execute("DELETE FROM drug_aliases WHERE drug_id = ?1;", params![id])?;
        tx.execute("DELETE FROM drugs WHERE id = ?1;", params![id])?;
        tx.commit()?;
        Ok(())
    })();

    let _ = conn.execute("PRAGMA foreign_keys = ON;", []);

    res.map_err(|e| format!("Failed to delete drug: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn update_drug(
    state: State<'_, DbState>,
    id: i64,
    name: String,
    barcode: String,
    price_per_item_da: f64,
    cost_price_da: f64,
    tva: f64,
    mg: f64,
    batch_number: Option<String>,
    expiry_date: Option<String>,
    stock_quantity: Option<i64>,
) -> Result<(), String> {
    let mut conn = state.conn.lock().map_err(|_| "Failed to lock database connection")?;
    let tx = conn.transaction().map_err(|e| format!("Failed to start transaction: {}", e))?;

    let barcode_clean = barcode.trim().to_string();
    if barcode_clean.is_empty() {
        return Err("Le code-barres ne peut pas être vide".into());
    }

    let duplicate: Option<i64> = tx.query_row(
        "SELECT id FROM drugs WHERE barcode = ?1 AND id != ?2;",
        params![barcode_clean, id],
        |row| row.get(0),
    ).ok();

    if duplicate.is_some() {
        return Err("Ce code-barres est déjà utilisé par un autre médicament".into());
    }

    tx.execute(
        "UPDATE drugs SET 
            name = ?1, 
            barcode = ?2, 
            price_per_item_da = ?3, 
            cost_price_da = ?4, 
            tva = ?5, 
            mg = ?6, 
            updated_at = datetime('now', 'localtime') 
         WHERE id = ?7;",
        params![name.trim(), barcode_clean, price_per_item_da, cost_price_da, tva, mg, id],
    ).map_err(|e| format!("Failed to update drug: {}", e))?;

    let batch_count: i64 = tx.query_row(
        "SELECT COUNT(*) FROM stock_batches WHERE drug_id = ?1;",
        params![id],
        |row| row.get(0),
    ).unwrap_or(0);

    let clean_batch_num = batch_number.as_deref().map(|s| s.trim()).filter(|s| !s.is_empty());
    let clean_exp = expiry_date.as_deref().map(|s| s.trim()).filter(|s| !s.is_empty());

    if batch_count <= 1 {
        let single_batch_id: Option<i64> = tx.query_row(
            "SELECT id FROM stock_batches WHERE drug_id = ?1 LIMIT 1;",
            params![id],
            |row| row.get(0),
        ).ok();

        if let Some(b_id) = single_batch_id {
            if let Some(qty) = stock_quantity {
                tx.execute(
                    "UPDATE stock_batches SET 
                        batch_number = COALESCE(?1, batch_number), 
                        expiry_date = COALESCE(?2, expiry_date), 
                        items_remaining = ?3,
                        price_per_item_da = ?4, 
                        cost_price_da = ?5, 
                        tva = ?6, 
                        mg = ?7 
                     WHERE id = ?8;",
                    params![clean_batch_num, clean_exp, qty, price_per_item_da, cost_price_da, tva, mg, b_id],
                ).map_err(|e| format!("Failed to update stock batch: {}", e))?;
            } else {
                tx.execute(
                    "UPDATE stock_batches SET 
                        batch_number = COALESCE(?1, batch_number), 
                        expiry_date = COALESCE(?2, expiry_date), 
                        price_per_item_da = ?3, 
                        cost_price_da = ?4, 
                        tva = ?5, 
                        mg = ?6 
                     WHERE id = ?7;",
                    params![clean_batch_num, clean_exp, price_per_item_da, cost_price_da, tva, mg, b_id],
                ).map_err(|e| format!("Failed to update stock batch: {}", e))?;
            }
        } else if let (Some(b_num), Some(exp)) = (clean_batch_num, clean_exp) {
            let qty = stock_quantity.unwrap_or(0);
            if qty > 0 {
                tx.execute(
                    "INSERT INTO stock_batches (drug_id, batch_number, expiry_date, packages_received, items_remaining, price_per_item_da, cost_price_da, tva, mg)
                     VALUES (?1, ?2, ?3, ?4, ?4, ?5, ?6, ?7, ?8);",
                    params![id, b_num, exp, qty, price_per_item_da, cost_price_da, tva, mg],
                ).map_err(|e| format!("Failed to insert batch: {}", e))?;
            }
        }
    } else {
        tx.execute(
            "UPDATE stock_batches SET price_per_item_da = ?1, cost_price_da = ?2, tva = ?3, mg = ?4 WHERE drug_id = ?5 AND items_remaining > 0;",
            params![price_per_item_da, cost_price_da, tva, mg, id],
        ).map_err(|e| format!("Failed to update batch prices: {}", e))?;

        if clean_batch_num.is_some() || clean_exp.is_some() {
            let nearest_id: Option<i64> = tx.query_row(
                "SELECT id FROM stock_batches WHERE drug_id = ?1 AND items_remaining > 0 ORDER BY expiry_date ASC LIMIT 1;",
                params![id],
                |row| row.get(0),
            ).ok();

            if let Some(n_id) = nearest_id {
                tx.execute(
                    "UPDATE stock_batches SET batch_number = COALESCE(?1, batch_number), expiry_date = COALESCE(?2, expiry_date) WHERE id = ?3;",
                    params![clean_batch_num, clean_exp, n_id],
                ).map_err(|e| format!("Failed to update nearest batch: {}", e))?;
            }
        }
    }

    tx.commit().map_err(|e| format!("Failed to commit update: {}", e))?;
    Ok(())
}

#[derive(serde::Serialize)]
pub struct Notification {
    pub id: String,
    pub type_: String,
    pub message_fr: String,
    pub message_ar: String,
    pub severity: String,
}

#[tauri::command]
pub fn get_notifications(state: State<'_, DbState>) -> Result<Vec<Notification>, String> {
    let conn = state.conn.lock().map_err(|_| "Failed to lock database connection")?;

    let mut notifications = Vec::new();

    // 1. Get settings
    let (warning_days, last_backup_at): (i32, Option<String>) = conn.query_row(
        "SELECT expiry_warning_days, last_backup_at FROM settings WHERE id = 1;",
        [],
        |row| Ok((row.get(0)?, row.get(1)?))
    ).map_err(|e| e.to_string())?;

    // 2. Query empty stock drugs
    let mut stmt = conn.prepare(
        "SELECT id, name FROM drugs 
         WHERE COALESCE((SELECT SUM(items_remaining) FROM stock_batches WHERE drug_id = drugs.id), 0) = 0 
         ORDER BY name ASC;"
    ).map_err(|e| e.to_string())?;
    let empty_rows = stmt.query_map([], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
    }).map_err(|e| e.to_string())?;
    for r in empty_rows {
        let (id, name) = r.map_err(|e| e.to_string())?;
        notifications.push(Notification {
            id: format!("empty_stock_{}", id),
            type_: "empty_stock".to_string(),
            message_fr: format!("{} est en rupture de stock.", name),
            message_ar: format!("{} نفد من المخزون.", name),
            severity: "danger".to_string(),
        });
    }
    std::mem::drop(stmt);

    // 3. Query low stock drugs
    let mut stmt = conn.prepare(
        "SELECT id, name, COALESCE((SELECT SUM(items_remaining) FROM stock_batches WHERE drug_id = drugs.id), 0)
         FROM drugs 
         WHERE COALESCE((SELECT SUM(items_remaining) FROM stock_batches WHERE drug_id = drugs.id), 0) > 0 
           AND COALESCE((SELECT SUM(items_remaining) FROM stock_batches WHERE drug_id = drugs.id), 0) <= 5 
         ORDER BY name ASC;"
    ).map_err(|e| e.to_string())?;
    let low_rows = stmt.query_map([], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, i64>(2)?))
    }).map_err(|e| e.to_string())?;
    for r in low_rows {
        let (id, name, pcs) = r.map_err(|e| e.to_string())?;
        notifications.push(Notification {
            id: format!("low_stock_{}", id),
            type_: "near_empty_stock".to_string(),
            message_fr: format!("{} : stock faible ({} pcs).", name, pcs),
            message_ar: format!("{} : مخزون منخفض ({} حبة).", name, pcs),
            severity: "warning".to_string(),
        });
    }
    std::mem::drop(stmt);

    // 4. Query expired batches
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let mut stmt = conn.prepare(
        "SELECT b.id, d.name, b.expiry_date 
         FROM stock_batches b
         JOIN drugs d ON b.drug_id = d.id
         WHERE b.items_remaining > 0 AND b.expiry_date <= ?1
         ORDER BY b.expiry_date ASC;"
    ).map_err(|e| e.to_string())?;
    let expired_rows = stmt.query_map(params![today], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?))
    }).map_err(|e| e.to_string())?;
    for r in expired_rows {
        let (id, name, expiry) = r.map_err(|e| e.to_string())?;
        notifications.push(Notification {
            id: format!("expired_batch_{}", id),
            type_: "expired".to_string(),
            message_fr: format!("{} (Lot #{}) est expiré ({}).", name, id, expiry),
            message_ar: format!("{} (دفعة #{}) منتهي الصلاحية ({}).", name, id, expiry),
            severity: "danger".to_string(),
        });
    }
    std::mem::drop(stmt);

    // 5. Query near-expired batches
    let warning_threshold_date = (chrono::Local::now() + chrono::Duration::days(warning_days as i64))
        .format("%Y-%m-%d").to_string();
    let mut stmt = conn.prepare(
        "SELECT b.id, d.name, b.expiry_date 
         FROM stock_batches b
         JOIN drugs d ON b.drug_id = d.id
         WHERE b.items_remaining > 0 AND b.expiry_date > ?1 AND b.expiry_date <= ?2
         ORDER BY b.expiry_date ASC;"
    ).map_err(|e| e.to_string())?;
    let near_expired_rows = stmt.query_map(params![today, warning_threshold_date], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?))
    }).map_err(|e| e.to_string())?;
    for r in near_expired_rows {
        let (id, name, expiry) = r.map_err(|e| e.to_string())?;
        notifications.push(Notification {
            id: format!("near_expired_batch_{}", id),
            type_: "near_expired".to_string(),
            message_fr: format!("{} (Lot #{}) expire bientôt ({}).", name, id, expiry),
            message_ar: format!("{} (دفعة #{}) ستنتهي صلاحيته قريباً ({}).", name, id, expiry),
            severity: "warning".to_string(),
        });
    }
    std::mem::drop(stmt);

    // 6. Check backup status
    let need_backup = match last_backup_at {
        None => true,
        Some(date_str) => {
            if let Ok(last_date) = chrono::NaiveDateTime::parse_from_str(&date_str, "%Y-%m-%d %H:%M:%S") {
                let diff = chrono::Local::now().naive_local().signed_duration_since(last_date);
                diff.num_days() >= 7
            } else {
                true
            }
        }
    };
    if need_backup {
        notifications.push(Notification {
            id: "backup_reminder".to_string(),
            type_: "backup_reminder".to_string(),
            message_fr: "Veuillez effectuer une sauvegarde sur clé USB.".to_string(),
            message_ar: "يرجى إجراء نسخ احتياطي على فلاشة USB.".to_string(),
            severity: "warning".to_string(),
        });
    }

    Ok(notifications)
}

#[derive(serde::Serialize)]
pub struct PatientItem {
    pub id: i64,
    pub name: String,
    pub birth_date: String,
    pub last_purchase_date: Option<String>,
}

#[tauri::command]
pub fn get_patients_list(state: State<'_, DbState>) -> Result<Vec<PatientItem>, String> {
    let conn = state.conn.lock().map_err(|_| "Failed to lock database connection")?;
    
    let mut stmt = conn.prepare(
        "SELECT c.id, c.name, c.birth_date, 
                (SELECT MAX(created_at) FROM sales WHERE customer_id = c.id) AS last_purchase
         FROM customers c
         ORDER BY c.name ASC;"
    ).map_err(|e| e.to_string())?;

    let rows = stmt.query_map([], |row| {
        Ok(PatientItem {
            id: row.get(0)?,
            name: row.get(1)?,
            birth_date: row.get(2)?,
            last_purchase_date: row.get(3)?,
        })
    }).map_err(|e| e.to_string())?;

    let mut list = Vec::new();
    for r in rows {
        list.push(r.map_err(|e| e.to_string())?);
    }
    Ok(list)
}

#[tauri::command]
pub fn update_customer(
    state: State<'_, DbState>,
    id: i64,
    name: String,
    birth_date: String,
) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|_| "Failed to lock database connection")?;
    
    let name_trimmed = name.trim();
    let birth_date_trimmed = birth_date.trim();

    if name_trimmed.is_empty() {
        return Err("NAME_REQUIRED".into());
    }
    if birth_date_trimmed.is_empty() {
        return Err("BIRTH_DATE_REQUIRED".into());
    }

    // Check if customer with same name & birth date already exists for another customer
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM customers WHERE LOWER(TRIM(name)) = LOWER(TRIM(?1)) AND TRIM(birth_date) = TRIM(?2) AND id != ?3;",
        params![name_trimmed, birth_date_trimmed, id],
        |row| row.get(0),
    ).map_err(|e| format!("Database query error: {}", e))?;

    if count > 0 {
        return Err("PATIENT_ALREADY_EXISTS".into());
    }

    conn.execute(
        "UPDATE customers SET name = ?1, birth_date = ?2, updated_at = datetime('now', 'localtime') WHERE id = ?3;",
        params![name_trimmed, birth_date_trimmed, id],
    ).map_err(|e| format!("Failed to update customer: {}", e))?;

    Ok(())
}

#[tauri::command]
pub fn get_customer_sales(state: State<'_, DbState>, customer_id: i64, admin: bool) -> Result<Vec<LastInvoice>, String> {
    let conn = state.conn.lock().map_err(|_| "Failed to lock database connection")?;
    
    let mut stmt = conn.prepare(
        "SELECT s.id, s.prescribing_doctor_name, s.patient_name, c.name, c.birth_date, s.treatment_period_days, s.total_da, s.created_at, s.is_loan, s.loan_status, s.loan_settled_at 
         FROM sales s
         LEFT JOIN customers c ON s.customer_id = c.id
         WHERE s.customer_id = ?1
         ORDER BY s.id DESC;"
    ).map_err(|e| e.to_string())?;

    let rows = stmt.query_map(params![customer_id], |row| {
        let is_loan_val: i32 = row.get(8).unwrap_or(0);
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, Option<String>>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, Option<String>>(4)?,
            row.get::<_, Option<i32>>(5)?,
            row.get::<_, f64>(6)?,
            row.get::<_, String>(7)?,
            is_loan_val == 1,
            row.get::<_, Option<String>>(9)?,
            row.get::<_, Option<String>>(10)?,
        ))
    }).map_err(|e| e.to_string())?;

    let mut sales_records = Vec::new();
    for r in rows {
        sales_records.push(r.map_err(|e| e.to_string())?);
    }
    
    std::mem::drop(stmt);

    let mut sales = Vec::new();
    for (id, doctor, patient, cust_name, birth_date, treatment_days, total, date, is_loan, loan_status, loan_settled_at) in sales_records {
        let mut item_stmt = conn.prepare(
            "SELECT COALESCE(NULLIF(si.drug_name, ''), d.name, 'Article archivé'), si.quantity_items, si.unit_price_da, si.line_total_da, COALESCE(NULLIF(si.cost_price_da, 0.0), d.cost_price_da, 0.0) 
             FROM sale_items si 
             LEFT JOIN drugs d ON si.drug_id = d.id 
             WHERE si.sale_id = ?1;"
        ).map_err(|e| e.to_string())?;

        let item_rows = item_stmt.query_map(params![id], |item_row| {
            let name: String = item_row.get(0)?;
            let qty: i32 = item_row.get(1)?;
            let unit_price: f64 = item_row.get(2)?;
            let line_total: f64 = item_row.get(3)?;
            let cost_price: f64 = item_row.get(4)?;
            Ok(InvoiceItem {
                drug_name: name,
                quantity_items: qty,
                unit_price_da: unit_price,
                line_total_da: line_total,
                cost_price_da: if admin { Some(cost_price) } else { None },
            })
        }).map_err(|e| e.to_string())?;

        let mut items = Vec::new();
        for ir in item_rows {
            items.push(ir.map_err(|e| e.to_string())?);
        }

        let patient_display = match patient {
            Some(ref p) if !p.trim().is_empty() => Some(p.clone()),
            _ => cust_name,
        };

        sales.push(LastInvoice {
            id,
            prescribing_doctor_name: doctor,
            patient_name: patient_display,
            patient_birth_date: birth_date,
            treatment_period_days: treatment_days,
            total_da: total,
            created_at: date,
            items,
            is_loan: Some(is_loan),
            loan_status,
            loan_settled_at,
        });
    }

    Ok(sales)
}

// -----------------------------------------------------------------------------
// Stock Invoice Import & OCR Validation Engine
// -----------------------------------------------------------------------------

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct ExtractedRawItem {
    pub raw_designation: String,
    pub batch_number: String,
    pub expiry_date: String,
    pub quantity: i32,
    #[serde(alias = "cost_price_da")]
    pub unit_price_da: f64,
    pub ppa_da: f64,
    pub total_da: f64,
    pub math_verified: bool,
    pub page_index: usize,
    #[serde(default)]
    pub tva: Option<f64>,
    #[serde(default)]
    pub mg: Option<f64>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct ExtractedInvoicePayload {
    pub success: bool,
    pub error: Option<String>,
    pub supplier: Option<String>,
    pub invoice_number: Option<String>,
    pub invoice_date: Option<String>,
    pub total_ht: Option<f64>,
    pub discount: Option<f64>,
    pub grand_total: Option<f64>,
    pub pages_rendered: Option<Vec<String>>,
    pub items: Option<Vec<ExtractedRawItem>>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct MatchedDrugCandidate {
    pub id: i64,
    pub name: String,
    pub barcode: String,
    pub price_per_item_da: f64,
    pub cost_price_da: f64,
    pub items_per_package: i32,
    pub match_score: f64,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct AiCandidateDto {
    pub id: i64,
    pub name: String,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct AiItemMatchInput {
    pub index: usize,
    pub raw_designation: String,
    pub candidates: Vec<AiCandidateDto>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct AiMatchDecision {
    pub index: usize,
    pub is_new_drug: bool,
    pub matched_drug_id: Option<i64>,
    pub reason: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
#[allow(dead_code)]
pub struct AiMatchResponse {
    pub success: bool,
    pub error: Option<String>,
    pub decisions: Option<Vec<AiMatchDecision>>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct ValidatedInvoiceItem {
    pub raw_designation: String,
    pub matched_drug: Option<MatchedDrugCandidate>,
    pub all_candidates: Vec<MatchedDrugCandidate>,
    pub is_new_drug: bool,
    pub batch_number: String,
    pub expiry_date: String,
    pub quantity_packages: i32,
    pub cost_price_da: f64,
    pub ppa_da: f64,
    pub total_da: f64,
    pub calculated_total_da: f64,
    pub tva: f64,
    pub mg: f64,
    pub math_status: String,       // "valid", "mismatch"
    pub overall_status: String,    // "valid", "warning", "error"
    pub validation_messages: Vec<String>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct CommitInvoiceItemInput {
    pub drug_id: Option<i64>,
    pub create_new_drug: bool,
    pub new_drug_name: Option<String>,
    pub new_drug_barcode: Option<String>,
    pub new_drug_items_per_package: Option<i32>,
    pub alias_to_save: Option<String>,
    pub batch_number: String,
    pub expiry_date: String,
    pub packages_received: i32,
    pub cost_price_da: f64,
    pub ppa_da: f64,
    #[serde(default)]
    pub tva: Option<f64>,
    #[serde(default)]
    pub mg: Option<f64>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct CommitInvoiceInput {
    pub supplier_name: String,
    pub invoice_number: String,
    pub invoice_date: String,
    pub total_amount_da: f64,
    pub pdf_path: String,
    pub items: Vec<CommitInvoiceItemInput>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct ImportedInvoiceRecord {
    pub id: i64,
    pub supplier_name: String,
    pub invoice_number: String,
    pub invoice_date: String,
    pub total_amount_da: f64,
    pub pdf_path: String,
    pub created_at: String,
    pub items_count: usize,
    pub total_packages: i32,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct InvoiceDrugItem {
    pub drug_id: i64,
    pub name: String,
    pub barcode: String,
    pub price_per_item_da: f64,
    pub packages_received: i32,
    pub batch_number: String,
    pub expiry_date: String,
}

pub fn str_similarity(a: &str, b: &str) -> f64 {
    let a_norm = a.trim().to_lowercase();
    let b_norm = b.trim().to_lowercase();
    if a_norm == b_norm {
        return 1.0;
    }
    if a_norm.is_empty() || b_norm.is_empty() {
        return 0.0;
    }
    if a_norm.contains(&b_norm) || b_norm.contains(&a_norm) {
        return 0.85;
    }
    let s1: Vec<char> = a_norm.chars().collect();
    let s2: Vec<char> = b_norm.chars().collect();
    let len1 = s1.len();
    let len2 = s2.len();
    let mut dp = vec![vec![0; len2 + 1]; len1 + 1];
    for i in 0..=len1 { dp[i][0] = i; }
    for j in 0..=len2 { dp[0][j] = j; }
    for i in 1..=len1 {
        for j in 1..=len2 {
            let cost = if s1[i - 1] == s2[j - 1] { 0 } else { 1 };
            dp[i][j] = std::cmp::min(
                dp[i - 1][j] + 1,
                std::cmp::min(dp[i][j - 1] + 1, dp[i - 1][j - 1] + cost),
            );
        }
    }
    let dist = dp[len1][len2];
    let max_len = std::cmp::max(len1, len2) as f64;
    (1.0 - (dist as f64 / max_len)).max(0.0)
}

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[cfg(debug_assertions)]
fn resolve_python_path() -> String {
    let specific_python = std::path::Path::new("C:\\Python313\\python.exe");
    if specific_python.exists() {
        return "C:\\Python313\\python.exe".to_string();
    }
    "python".to_string()
}

#[cfg(debug_assertions)]
fn resolve_engine_script() -> Result<std::path::PathBuf, String> {
    let direct_path = std::path::PathBuf::from("e:/pos_pharmacy/src-tauri/engine/invoice_extractor.py");
    if direct_path.exists() {
        return Ok(direct_path);
    }
    if let Ok(curr) = std::env::current_dir() {
        let candidate = curr.join("src-tauri").join("engine").join("invoice_extractor.py");
        if candidate.exists() {
            return Ok(candidate);
        }
        let candidate2 = curr.join("engine").join("invoice_extractor.py");
        if candidate2.exists() {
            return Ok(candidate2);
        }
    }
    Err("invoice_extractor.py engine script not found".into())
}

fn resolve_extractor_command() -> Result<Command, String> {
    // 1. Production bundle: check sidecar next to current running executable
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let candidates = [
                exe_dir.join("invoice_extractor.exe"),
                exe_dir.join("invoice_extractor-x86_64-pc-windows-msvc.exe"),
                exe_dir.join("bin").join("invoice_extractor.exe"),
                exe_dir.join("bin").join("invoice_extractor-x86_64-pc-windows-msvc.exe"),
                exe_dir.join("resources").join("invoice_extractor.exe"),
                exe_dir.join("resources").join("invoice_extractor-x86_64-pc-windows-msvc.exe"),
                exe_dir.join("resources").join("bin").join("invoice_extractor.exe"),
                exe_dir.join("..").join("invoice_extractor.exe"),
            ];
            for candidate in candidates {
                if candidate.exists() {
                    let mut cmd = Command::new(candidate);
                    #[cfg(windows)]
                    cmd.creation_flags(CREATE_NO_WINDOW);
                    return Ok(cmd);
                }
            }
        }
    }

    // 2. Development sidecar location in src-tauri/bin
    if let Ok(curr) = std::env::current_dir() {
        let candidates = [
            curr.join("src-tauri").join("bin").join("invoice_extractor-x86_64-pc-windows-msvc.exe"),
            curr.join("src-tauri").join("bin").join("invoice_extractor.exe"),
            curr.join("bin").join("invoice_extractor-x86_64-pc-windows-msvc.exe"),
            curr.join("bin").join("invoice_extractor.exe"),
        ];
        for candidate in candidates {
            if candidate.exists() {
                let mut cmd = Command::new(candidate);
                #[cfg(windows)]
                cmd.creation_flags(CREATE_NO_WINDOW);
                return Ok(cmd);
            }
        }
    }

    // 3. Fallback to Python script — DEVELOPMENT ONLY.
    // Release builds must never depend on a system Python installation.
    #[cfg(debug_assertions)]
    if let Ok(script_path) = resolve_engine_script() {
        let python_exe = resolve_python_path();
        let mut cmd = Command::new(python_exe);
        #[cfg(windows)]
        cmd.creation_flags(CREATE_NO_WINDOW);
        cmd.arg(script_path);
        return Ok(cmd);
    }

    Err("Le module d'analyse locale (invoice_extractor) est introuvable.".into())
}

#[tauri::command]
pub async fn pick_invoice_pdf_file() -> Result<Option<String>, String> {
    let file = rfd::AsyncFileDialog::new()
        .add_filter("Documents PDF (*.pdf)", &["pdf", "PDF"])
        .set_title("Sélectionner la facture d'achat (PDF)")
        .pick_file()
        .await;

    Ok(file.map(|f| f.path().to_string_lossy().to_string()))
}

#[tauri::command]
pub fn scan_and_extract_invoice(
    pdf_path: String,
    app_handle: tauri::AppHandle,
) -> Result<ExtractedInvoicePayload, String> {
    let app_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    let cache_dir = app_dir.join("invoice_render_cache");
    let _ = std::fs::create_dir_all(&cache_dir);

    let mut cmd = match resolve_extractor_command() {
        Ok(c) => c,
        Err(e) => {
            return Err(format!(
                "{}\n\nConseil : Activez l'option 'Appliquer directement l'IA (Désactiver l'OCR hors ligne)' dans les Paramètres pour analyser directement avec Gemini sans avoir besoin de Python.",
                e
            ));
        }
    };
    let output = cmd
        .arg(&pdf_path)
        .arg(&cache_dir)
        .output()
        .map_err(|e| format!("Impossible d'exécuter le moteur OCR: {}.\n\nConseil : Activez 'Appliquer directement l'IA' dans les Paramètres.", e))?;

    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        let out_msg = String::from_utf8_lossy(&output.stdout);
        let detail = if !err_msg.trim().is_empty() {
            err_msg.to_string()
        } else if !out_msg.trim().is_empty() {
            out_msg.to_string()
        } else {
            "Le moteur OCR s'est arrêté inopinément.".to_string()
        };
        return Err(format!("Échec de l'analyse OCR locale : {}\n\nConseil : Vous pouvez activer l'option 'Appliquer directement l'IA' dans Paramètres.", detail.trim()));
    }

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let payload: ExtractedInvoicePayload = serde_json::from_str(&stdout_str)
        .map_err(|e| format!("Erreur lors de la lecture des données extraites : {}", e))?;

    Ok(payload)
}

#[tauri::command]
pub fn get_gemini_api_key(state: State<'_, DbState>) -> Result<String, String> {
    let conn = state.conn.lock().map_err(|_| "Failed to lock database connection")?;
    let db_key: String = conn.query_row(
        "SELECT gemini_api_key FROM settings WHERE id = 1;",
        [],
        |row| row.get(0),
    ).unwrap_or_default();

    if !db_key.trim().is_empty() {
        return Ok(db_key);
    }

    Ok(std::env::var("GEMINI_API_KEY").unwrap_or_default())
}

#[tauri::command]
pub fn save_gemini_api_key(
    state: State<'_, DbState>,
    api_key: String,
) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|_| "Failed to lock database connection")?;
    conn.execute(
        "UPDATE settings SET gemini_api_key = ?1 WHERE id = 1;",
        params![api_key.trim()],
    ).map_err(|e| format!("Failed to save Gemini API key: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn delete_gemini_api_key(state: State<'_, DbState>) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|_| "Failed to lock database connection")?;
    conn.execute(
        "UPDATE settings SET gemini_api_key = '' WHERE id = 1;",
        [],
    ).map_err(|e| format!("Failed to delete Gemini API key: {}", e))?;
    Ok(())
}

const GEMINI_MODELS: &[&str] = &[
    "gemini-3.5-flash-lite",
    "gemini-3.1-flash-lite",
    "gemini-flash-lite-latest",
    "gemini-3.5-flash",
    "gemini-2.5-flash",
];

fn call_gemini_api(
    api_key: &str,
    prompt: &str,
    pdf_base64: Option<&str>,
) -> Result<String, String> {
    let mut parts = vec![serde_json::json!({ "text": prompt })];
    if let Some(b64) = pdf_base64 {
        parts.push(serde_json::json!({
            "inline_data": {
                "mime_type": "application/pdf",
                "data": b64
            }
        }));
    }

    let request_body = serde_json::json!({
        "contents": [{
            "parts": parts
        }],
        "generationConfig": {
            "response_mime_type": "application/json",
            "temperature": 0.1
        }
    });

    let mut last_error = String::from("Échec de connexion au service d'analyse IA.");

    for model_name in GEMINI_MODELS {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            model_name, api_key
        );

        match ureq::post(&url)
            .header("Content-Type", "application/json")
            .send_json(&request_body)
        {
            Ok(mut resp) => {
                let resp_json: serde_json::Value = resp
                    .body_mut()
                    .read_json()
                    .map_err(|e| format!("Erreur lors de la lecture de la réponse IA: {}", e))?;

                if let Some(candidates) = resp_json.get("candidates").and_then(|c| c.as_array()) {
                    if let Some(first_cand) = candidates.first() {
                        if let Some(parts) = first_cand.get("content").and_then(|c| c.get("parts")).and_then(|p| p.as_array()) {
                            if let Some(first_part) = parts.first() {
                                if let Some(text) = first_part.get("text").and_then(|t| t.as_str()) {
                                    let mut clean_text = text.trim();
                                    if clean_text.starts_with("```json") {
                                        clean_text = clean_text.trim_start_matches("```json").trim();
                                    } else if clean_text.starts_with("```") {
                                        clean_text = clean_text.trim_start_matches("```").trim();
                                    }
                                    if clean_text.ends_with("```") {
                                        clean_text = clean_text.trim_end_matches("```").trim();
                                    }
                                    return Ok(clean_text.to_string());
                                }
                            }
                        }
                    }
                }
                last_error = "Aucune donnée n'a été renvoyée par le service d'analyse IA.".to_string();
            }
            Err(ureq::Error::StatusCode(code)) => {
                if code == 400 || code == 403 {
                    return Err("Clé API invalide ou non autorisée. Veuillez vérifier votre clé dans les Paramètres.".into());
                } else if code == 429 {
                    last_error = "Quota de requêtes dépassé. Veuillez patienter quelques instants avant de réessayer.".into();
                } else if code == 503 {
                    last_error = "Le service d'analyse IA est momentanément surchargé. Veuillez réessayer dans quelques instants.".into();
                } else if code == 404 {
                    last_error = "Le modèle d'IA sélectionné est temporairement indisponible.".into();
                } else {
                    last_error = format!("Erreur du service IA (code {}).", code);
                }
            }
            Err(e) => {
                last_error = format!("Impossible de joindre le service IA: {}", e);
            }
        }
    }

    Err(last_error)
}

fn parse_f64_val(val: Option<&serde_json::Value>) -> f64 {
    match val {
        Some(serde_json::Value::Number(n)) => n.as_f64().unwrap_or(0.0),
        Some(serde_json::Value::String(s)) => {
            let cleaned = s.replace(' ', "").replace(',', ".").replace("DA", "").replace("da", "").replace('%', "");
            cleaned.parse::<f64>().unwrap_or(0.0)
        }
        _ => 0.0,
    }
}

fn parse_i32_val(val: Option<&serde_json::Value>) -> i32 {
    match val {
        Some(serde_json::Value::Number(n)) => n.as_i64().unwrap_or(1) as i32,
        Some(serde_json::Value::String(s)) => {
            let cleaned: String = s.chars().filter(|c| c.is_ascii_digit() || *c == '-').collect();
            cleaned.parse::<i32>().unwrap_or(1)
        }
        _ => 1,
    }
}

fn normalize_expiry_date(exp: &str) -> String {
    let trimmed = exp.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let parts: Vec<&str> = trimmed.split(['/', '-', '.']).collect();
    if parts.len() == 2 {
        if let (Ok(m), Ok(y)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
            let full_year = if y < 100 { 2000 + y } else { y };
            if (1..=12).contains(&m) && (2024..=2045).contains(&full_year) {
                return format!("{:04}-{:02}-28", full_year, m);
            }
        }
    }
    trimmed.to_string()
}

#[tauri::command]
pub fn test_gemini_api_key(
    api_key: Option<String>,
    state: State<'_, DbState>,
) -> Result<String, String> {
    let final_key = match api_key.filter(|k| !k.trim().is_empty()) {
        Some(k) => k,
        None => {
            let conn = state.conn.lock().map_err(|_| "Failed to lock database connection")?;
            let db_key: String = conn.query_row(
                "SELECT gemini_api_key FROM settings WHERE id = 1;",
                [],
                |row| row.get(0),
            ).unwrap_or_default();
            if !db_key.trim().is_empty() {
                db_key
            } else {
                std::env::var("GEMINI_API_KEY").unwrap_or_default()
            }
        }
    };

    if final_key.trim().is_empty() {
        return Err("Veuillez saisir votre clé API.".into());
    }

    call_gemini_api(
        &final_key,
        "Réponds uniquement au format JSON: {\"success\": true, \"message\": \"Clé API valide et opérationnelle !\"}",
        None,
    )?;

    Ok("Clé API valide et opérationnelle !".into())
}

#[tauri::command]
pub fn extract_invoice_with_gemini(
    pdf_path: String,
    api_key: Option<String>,
    _app_handle: tauri::AppHandle,
    state: State<'_, DbState>,
) -> Result<ExtractedInvoicePayload, String> {
    let final_key = match api_key.filter(|k| !k.trim().is_empty()) {
        Some(k) => k,
        None => {
            let conn = state.conn.lock().map_err(|_| "Failed to lock database connection")?;
            let db_key: String = conn.query_row(
                "SELECT gemini_api_key FROM settings WHERE id = 1;",
                [],
                |row| row.get(0),
            ).unwrap_or_default();
            if !db_key.trim().is_empty() {
                db_key
            } else {
                std::env::var("GEMINI_API_KEY").unwrap_or_default()
            }
        }
    };

    if final_key.trim().is_empty() {
        return Err("Clé API non configurée. Veuillez renseigner votre clé API dans les Paramètres.".into());
    }

    let pdf_bytes = std::fs::read(&pdf_path)
        .map_err(|e| format!("Impossible de lire le fichier: {}", e))?;
    let b64_pdf = BASE64_STANDARD.encode(&pdf_bytes);

    let prompt_text = "\
Tu es un expert en facturation pharmaceutique en Algérie (grossistes répartiteurs: \
Millennium Medic, Pharma Spot, Setif Medic, Biopharm, CPA, etc.).\n\
Analyse attentivement cette facture d'achat de médicaments et extrait rigoureusement \
les métadonnées et toutes les lignes d'articles de la facture.\n\n\
Format de sortie JSON obligatoire et strict:\n\
{\n\
  \"supplier\": \"Nom du grossiste\",\n\
  \"invoice_number\": \"N° Facture ou N° BL\",\n\
  \"invoice_date\": \"YYYY-MM-DD\",\n\
  \"total_brut\": 0.0,\n\
  \"discount\": 0.0,\n\
  \"grand_total\": 0.0,\n\
  \"items\": [\n\
    {\n\
      \"raw_designation\": \"Nom complet du médicament avec dosage et forme (ex: DOLIPRANE 1000MG CPR)\",\n\
      \"quantity\": 10,\n\
      \"batch_number\": \"Numéro de Lot (ex: 23H091)\",\n\
      \"expiry_date\": \"YYYY-MM-DD\",\n\
      \"ppa_da\": 250.0,\n\
      \"cost_price_da\": 180.0,\n\
      \"tva\": 0.0,\n\
      \"mg\": 20.0,\n\
      \"total_da\": 1800.0\n\
    }\n\
  ]\n\
}\n\n\
Règles impératives:\n\
1. Extraire TOUTES les lignes de médicaments du tableau, sans en omettre aucune.\n\
2. raw_designation : nom complet avec dosage et forme.\n\
3. quantity : nombre de boîtes facturées (QTE).\n\
4. batch_number : numéro de lot.\n\
5. expiry_date : date d'expiration exacte au format YYYY-MM-DD (ex: 11/27 -> 2027-11-28).\n\
6. ppa_da : Prix Public Algérien (PPA / P.Vente), toujours supérieur au prix d'achat PUHT.\n\
7. cost_price_da : Prix Unitaire Hors Taxe (PUHT / P.U.Ht / P.Achat).\n\
8. tva : Taux de TVA (0.0, 9.0 ou 19.0). Si exonéré ou 0, tva = 0.0.\n\
9. mg : Marge bénéficiaire (MG / Mge / Marge). Si absente ou 0, calculer: ((ppa_da - cost_price_da * (1 + tva/100)) / (cost_price_da * (1 + tva/100))) * 100.\n\
10. total_da = quantity * cost_price_da.\n\
11. total_brut : Montant total brut HT des articles avant remise.\n\
12. discount : Montant de la remise globale / ristourne, sinon 0.0.\n\
13. grand_total : Montant Net à Payer (Total TTC ou Net HT).\n\
14. Renvoie UNIQUEMENT le JSON valide sans texte additionnel.";

    let json_text = call_gemini_api(&final_key, prompt_text, Some(&b64_pdf))?;
    let parsed: serde_json::Value = serde_json::from_str(&json_text)
        .map_err(|e| format!("Erreur d'analyse JSON du résultat IA: {}", e))?;

    let mut items = Vec::new();
    let raw_items_opt = parsed.get("items")
        .or_else(|| parsed.get("medicaments"))
        .or_else(|| parsed.get("articles"))
        .or_else(|| parsed.get("data"))
        .and_then(|v| v.as_array());

    if let Some(arr) = raw_items_opt {
        for it in arr {
            let desig = it.get("raw_designation").and_then(|v| v.as_str()).unwrap_or("").trim();
            if desig.is_empty() {
                continue;
            }
            let qty = parse_i32_val(it.get("quantity")).max(1);
            let lot = it.get("batch_number").and_then(|v| v.as_str()).unwrap_or("LOT-AUTO").trim();
            let raw_exp = it.get("expiry_date").and_then(|v| v.as_str()).unwrap_or("").trim();
            let exp = normalize_expiry_date(raw_exp);

            let mut puht = parse_f64_val(it.get("cost_price_da").or_else(|| it.get("unit_price_da")));
            let mut ppa = parse_f64_val(it.get("ppa_da"));
            if puht > 0.0 && ppa > 0.0 && puht > ppa {
                std::mem::swap(&mut puht, &mut ppa);
            }
            let tva = parse_f64_val(it.get("tva"));
            let mut mg = parse_f64_val(it.get("mg"));
            if mg == 0.0 && puht > 0.0 && ppa > puht {
                let base = if tva > 0.0 { puht * (1.0 + tva / 100.0) } else { puht };
                mg = ((ppa - base) / base * 100.0 * 10.0).round() / 10.0;
            }
            let total = it.get("total_da")
                .map(|v| parse_f64_val(Some(v)))
                .unwrap_or_else(|| (qty as f64) * puht);
            let math_verified = (total - (qty as f64) * puht).abs() <= 0.5;

            items.push(ExtractedRawItem {
                raw_designation: desig.to_string(),
                batch_number: if lot.is_empty() { "LOT-AUTO".to_string() } else { lot.to_string() },
                expiry_date: exp,
                quantity: qty,
                unit_price_da: (puht * 100.0).round() / 100.0,
                ppa_da: (ppa * 100.0).round() / 100.0,
                total_da: (total * 100.0).round() / 100.0,
                math_verified,
                page_index: 0,
                tva: Some(tva),
                mg: Some(mg),
            });
        }
    }

    let supplier = parsed.get("supplier").and_then(|v| v.as_str()).unwrap_or("Grossiste Pharmacie").to_string();
    let invoice_number = parsed.get("invoice_number").and_then(|v| v.as_str()).unwrap_or("AUTO-INV").to_string();
    let invoice_date = parsed.get("invoice_date").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let total_brut = parse_f64_val(parsed.get("total_brut"));
    let mut grand_total = parse_f64_val(parsed.get("grand_total"));

    let items_sum: f64 = items.iter().map(|i| i.total_da).sum();
    if grand_total == 0.0 && !items.is_empty() {
        grand_total = (items_sum * 100.0).round() / 100.0;
    }

    let pdf_data_url = format!("data:application/pdf;base64,{}", b64_pdf);

    Ok(ExtractedInvoicePayload {
        success: true,
        error: None,
        supplier: Some(supplier),
        invoice_number: Some(invoice_number),
        invoice_date: Some(invoice_date),
        total_ht: Some((total_brut * 100.0).round() / 100.0),
        discount: Some(0.0),
        grand_total: Some((grand_total * 100.0).round() / 100.0),
        pages_rendered: Some(vec![pdf_data_url]),
        items: Some(items),
    })
}

#[tauri::command]
pub fn ai_match_invoice_drugs(
    items: Vec<AiItemMatchInput>,
    api_key: Option<String>,
    _app_handle: tauri::AppHandle,
    state: State<'_, DbState>,
) -> Result<Vec<AiMatchDecision>, String> {
    if items.is_empty() {
        return Ok(Vec::new());
    }

    let final_key = match api_key.filter(|k| !k.trim().is_empty()) {
        Some(k) => k,
        None => {
            let conn = state.conn.lock().map_err(|_| "Failed to lock database connection")?;
            let db_key: String = conn.query_row(
                "SELECT gemini_api_key FROM settings WHERE id = 1;",
                [],
                |row| row.get(0),
            ).unwrap_or_default();
            if !db_key.trim().is_empty() {
                db_key
            } else {
                std::env::var("GEMINI_API_KEY").unwrap_or_default()
            }
        }
    };

    if final_key.trim().is_empty() {
        return Err("Clé API manquante. Veuillez configurer votre clé dans les Paramètres.".into());
    }

    let payload_json = serde_json::to_string(&items).map_err(|e| e.to_string())?;
    let prompt_text = format!(
        "Tu es un pharmacien expert. Associe chaque médicament extrait de la facture à la liste des médicaments candidats de notre catalogue.\n\
        Données d'entrée (format JSON):\n{}\n\n\
        Format de sortie JSON obligatoire et strict:\n\
        {{\n  \"decisions\": [\n    {{\n      \"index\": 0,\n      \"is_new_drug\": false,\n      \"matched_drug_id\": 123,\n      \"reason\": \"Correspondance exacte\"\n    }}\n  ]\n}}\n\
        Règles:\n\
        - Si un candidat correspond fidèlement (même nom de molécule, même forme, même dosage), associe-le avec son matched_drug_id et is_new_drug = false.\n\
        - Si aucun candidat ne correspond ou s'il y a un doute significatif, is_new_drug = true et matched_drug_id = null.\n\
        - Renvoie UNIQUEMENT le JSON valide sans texte additionnel.",
        payload_json
    );

    let json_text = call_gemini_api(&final_key, &prompt_text, None)?;
    let parsed: serde_json::Value = serde_json::from_str(&json_text)
        .map_err(|e| format!("Erreur d'analyse de la réponse IA: {}", e))?;

    let mut decisions = Vec::new();
    if let Some(arr) = parsed.get("decisions").and_then(|v| v.as_array()) {
        for d in arr {
            let idx = parse_i32_val(d.get("index")) as usize;
            let is_new = d.get("is_new_drug").and_then(|v| v.as_bool()).unwrap_or(true);
            let matched_id = d.get("matched_drug_id").and_then(|v| v.as_i64());
            let reason = d.get("reason").and_then(|v| v.as_str()).map(|s| s.to_string());
            decisions.push(AiMatchDecision {
                index: idx,
                is_new_drug: is_new,
                matched_drug_id: matched_id,
                reason,
            });
        }
    }

    Ok(decisions)
}

#[tauri::command]
pub fn validate_and_match_invoice(
    state: State<'_, DbState>,
    items: Vec<ExtractedRawItem>,
) -> Result<Vec<ValidatedInvoiceItem>, String> {
    let conn = state.conn.lock().map_err(|_| "Failed to lock database connection")?;

    // Load all current drugs for fuzzy matching
    let mut stmt = conn.prepare(
        "SELECT id, barcode, name, price_per_item_da, cost_price_da, items_per_package FROM drugs;",
    ).map_err(|e| e.to_string())?;

    let all_drugs: Vec<MatchedDrugCandidate> = stmt
        .query_map([], |row| {
            Ok(MatchedDrugCandidate {
                id: row.get(0)?,
                barcode: row.get(1)?,
                name: row.get(2)?,
                price_per_item_da: row.get(3)?,
                cost_price_da: row.get(4)?,
                items_per_package: row.get(5)?,
                match_score: 0.0,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let mut validated_list = Vec::new();

    for item in items {
        let mut messages = Vec::new();
        let mut matched_drug: Option<MatchedDrugCandidate> = None;
        let mut candidate_list = Vec::new();

        let raw_clean = item.raw_designation.trim();

        // 1. Check drug_aliases table first
        let alias_opt: Option<(i64, String, String, f64, f64, i32)> = conn.query_row(
            "SELECT d.id, d.barcode, d.name, d.price_per_item_da, d.cost_price_da, d.items_per_package 
             FROM drug_aliases a 
             JOIN drugs d ON a.drug_id = d.id 
             WHERE LOWER(a.alias_name) = LOWER(?1);",
            params![raw_clean],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)),
        ).ok();

        if let Some((id, barcode, name, price, cost, items_pkg)) = alias_opt {
            matched_drug = Some(MatchedDrugCandidate {
                id,
                barcode,
                name,
                price_per_item_da: price,
                cost_price_da: cost,
                items_per_package: items_pkg,
                match_score: 1.0,
            });
        } else {
            // 2. Exact match in drugs
            if let Some(exact) = all_drugs.iter().find(|d| d.name.eq_ignore_ascii_case(raw_clean)) {
                let mut found = exact.clone();
                found.match_score = 1.0;
                matched_drug = Some(found);
            } else {
                // 3. Fuzzy search in drugs
                let mut scored: Vec<MatchedDrugCandidate> = all_drugs
                    .iter()
                    .map(|d| {
                        let score = str_similarity(raw_clean, &d.name);
                        let mut c = d.clone();
                        c.match_score = score;
                        c
                    })
                    .filter(|d| d.match_score >= 0.45)
                    .collect();

                scored.sort_by(|a, b| b.match_score.partial_cmp(&a.match_score).unwrap_or(std::cmp::Ordering::Equal));
                if let Some(best) = scored.first() {
                    if best.match_score >= 0.65 {
                        matched_drug = Some(best.clone());
                    }
                }
                candidate_list = scored.into_iter().take(5).collect();
            }
        }

        // Math checks
        let calc_total = (item.quantity as f64) * item.unit_price_da;
        let diff = (calc_total - item.total_da).abs();
        let math_status = if diff <= 1.0 {
            "valid".to_string()
        } else {
            messages.push(format!("Écart calcul: {} x {} = {:.2} DA (Facture: {:.2} DA)", item.quantity, item.unit_price_da, calc_total, item.total_da));
            "mismatch".to_string()
        };

        // Expiry date checks
        if item.expiry_date.trim().is_empty() {
            messages.push("Date de péremption manquante ou non reconnue.".to_string());
        }

        // Overall status
        let is_new = matched_drug.is_none();
        let overall_status = if math_status == "mismatch" {
            "error".to_string()
        } else if is_new || item.expiry_date.trim().is_empty() {
            "warning".to_string()
        } else {
            "valid".to_string()
        };

        validated_list.push(ValidatedInvoiceItem {
            raw_designation: item.raw_designation,
            matched_drug,
            all_candidates: candidate_list,
            is_new_drug: is_new,
            batch_number: item.batch_number,
            expiry_date: item.expiry_date,
            quantity_packages: item.quantity,
            cost_price_da: item.unit_price_da,
            ppa_da: item.ppa_da,
            total_da: item.total_da,
            calculated_total_da: calc_total,
            tva: item.tva.unwrap_or(0.0),
            mg: item.mg.unwrap_or(0.0),
            math_status,
            overall_status,
            validation_messages: messages,
        });
    }

    Ok(validated_list)
}

pub fn run_commit_imported_invoice_transaction(
    tx: &rusqlite::Transaction,
    invoice_data: CommitInvoiceInput,
) -> Result<i64, String> {
    if invoice_data.items.is_empty() {
        return Err("Aucun article dans la facture à intégrer.".into());
    }

    // 1. Insert into imported_invoices
    tx.execute(
        "INSERT INTO imported_invoices (supplier_name, invoice_number, invoice_date, total_amount_da, pdf_path)
         VALUES (?1, ?2, ?3, ?4, ?5);",
        params![
            invoice_data.supplier_name.trim(),
            invoice_data.invoice_number.trim(),
            invoice_data.invoice_date.trim(),
            invoice_data.total_amount_da,
            invoice_data.pdf_path.trim()
        ],
    ).map_err(|e| format!("Failed to record imported invoice: {}", e))?;

    let invoice_id = tx.last_insert_rowid();
    let mut saved_items: Vec<InvoiceDrugItem> = Vec::new();

    // 2. Process each item
    for item in invoice_data.items {
        let final_drug_id: i64;
        let item_tva = item.tva.unwrap_or(0.0);
        let item_mg = item.mg.unwrap_or(0.0);

        // Check if drug already exists (by drug_id, barcode, alias, or exact name)
        let mut existing_id: Option<i64> = item.drug_id;

        if existing_id.is_none() {
            if let Some(ref bc) = item.new_drug_barcode {
                let bc_trim = bc.trim();
                if !bc_trim.is_empty() {
                    existing_id = tx.query_row(
                        "SELECT id FROM drugs WHERE barcode = ?1;",
                        params![bc_trim],
                        |row| row.get(0),
                    ).ok();
                }
            }
        }

        let raw_name = item.new_drug_name.clone().unwrap_or_else(|| "Nouveau Médicament".to_string());
        let raw_clean = raw_name.trim();

        if existing_id.is_none() {
            existing_id = tx.query_row(
                "SELECT drug_id FROM drug_aliases WHERE LOWER(alias_name) = LOWER(?1);",
                params![raw_clean],
                |row| row.get(0),
            ).ok();
        }

        if existing_id.is_none() {
            existing_id = tx.query_row(
                "SELECT id FROM drugs WHERE LOWER(TRIM(name)) = LOWER(?1);",
                params![raw_clean],
                |row| row.get(0),
            ).ok();
        }

        if let Some(id) = existing_id {
            final_drug_id = id;
            // Update master drug latest prices, tva, and mg if provided
            if item.cost_price_da > 0.0 {
                let _ = tx.execute(
                    "UPDATE drugs SET cost_price_da = ?1, price_per_item_da = ?2, tva = ?3, mg = ?4, updated_at = datetime('now', 'localtime') WHERE id = ?5;",
                    params![item.cost_price_da, item.ppa_da, item_tva, item_mg, final_drug_id],
                );
            }
        } else {
            // Drug does not exist: create it with unique 6-digit barcode if none provided
            let barcode = match item.new_drug_barcode {
                Some(b) if !b.trim().is_empty() => b.trim().to_string(),
                _ => generate_unique_6digit_barcode(tx)?,
            };
            let items_per_pkg = item.new_drug_items_per_package.unwrap_or(10).max(1);

            tx.execute(
                "INSERT INTO drugs (barcode, name, requires_prescription, price_per_item_da, cost_price_da, items_per_package, tva, mg)
                 VALUES (?1, ?2, 0, ?3, ?4, ?5, ?6, ?7);",
                params![
                    barcode,
                    raw_clean,
                    item.ppa_da,
                    item.cost_price_da,
                    items_per_pkg,
                    item_tva,
                    item_mg
                ],
            ).map_err(|e| format!("Failed to create new drug: {}", e))?;

            final_drug_id = tx.last_insert_rowid();
        }

        // Save alias to drug_aliases if provided
        if let Some(alias) = item.alias_to_save {
            let alias_trim = alias.trim();
            if !alias_trim.is_empty() {
                let _ = tx.execute(
                    "INSERT OR IGNORE INTO drug_aliases (drug_id, alias_name) VALUES (?1, ?2);",
                    params![final_drug_id, alias_trim],
                );
            }
        }

        // Fetch items_per_package to calculate items_remaining
        let items_per_package: i32 = tx.query_row(
            "SELECT items_per_package FROM drugs WHERE id = ?1;",
            params![final_drug_id],
            |row| row.get(0),
        ).unwrap_or(1);

        let total_units = item.packages_received * items_per_package;
        let batch_num = if item.batch_number.trim().is_empty() {
            "LOT-AUTO".to_string()
        } else {
            item.batch_number.trim().to_string()
        };

        let expiry = if item.expiry_date.trim().is_empty() {
            // Default 2 years ahead if unspecified
            (chrono::Local::now() + chrono::Duration::days(730)).format("%Y-%m-%d").to_string()
        } else {
            item.expiry_date.trim().to_string()
        };

        // Insert stock batch with batch-specific pricing and invoice link
        tx.execute(
            "INSERT INTO stock_batches (drug_id, batch_number, expiry_date, packages_received, items_remaining, price_per_item_da, cost_price_da, tva, mg, invoice_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10);",
            params![
                final_drug_id,
                batch_num,
                expiry,
                item.packages_received,
                total_units,
                item.ppa_da,
                item.cost_price_da,
                item_tva,
                item_mg,
                invoice_id
            ],
        ).map_err(|e| format!("Failed to create stock batch: {}", e))?;

        let (drug_name, drug_barcode, drug_price): (String, String, f64) = tx.query_row(
            "SELECT name, barcode, price_per_item_da FROM drugs WHERE id = ?1;",
            params![final_drug_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        ).unwrap_or((raw_clean.to_string(), "".to_string(), item.ppa_da));

        saved_items.push(InvoiceDrugItem {
            drug_id: final_drug_id,
            name: drug_name,
            barcode: drug_barcode,
            price_per_item_da: drug_price,
            packages_received: item.packages_received,
            batch_number: batch_num,
            expiry_date: expiry,
        });
    }

    let raw_json_str = serde_json::to_string(&saved_items).unwrap_or_default();
    let _ = tx.execute(
        "UPDATE imported_invoices SET raw_json = ?1 WHERE id = ?2;",
        params![raw_json_str, invoice_id],
    );

    Ok(invoice_id)
}

#[tauri::command]
pub fn commit_imported_invoice(
    state: State<'_, DbState>,
    invoice_data: CommitInvoiceInput,
) -> Result<i64, String> {
    let mut conn = state.conn.lock().map_err(|_| "Failed to lock database connection")?;
    let tx = conn.transaction().map_err(|e| format!("Failed to start transaction: {}", e))?;
    let invoice_id = run_commit_imported_invoice_transaction(&tx, invoice_data)?;
    tx.commit().map_err(|e| format!("Transaction commit failed: {}", e))?;
    Ok(invoice_id)
}

#[tauri::command]
pub fn get_imported_invoices(state: State<'_, DbState>) -> Result<Vec<ImportedInvoiceRecord>, String> {
    let conn = state.conn.lock().map_err(|_| "Failed to lock database connection")?;
    let mut stmt = conn.prepare(
        "SELECT id, supplier_name, invoice_number, invoice_date, total_amount_da, pdf_path, created_at, raw_json 
         FROM imported_invoices 
         ORDER BY id DESC;",
    ).map_err(|e| e.to_string())?;

    let rows = stmt.query_map([], |row| {
        let raw_json: String = row.get(7).unwrap_or_default();
        let items: Vec<InvoiceDrugItem> = serde_json::from_str(&raw_json).unwrap_or_default();
        let items_count = items.len();
        let total_packages: i32 = items.iter().map(|it| it.packages_received).sum();

        Ok(ImportedInvoiceRecord {
            id: row.get(0)?,
            supplier_name: row.get(1)?,
            invoice_number: row.get(2)?,
            invoice_date: row.get(3)?,
            total_amount_da: row.get(4)?,
            pdf_path: row.get(5)?,
            created_at: row.get(6)?,
            items_count,
            total_packages,
        })
    }).map_err(|e| e.to_string())?;

    let mut list = Vec::new();
    for r in rows {
        list.push(r.map_err(|e| e.to_string())?);
    }
    Ok(list)
}

#[tauri::command]
pub fn delete_imported_invoice(
    state: State<'_, DbState>,
    invoice_id: i64,
    delete_stock: bool,
) -> Result<(), String> {
    let mut conn = state.conn.lock().map_err(|_| "Failed to lock database connection")?;

    conn.execute("PRAGMA foreign_keys = OFF;", [])
        .map_err(|e| format!("Failed to disable foreign keys: {}", e))?;

    let res = (|| -> Result<(), rusqlite::Error> {
        let tx = conn.transaction()?;

        if delete_stock {
            // Unlink any sales items that referenced batches from this invoice so historical sales remain intact
            tx.execute(
                "UPDATE sale_items SET batch_id = NULL 
                 WHERE batch_id IN (SELECT id FROM stock_batches WHERE invoice_id = ?1);",
                params![invoice_id],
            )?;

            // Delete the stock batches imported with this invoice
            tx.execute(
                "DELETE FROM stock_batches WHERE invoice_id = ?1;",
                params![invoice_id],
            )?;
        } else {
            // Keep stock batches, but unlink them from the invoice (treated as unknown invoice / manual)
            tx.execute(
                "UPDATE stock_batches SET invoice_id = NULL WHERE invoice_id = ?1;",
                params![invoice_id],
            )?;
        }

        // Delete the imported invoice record
        tx.execute("DELETE FROM imported_invoices WHERE id = ?1;", params![invoice_id])?;

        tx.commit()?;
        Ok(())
    })();

    let _ = conn.execute("PRAGMA foreign_keys = ON;", []);
    res.map_err(|e| format!("Impossible de supprimer la facture: {}", e))
}

#[tauri::command]
pub fn open_invoice_file(path: String) -> Result<(), String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("Chemin de fichier manquant".to_string());
    }
    let p = std::path::Path::new(trimmed);
    if !p.exists() {
        return Err(format!("Le fichier n'existe pas: {}", trimmed));
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(trimmed)
            .spawn()
            .map_err(|e| format!("Impossible d'ouvrir le fichier : {}", e))?;
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::process::Command::new("xdg-open")
            .arg(trimmed)
            .spawn()
            .map_err(|e| format!("Impossible d'ouvrir le fichier : {}", e))?;
    }
    Ok(())
}

#[tauri::command]
pub fn get_imported_invoice_drugs(
    state: State<'_, DbState>,
    invoice_id: i64,
) -> Result<Vec<InvoiceDrugItem>, String> {
    let conn = state.conn.lock().map_err(|_| "Failed to lock database connection")?;

    let raw_json: String = conn.query_row(
        "SELECT raw_json FROM imported_invoices WHERE id = ?1;",
        params![invoice_id],
        |row| row.get(0),
    ).unwrap_or_default();

    if !raw_json.trim().is_empty() {
        if let Ok(mut items) = serde_json::from_str::<Vec<InvoiceDrugItem>>(&raw_json) {
            if !items.is_empty() {
                for item in &mut items {
                    if item.barcode.is_empty() {
                        if let Ok(bc) = conn.query_row(
                            "SELECT barcode FROM drugs WHERE id = ?1;",
                            params![item.drug_id],
                            |r| r.get::<_, String>(0),
                        ) {
                            item.barcode = bc;
                        }
                    }
                }
                return Ok(items);
            }
        }
    }

    // Fallback: search stock_batches around invoice creation date or matching invoice_id
    let mut stmt = conn.prepare(
        "SELECT DISTINCT d.id, d.name, d.barcode, sb.price_per_item_da, sb.packages_received, sb.batch_number, sb.expiry_date
         FROM stock_batches sb
         JOIN drugs d ON sb.drug_id = d.id
         WHERE sb.invoice_id = ?1 OR (date(sb.received_at) = (SELECT date(created_at) FROM imported_invoices WHERE id = ?1))
         ORDER BY d.name ASC;",
    ).map_err(|e| e.to_string())?;

    let rows = stmt.query_map(params![invoice_id], |row| {
        Ok(InvoiceDrugItem {
            drug_id: row.get(0)?,
            name: row.get(1)?,
            barcode: row.get(2)?,
            price_per_item_da: row.get(3)?,
            packages_received: row.get(4)?,
            batch_number: row.get(5)?,
            expiry_date: row.get(6)?,
        })
    }).map_err(|e| e.to_string())?;

    let mut list = Vec::new();
    for r in rows {
        list.push(r.map_err(|e| e.to_string())?);
    }
    Ok(list)
}

#[tauri::command]
pub fn get_imported_invoice_by_id(
    state: State<'_, DbState>,
    invoice_id: i64,
) -> Result<Option<ImportedInvoiceRecord>, String> {
    let conn = state.conn.lock().map_err(|_| "Failed to lock database connection")?;
    let mut stmt = conn.prepare(
        "SELECT id, supplier_name, invoice_number, invoice_date, total_amount_da, pdf_path, created_at, raw_json 
         FROM imported_invoices 
         WHERE id = ?1;"
    ).map_err(|e| e.to_string())?;

    let mut rows = stmt.query_map(params![invoice_id], |row| {
        let raw_json: String = row.get(7).unwrap_or_default();
        let items: Vec<InvoiceDrugItem> = serde_json::from_str(&raw_json).unwrap_or_default();
        let items_count = items.len();
        let total_packages: i32 = items.iter().map(|it| it.packages_received).sum();

        Ok(ImportedInvoiceRecord {
            id: row.get(0)?,
            supplier_name: row.get(1)?,
            invoice_number: row.get(2)?,
            invoice_date: row.get(3)?,
            total_amount_da: row.get(4)?,
            pdf_path: row.get(5)?,
            created_at: row.get(6)?,
            items_count,
            total_packages,
        })
    }).map_err(|e| e.to_string())?;

    if let Some(res) = rows.next() {
        Ok(Some(res.map_err(|e| e.to_string())?))
    } else {
        Ok(None)
    }
}

#[tauri::command]
pub fn get_last_imported_invoice_drugs(
    state: State<'_, DbState>,
) -> Result<Vec<InvoiceDrugItem>, String> {
    let conn = state.conn.lock().map_err(|_| "Failed to lock database connection")?;
    let last_id: Option<i64> = conn.query_row(
        "SELECT id FROM imported_invoices ORDER BY id DESC LIMIT 1;",
        [],
        |row| row.get(0),
    ).ok();
    drop(conn);

    match last_id {
        Some(id) => get_imported_invoice_drugs(state, id),
        None => Ok(vec![]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::migrations::run_migrations(&conn).unwrap();
        conn
    }

    #[test]
    fn test_fefo_deduction() {
        let mut conn = setup_test_db();
        
        // Insert a drug
        conn.execute(
            "INSERT INTO drugs (barcode, name, requires_prescription, price_per_item_da, cost_price_da, items_per_package) 
             VALUES ('123456', 'Doliprane', 0, 200.0, 150.0, 10);",
            []
        ).unwrap();
        let drug_id = conn.last_insert_rowid();

        // Insert stock batches:
        // Batch 1: expires 2026-08-01 (expires first), remaining = 5
        conn.execute(
            "INSERT INTO stock_batches (drug_id, expiry_date, packages_received, items_remaining) 
             VALUES (?1, '2026-08-01', 1, 5);",
            params![drug_id]
        ).unwrap();
        let batch1_id = conn.last_insert_rowid();

        // Batch 2: expires 2026-12-01 (expires second), remaining = 10
        conn.execute(
            "INSERT INTO stock_batches (drug_id, expiry_date, packages_received, items_remaining) 
             VALUES (?1, '2026-12-01', 1, 10);",
            params![drug_id]
        ).unwrap();
        let batch2_id = conn.last_insert_rowid();

        let tx = conn.transaction().unwrap();

        // Checkout 8 items of Doliprane
        let cart = vec![CartItemInput {
            drug_id,
            quantity_items: 8,
            unit_price_da: 200.0,
        }];

        let sale_id = run_checkout_transaction_logic(
            &tx,
            None,
            Some("Client Test".to_string()),
            None,
            None,
            Some(30),
            cart,
            None,
            None,
        ).unwrap();

        tx.commit().unwrap();

        // Verify sale was created and treatment period days recorded
        assert!(sale_id > 0);
        let recorded_period: Option<i32> = conn.query_row(
            "SELECT treatment_period_days FROM sales WHERE id = ?1;",
            params![sale_id],
            |row| row.get(0)
        ).unwrap();
        assert_eq!(recorded_period, Some(30));

        // Verify stock deduction:
        // Batch 1 (expires first) should be completely exhausted (0 remaining)
        // Batch 2 should have 7 remaining (10 - (8 - 5))
        let batch1_rem: i32 = conn.query_row(
            "SELECT items_remaining FROM stock_batches WHERE id = ?1;",
            params![batch1_id],
            |row| row.get(0)
        ).unwrap();
        let batch2_rem: i32 = conn.query_row(
            "SELECT items_remaining FROM stock_batches WHERE id = ?1;",
            params![batch2_id],
            |row| row.get(0)
        ).unwrap();

        assert_eq!(batch1_rem, 0);
        assert_eq!(batch2_rem, 7);
    }

    #[test]
    fn test_similarity_matching() {
        assert_eq!(str_similarity("DOLIPRANE 1000MG", "doliprane 1000mg"), 1.0);
        assert!(str_similarity("DOLIPRANE 1G", "DOLIPRANE 1000MG") > 0.6);
        assert!(str_similarity("AMOXICILLINE 500MG", "AMOXI 500") > 0.45);
        assert_eq!(str_similarity("XYZ", "ABC"), 0.0);
    }

    #[test]
    fn test_imported_invoice_commit_and_alias() {
        let mut conn = setup_test_db();

        // 1. Insert existing drug
        conn.execute(
            "INSERT INTO drugs (barcode, name, price_per_item_da, cost_price_da, items_per_package)
             VALUES ('6130001', 'Paracetamol 500mg', 120.0, 90.0, 20);",
            [],
        ).unwrap();
        let drug_id = conn.last_insert_rowid();

        // 2. Commit invoice with:
        // - Item 1: existing drug, with new batch and an alias 'PARA 500MG TAB'
        // - Item 2: brand new drug 'Augmentin 1g'
        let commit_input = CommitInvoiceInput {
            supplier_name: "Biopharm".to_string(),
            invoice_number: "INV-2026-001".to_string(),
            invoice_date: "2026-09-18".to_string(),
            total_amount_da: 5000.0,
            pdf_path: "test.pdf".to_string(),
            items: vec![
                CommitInvoiceItemInput {
                    drug_id: Some(drug_id),
                    create_new_drug: false,
                    new_drug_name: None,
                    new_drug_barcode: None,
                    new_drug_items_per_package: None,
                    alias_to_save: Some("PARA 500MG TAB".to_string()),
                    batch_number: "LOT-P12".to_string(),
                    expiry_date: "2028-12-31".to_string(),
                    packages_received: 10,
                    cost_price_da: 95.0,
                    ppa_da: 130.0,
                    tva: Some(9.0),
                    mg: Some(20.0),
                },
                CommitInvoiceItemInput {
                    drug_id: None,
                    create_new_drug: true,
                    new_drug_name: Some("Augmentin 1g".to_string()),
                    new_drug_barcode: Some("6139999".to_string()),
                    new_drug_items_per_package: Some(12),
                    alias_to_save: Some("AUGM 1G SACHET".to_string()),
                    batch_number: "LOT-A99".to_string(),
                    expiry_date: "2027-06-30".to_string(),
                    packages_received: 5,
                    cost_price_da: 450.0,
                    ppa_da: 600.0,
                    tva: Some(9.0),
                    mg: Some(25.0),
                },
            ],
        };

        let tx = conn.transaction().unwrap();
        let invoice_id = run_commit_imported_invoice_transaction(&tx, commit_input).unwrap();
        tx.commit().unwrap();
        assert!(invoice_id > 0);

        // Check invoice recorded
        let inv_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM imported_invoices WHERE invoice_number = 'INV-2026-001';",
            [],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(inv_count, 1);

        // Check alias was recorded
        let alias_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM drug_aliases WHERE alias_name = 'PARA 500MG TAB' AND drug_id = ?1;",
            params![drug_id],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(alias_count, 1);

        // Check new drug created
        let new_drug_id: i64 = conn.query_row(
            "SELECT id FROM drugs WHERE name = 'Augmentin 1g';",
            [],
            |r| r.get(0),
        ).unwrap();
        assert!(new_drug_id > 0);

        // Check stock batch for new drug (5 packages * 12 items = 60 items)
        let batch_items: i32 = conn.query_row(
            "SELECT items_remaining FROM stock_batches WHERE drug_id = ?1 AND batch_number = 'LOT-A99';",
            params![new_drug_id],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(batch_items, 60);
    }

    #[test]
    fn test_patient_uniqueness_by_name_and_birth_date() {
        let conn = setup_test_db();

        // 1. Insert patient 1
        conn.execute(
            "INSERT INTO customers (name, birth_date) VALUES ('Amine Benali', '1990-05-12');",
            [],
        ).unwrap();

        // 2. Querying duplicate with same name and birth date
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM customers WHERE LOWER(TRIM(name)) = LOWER(TRIM(?1)) AND TRIM(birth_date) = TRIM(?2);",
            params!["amine benali ", "1990-05-12"],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(count, 1);

        // 3. Same name but different birth date should not collide
        let count_diff_birth: i64 = conn.query_row(
            "SELECT COUNT(*) FROM customers WHERE LOWER(TRIM(name)) = LOWER(TRIM(?1)) AND TRIM(birth_date) = TRIM(?2);",
            params!["Amine Benali", "1995-01-01"],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(count_diff_birth, 0);
    }

    #[test]
    fn test_eight_medicine_fields() {
        let conn = setup_test_db();

        // Insert drug with all 8 fields: designation, quantity, No Lot, PPA, PUHT, Exp, TVA, MG
        conn.execute(
            "INSERT INTO drugs (barcode, name, price_per_item_da, cost_price_da, items_per_package, tva, mg)
             VALUES ('6130099', 'DOLIPRANE 1000MG CPR', 210.0, 150.0, 1, 9.0, 28.4);",
            [],
        ).unwrap();
        let drug_id = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO stock_batches (drug_id, batch_number, expiry_date, packages_received, items_remaining)
             VALUES (?1, 'LOT-24A10', '2027-11-30', 25, 25);",
            params![drug_id],
        ).unwrap();

        // Query drug with stock batch and verify all 8 fields
        let (name, ppa, puht, tva, mg, qty, lot, exp): (String, f64, f64, f64, f64, i32, String, String) = conn.query_row(
            "SELECT d.name, d.price_per_item_da, d.cost_price_da, d.tva, d.mg, sb.items_remaining, sb.batch_number, sb.expiry_date
             FROM drugs d
             JOIN stock_batches sb ON d.id = sb.drug_id
             WHERE d.id = ?1;",
            params![drug_id],
            |row| Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
                row.get(7)?,
            )),
        ).unwrap();

        assert_eq!(name, "DOLIPRANE 1000MG CPR"); // designation
        assert_eq!(qty, 25);                      // quantity
        assert_eq!(lot, "LOT-24A10");             // No Lot
        assert_eq!(ppa, 210.0);                   // PPA
        assert_eq!(puht, 150.0);                  // PUHT
        assert_eq!(exp, "2027-11-30");            // Exp
        assert_eq!(tva, 9.0);                     // TVA
        assert_eq!(mg, 28.4);                     // MG
    }

    #[test]
    fn test_stock_separation_and_auto_barcode() {
        let mut conn = setup_test_db();

        // 1. Commit invoice with item having NO barcode -> must generate 6-digit barcode
        let commit_input = CommitInvoiceInput {
            supplier_name: "Grossiste Test".to_string(),
            invoice_number: "INV-6DIGIT".to_string(),
            invoice_date: "2026-09-20".to_string(),
            total_amount_da: 1000.0,
            pdf_path: "invoice.pdf".to_string(),
            items: vec![
                CommitInvoiceItemInput {
                    drug_id: None,
                    create_new_drug: true,
                    new_drug_name: Some("Spasfon 80mg".to_string()),
                    new_drug_barcode: None, // No barcode provided
                    new_drug_items_per_package: Some(10),
                    alias_to_save: None,
                    batch_number: "LOT-SPAS-01".to_string(),
                    expiry_date: "2027-01-01".to_string(),
                    packages_received: 20,
                    cost_price_da: 150.0,
                    ppa_da: 220.0,
                    tva: Some(9.0),
                    mg: Some(30.0),
                },
            ],
        };

        let tx = conn.transaction().unwrap();
        run_commit_imported_invoice_transaction(&tx, commit_input).unwrap();
        tx.commit().unwrap();

        // Verify drug was created and barcode is 6 digits
        let (drug_id, barcode): (i64, String) = conn.query_row(
            "SELECT id, barcode FROM drugs WHERE name = 'Spasfon 80mg';",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        ).unwrap();

        assert_eq!(barcode.len(), 6);
        assert!(barcode.chars().all(|c| c.is_ascii_digit()));

        // 2. Commit SECOND invoice for the SAME drug (even if create_new_drug: true or by same name)
        // Must add it as a new stock with its OWN price and lot, NOT failing UNIQUE constraint
        let commit_input_2 = CommitInvoiceInput {
            supplier_name: "Grossiste 2".to_string(),
            invoice_number: "INV-6DIGIT-2".to_string(),
            invoice_date: "2026-09-21".to_string(),
            total_amount_da: 2000.0,
            pdf_path: "invoice2.pdf".to_string(),
            items: vec![
                CommitInvoiceItemInput {
                    drug_id: None,
                    create_new_drug: true, // Attempt to create again
                    new_drug_name: Some("Spasfon 80mg".to_string()), // Exact same name
                    new_drug_barcode: None,
                    new_drug_items_per_package: Some(10),
                    alias_to_save: None,
                    batch_number: "LOT-SPAS-02".to_string(),
                    expiry_date: "2028-05-01".to_string(),
                    packages_received: 15,
                    cost_price_da: 160.0, // Different cost
                    ppa_da: 235.0,        // Different PPA
                    tva: Some(9.0),
                    mg: Some(32.0),
                },
            ],
        };

        let tx2 = conn.transaction().unwrap();
        // This must NOT fail with UNIQUE constraint failed: drugs.barcode!
        let inv2_result = run_commit_imported_invoice_transaction(&tx2, commit_input_2);
        assert!(inv2_result.is_ok(), "Second commit should succeed by attaching stock: {:?}", inv2_result.err());
        tx2.commit().unwrap();

        // 3. Verify stock separation: Same drug has 2 stock batches with their own PPA and PUHT
        let mut stmt = conn.prepare(
            "SELECT batch_number, expiry_date, items_remaining, price_per_item_da, cost_price_da 
             FROM stock_batches WHERE drug_id = ?1 ORDER BY expiry_date ASC;"
        ).unwrap();

        let batches: Vec<(String, String, i32, f64, f64)> = stmt.query_map(params![drug_id], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?))
        }).unwrap().map(|r| r.unwrap()).collect();

        assert_eq!(batches.len(), 2);
        assert_eq!(batches[0].0, "LOT-SPAS-01");
        assert_eq!(batches[0].2, 200); // 20 pkgs * 10
        assert_eq!(batches[0].3, 220.0);
        assert_eq!(batches[0].4, 150.0);

        assert_eq!(batches[1].0, "LOT-SPAS-02");
        assert_eq!(batches[1].2, 150); // 15 pkgs * 10
        assert_eq!(batches[1].3, 235.0);
        assert_eq!(batches[1].4, 160.0);
    }

    #[test]
    fn test_delete_drug_with_sales_history() {
        let mut conn = setup_test_db();

        // 1. Insert drug
        conn.execute(
            "INSERT INTO drugs (barcode, name, price_per_item_da, cost_price_da, items_per_package)
             VALUES ('999001', 'Aspirine 500mg', 100.0, 70.0, 10);",
            [],
        ).unwrap();
        let drug_id = conn.last_insert_rowid();

        // 2. Insert batch
        conn.execute(
            "INSERT INTO stock_batches (drug_id, batch_number, expiry_date, packages_received, items_remaining, price_per_item_da, cost_price_da)
             VALUES (?1, 'LOT-ASP1', '2028-01-01', 10, 100, 100.0, 70.0);",
            params![drug_id],
        ).unwrap();

        // 3. Checkout a sale referencing this drug
        let tx = conn.transaction().unwrap();
        let cart = vec![CartItemInput {
            drug_id,
            quantity_items: 5,
            unit_price_da: 100.0,
        }];
        let sale_id = run_checkout_transaction_logic(
            &tx,
            None,
            Some("Client Test".to_string()),
            None,
            None,
            None,
            cart,
            None,
            None,
        ).unwrap();
        tx.commit().unwrap();

        assert!(sale_id > 0);

        // 4. Delete the drug using the exact same logic as delete_drug command
        let drug_info: Option<(String, f64)> = conn.query_row(
            "SELECT name, cost_price_da FROM drugs WHERE id = ?1;",
            params![drug_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        ).ok();

        if let Some((name, cost)) = drug_info {
            let _ = conn.execute(
                "UPDATE sale_items SET 
                    drug_name = CASE WHEN drug_name IS NULL OR drug_name = '' THEN ?1 ELSE drug_name END,
                    cost_price_da = CASE WHEN cost_price_da = 0.0 THEN ?2 ELSE cost_price_da END
                 WHERE drug_id = ?3;",
                params![name, cost, drug_id],
            );
        }

        conn.execute("PRAGMA foreign_keys = OFF;", []).unwrap();
        let tx_del = conn.transaction().unwrap();
        tx_del.execute("UPDATE sale_items SET drug_id = NULL, batch_id = NULL WHERE drug_id = ?1;", params![drug_id]).unwrap();
        tx_del.execute("DELETE FROM stock_batches WHERE drug_id = ?1;", params![drug_id]).unwrap();
        tx_del.execute("DELETE FROM drug_aliases WHERE drug_id = ?1;", params![drug_id]).unwrap();
        tx_del.execute("DELETE FROM drugs WHERE id = ?1;", params![drug_id]).unwrap();
        tx_del.commit().unwrap();
        conn.execute("PRAGMA foreign_keys = ON;", []).unwrap();

        // 5. Verify drug is deleted from drugs table
        let drug_exists: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM drugs WHERE id = ?1);",
            params![drug_id],
            |r| r.get(0),
        ).unwrap();
        assert!(!drug_exists);

        // 6. Verify sale_items still retains historical record and name
        let (saved_name, qty, line_total): (String, i32, f64) = conn.query_row(
            "SELECT COALESCE(NULLIF(si.drug_name, ''), d.name, 'Article archivé'), si.quantity_items, si.line_total_da
             FROM sale_items si
             LEFT JOIN drugs d ON si.drug_id = d.id
             WHERE si.sale_id = ?1;",
            params![sale_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        ).unwrap();

        assert_eq!(saved_name, "Aspirine 500mg");
        assert_eq!(qty, 5);
        assert_eq!(line_total, 500.0);
    }

    #[test]
    fn test_loan_checkout_and_settlement() {
        let mut conn = setup_test_db();

        conn.execute(
            "INSERT INTO drugs (barcode, name, requires_prescription, price_per_item_da, cost_price_da, items_per_package) 
             VALUES ('555555', 'Paracetamol 500mg', 0, 150.0, 100.0, 10);",
            [],
        ).unwrap();
        let drug_id = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO stock_batches (drug_id, batch_number, expiry_date, packages_received, items_remaining, price_per_item_da, cost_price_da)
             VALUES (?1, 'LOT-P1', '2027-01-01', 5, 20, 150.0, 100.0);",
            params![drug_id],
        ).unwrap();

        conn.execute("INSERT INTO customers (name, birth_date) VALUES ('Ahmed Benali', '1980-05-15');", []).unwrap();
        let cust_id = conn.last_insert_rowid();

        // 1. Checkout with is_loan = Some(true)
        let tx = conn.transaction().unwrap();
        let cart = vec![CartItemInput {
            drug_id,
            quantity_items: 2,
            unit_price_da: 150.0,
        }];

        let sale_id = run_checkout_transaction_logic(
            &tx,
            Some(cust_id),
            Some("Ahmed Benali".to_string()),
            None,
            None,
            Some(30),
            cart,
            Some(true),
            None,
        ).unwrap();
        tx.commit().unwrap();

        // 2. Verify sale is marked as loan and active
        let (is_loan, status): (i32, String) = conn.query_row(
            "SELECT is_loan, loan_status FROM sales WHERE id = ?1;",
            params![sale_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        ).unwrap();

        assert_eq!(is_loan, 1);
        assert_eq!(status, "active");

        // Verify stock was deducted immediately (20 - 2 = 18)
        let remaining: i32 = conn.query_row(
            "SELECT items_remaining FROM stock_batches WHERE drug_id = ?1;",
            params![drug_id],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(remaining, 18);

        // 3. Settle the loan
        conn.execute(
            "UPDATE sales SET loan_status = 'settled', loan_settled_at = datetime('now', 'localtime') WHERE id = ?1 AND is_loan = 1;",
            params![sale_id],
        ).unwrap();

        let updated_status: String = conn.query_row(
            "SELECT loan_status FROM sales WHERE id = ?1;",
            params![sale_id],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(updated_status, "settled");
    }

    #[test]
    fn test_loan_deduction_from_sale() {
        let mut conn = setup_test_db();

        conn.execute(
            "INSERT INTO drugs (barcode, name, requires_prescription, price_per_item_da, cost_price_da, items_per_package) 
             VALUES ('666666', 'Amoxicilline 500mg', 0, 100.0, 70.0, 1);",
            [],
        ).unwrap();
        let drug_id = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO stock_batches (drug_id, batch_number, expiry_date, packages_received, items_remaining, price_per_item_da, cost_price_da)
             VALUES (?1, 'LOT-AMOX', '2027-01-01', 20, 20, 100.0, 70.0);",
            params![drug_id],
        ).unwrap();

        conn.execute("INSERT INTO customers (name, birth_date) VALUES ('Khaled Larbi', '1975-03-20');", []).unwrap();
        let cust_id = conn.last_insert_rowid();

        // 1. Customer previously took 1 box of Amoxicilline as an active loan
        let tx = conn.transaction().unwrap();
        let loan_cart = vec![CartItemInput {
            drug_id,
            quantity_items: 1,
            unit_price_da: 100.0,
        }];
        let loan_sale_id = run_checkout_transaction_logic(
            &tx,
            Some(cust_id),
            Some("Khaled Larbi".to_string()),
            None,
            None,
            Some(30),
            loan_cart,
            Some(true),
            None,
        ).unwrap();
        tx.commit().unwrap();

        // Stock after initial loan of 1: 20 - 1 = 19
        let stock_after_loan: i32 = conn.query_row(
            "SELECT items_remaining FROM stock_batches WHERE drug_id = ?1;",
            params![drug_id],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(stock_after_loan, 19);

        // 2. Today, customer wants 4 boxes of Amoxicilline with loan deduction of 1
        // Invoice charges for 4 boxes (400 DA), but stock deducts only 3 boxes!
        let tx2 = conn.transaction().unwrap();
        let current_cart = vec![CartItemInput {
            drug_id,
            quantity_items: 4,
            unit_price_da: 100.0,
        }];
        let loan_deductions = vec![LoanDeductionInput {
            loan_sale_id,
            drug_id,
            quantity: 1,
        }];

        let new_sale_id = run_checkout_transaction_logic(
            &tx2,
            Some(cust_id),
            Some("Khaled Larbi".to_string()),
            None,
            None,
            Some(30),
            current_cart,
            Some(false),
            Some(loan_deductions),
        ).unwrap();
        tx2.commit().unwrap();

        // 3. Verify stock: 19 - 3 = 16 (only 3 boxes physically deducted from stock!)
        let final_stock: i32 = conn.query_row(
            "SELECT items_remaining FROM stock_batches WHERE drug_id = ?1;",
            params![drug_id],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(final_stock, 16);

        // 4. Verify new sale is charged for 4 boxes (400.0 DA)
        let total_charged: f64 = conn.query_row(
            "SELECT total_da FROM sales WHERE id = ?1;",
            params![new_sale_id],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(total_charged, 400.0);

        // 5. Verify the old loan is now marked settled!
        let (loan_status, settled_at): (String, Option<String>) = conn.query_row(
            "SELECT loan_status, loan_settled_at FROM sales WHERE id = ?1;",
            params![loan_sale_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        ).unwrap();
        assert_eq!(loan_status, "settled");
        assert!(settled_at.is_some());
    }
}
