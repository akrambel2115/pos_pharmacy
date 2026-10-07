use rusqlite::{Connection, Result};

pub fn run_migrations(conn: &Connection) -> Result<()> {
    // Enable foreign keys
    conn.execute("PRAGMA foreign_keys = ON;", [])?;

    // Create Settings Table (strict 1-row constraint)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS settings (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            admin_pin_hash TEXT,
            expiry_warning_days INTEGER NOT NULL DEFAULT 30,
            default_language TEXT NOT NULL DEFAULT 'fr',
            last_backup_at TEXT,
            gemini_api_key TEXT NOT NULL DEFAULT ''
        );",
        [],
    )?;

    // Handle existing databases where gemini_api_key might be missing
    let _ = conn.execute("ALTER TABLE settings ADD COLUMN gemini_api_key TEXT NOT NULL DEFAULT '';", []);
    let _ = conn.execute("ALTER TABLE settings ADD COLUMN cashier_permissions TEXT NOT NULL DEFAULT '{}';", []);
    let _ = conn.execute("ALTER TABLE settings ADD COLUMN pharmacy_name TEXT NOT NULL DEFAULT '';", []);
    let _ = conn.execute("ALTER TABLE settings ADD COLUMN pharmacy_address TEXT NOT NULL DEFAULT '';", []);
    let _ = conn.execute("ALTER TABLE settings ADD COLUMN direct_ai_invoice INTEGER NOT NULL DEFAULT 0;", []);

    // Populate default settings row if missing
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM settings WHERE id = 1;",
        [],
        |row| row.get(0),
    )?;

    if count == 0 {
        conn.execute(
            "INSERT INTO settings (id, admin_pin_hash, expiry_warning_days, default_language, last_backup_at, gemini_api_key)
             VALUES (1, NULL, 30, 'fr', NULL, '');",
            [],
        )?;
    }

    // Create Drugs Table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS drugs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            barcode TEXT UNIQUE NOT NULL,
            name TEXT NOT NULL,
            requires_prescription INTEGER NOT NULL DEFAULT 0,
            price_per_item_da REAL NOT NULL,
            cost_price_da REAL NOT NULL,
            items_per_package INTEGER NOT NULL DEFAULT 1,
            tva REAL NOT NULL DEFAULT 9.0,
            mg REAL NOT NULL DEFAULT 0.0,
            created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime'))
        );",
        [],
    )?;

    // Handle existing databases where tva and mg columns might be missing
    let _ = conn.execute("ALTER TABLE drugs ADD COLUMN tva REAL NOT NULL DEFAULT 9.0;", []);
    let _ = conn.execute("ALTER TABLE drugs ADD COLUMN mg REAL NOT NULL DEFAULT 0.0;", []);

    // Create Stock Batches Table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS stock_batches (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            drug_id INTEGER NOT NULL,
            batch_number TEXT NOT NULL DEFAULT '',
            expiry_date TEXT NOT NULL, -- YYYY-MM-DD
            packages_received INTEGER NOT NULL DEFAULT 0,
            items_remaining INTEGER NOT NULL DEFAULT 0,
            price_per_item_da REAL NOT NULL DEFAULT 0.0,
            cost_price_da REAL NOT NULL DEFAULT 0.0,
            tva REAL NOT NULL DEFAULT 9.0,
            mg REAL NOT NULL DEFAULT 0.0,
            received_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
            FOREIGN KEY(drug_id) REFERENCES drugs(id) ON DELETE CASCADE
        );",
        [],
    )?;

    // Handle existing databases where pricing columns might be missing in stock_batches
    let _ = conn.execute("ALTER TABLE stock_batches ADD COLUMN price_per_item_da REAL NOT NULL DEFAULT 0.0;", []);
    let _ = conn.execute("ALTER TABLE stock_batches ADD COLUMN cost_price_da REAL NOT NULL DEFAULT 0.0;", []);
    let _ = conn.execute("ALTER TABLE stock_batches ADD COLUMN tva REAL NOT NULL DEFAULT 9.0;", []);
    let _ = conn.execute("ALTER TABLE stock_batches ADD COLUMN mg REAL NOT NULL DEFAULT 0.0;", []);
    let _ = conn.execute(
        "UPDATE stock_batches SET 
            price_per_item_da = COALESCE((SELECT price_per_item_da FROM drugs WHERE drugs.id = stock_batches.drug_id), 0.0),
            cost_price_da = COALESCE((SELECT cost_price_da FROM drugs WHERE drugs.id = stock_batches.drug_id), 0.0),
            tva = COALESCE((SELECT tva FROM drugs WHERE drugs.id = stock_batches.drug_id), 9.0),
            mg = COALESCE((SELECT mg FROM drugs WHERE drugs.id = stock_batches.drug_id), 0.0)
         WHERE price_per_item_da = 0.0;",
        [],
    );

    // Create Indexes for Expiry Date and Drug ID in stock_batches
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_stock_batches_expiry ON stock_batches(expiry_date);",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_stock_batches_drug_id ON stock_batches(drug_id);",
        [],
    )?;

    // Create Customers Table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS customers (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            birth_date TEXT NOT NULL DEFAULT '',
            id_card_number TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime'))
        );",
        [],
    )?;

    // Handle existing databases where these columns might be missing
    let _ = conn.execute("ALTER TABLE customers ADD COLUMN birth_date TEXT NOT NULL DEFAULT '';", []);
    let _ = conn.execute("ALTER TABLE customers ADD COLUMN id_card_number TEXT NOT NULL DEFAULT '';", []);
    let _ = conn.execute("CREATE UNIQUE INDEX IF NOT EXISTS idx_customers_name_birth ON customers(name, birth_date);", []);

    // Create Sales Table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS sales (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            customer_id INTEGER,
            prescribing_doctor_name TEXT,
            patient_name TEXT,
            total_da REAL NOT NULL,
            treatment_period_days INTEGER,
            created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
            FOREIGN KEY(customer_id) REFERENCES customers(id) ON DELETE SET NULL
        );",
        [],
    )?;

    // Handle existing databases where treatment_period_days might be missing
    let _ = conn.execute("ALTER TABLE sales ADD COLUMN treatment_period_days INTEGER;", []);
    let _ = conn.execute("ALTER TABLE sales ADD COLUMN is_loan INTEGER NOT NULL DEFAULT 0;", []);
    let _ = conn.execute("ALTER TABLE sales ADD COLUMN loan_status TEXT NOT NULL DEFAULT 'completed';", []);
    let _ = conn.execute("ALTER TABLE sales ADD COLUMN loan_settled_at TEXT;", []);
    let _ = conn.execute("CREATE INDEX IF NOT EXISTS idx_sales_is_loan ON sales(is_loan);", []);

    // Create Sale Items Table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS sale_items (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            sale_id INTEGER NOT NULL,
            drug_id INTEGER,
            batch_id INTEGER,
            drug_name TEXT NOT NULL DEFAULT '',
            quantity_items INTEGER NOT NULL,
            unit_price_da REAL NOT NULL,
            cost_price_da REAL NOT NULL DEFAULT 0.0,
            line_total_da REAL NOT NULL,
            FOREIGN KEY(sale_id) REFERENCES sales(id) ON DELETE CASCADE,
            FOREIGN KEY(drug_id) REFERENCES drugs(id) ON DELETE SET NULL,
            FOREIGN KEY(batch_id) REFERENCES stock_batches(id) ON DELETE SET NULL
        );",
        [],
    )?;

    // Handle existing databases where sale_items had rigid foreign keys or missing columns
    let has_drug_name: bool = conn.prepare("SELECT drug_name FROM sale_items LIMIT 1;").is_ok();
    if !has_drug_name {
        let _ = conn.execute("PRAGMA foreign_keys = OFF;", []);
        let _ = conn.execute(
            "CREATE TABLE IF NOT EXISTS sale_items_new (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                sale_id INTEGER NOT NULL,
                drug_id INTEGER,
                batch_id INTEGER,
                drug_name TEXT NOT NULL DEFAULT '',
                quantity_items INTEGER NOT NULL,
                unit_price_da REAL NOT NULL,
                cost_price_da REAL NOT NULL DEFAULT 0.0,
                line_total_da REAL NOT NULL,
                FOREIGN KEY(sale_id) REFERENCES sales(id) ON DELETE CASCADE,
                FOREIGN KEY(drug_id) REFERENCES drugs(id) ON DELETE SET NULL,
                FOREIGN KEY(batch_id) REFERENCES stock_batches(id) ON DELETE SET NULL
            );",
            [],
        );
        let _ = conn.execute(
            "INSERT INTO sale_items_new (id, sale_id, drug_id, batch_id, drug_name, quantity_items, unit_price_da, cost_price_da, line_total_da)
             SELECT 
                si.id, 
                si.sale_id, 
                si.drug_id, 
                si.batch_id, 
                COALESCE((SELECT name FROM drugs WHERE drugs.id = si.drug_id), ''), 
                si.quantity_items, 
                si.unit_price_da, 
                COALESCE((SELECT cost_price_da FROM drugs WHERE drugs.id = si.drug_id), 0.0), 
                si.line_total_da
             FROM sale_items si;",
            [],
        );
        let _ = conn.execute("DROP TABLE sale_items;", []);
        let _ = conn.execute("ALTER TABLE sale_items_new RENAME TO sale_items;", []);
        let _ = conn.execute("PRAGMA foreign_keys = ON;", []);
    }

    // Migration: Add batch_number to stock_batches if missing
    let _ = conn.execute("ALTER TABLE stock_batches ADD COLUMN batch_number TEXT NOT NULL DEFAULT '';", []);

    // Create Suppliers Table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS suppliers (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            phone TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime'))
        );",
        [],
    )?;

    // Create Imported Invoices Archive Table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS imported_invoices (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            supplier_name TEXT NOT NULL,
            invoice_number TEXT NOT NULL DEFAULT '',
            invoice_date TEXT NOT NULL DEFAULT '',
            total_amount_da REAL NOT NULL DEFAULT 0.0,
            pdf_path TEXT NOT NULL DEFAULT '',
            raw_json TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime'))
        );",
        [],
    )?;

    // Create Drug Aliases Table (Wholesaler Designation Mapping Memory)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS drug_aliases (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            drug_id INTEGER NOT NULL,
            alias_name TEXT NOT NULL UNIQUE,
            created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
            FOREIGN KEY(drug_id) REFERENCES drugs(id) ON DELETE CASCADE
        );",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_drug_aliases_name ON drug_aliases(alias_name);",
        [],
    )?;

    // Migration: Add invoice_id to stock_batches
    let _ = conn.execute("ALTER TABLE stock_batches ADD COLUMN invoice_id INTEGER REFERENCES imported_invoices(id) ON DELETE SET NULL;", []);
    let _ = conn.execute("CREATE INDEX IF NOT EXISTS idx_stock_batches_invoice_id ON stock_batches(invoice_id);", []);

    // Backfill invoice_id for existing stock_batches from imported_invoices
    if let Ok(mut stmt) = conn.prepare("SELECT id, raw_json FROM imported_invoices WHERE raw_json != '';") {
        if let Ok(rows) = stmt.query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))) {
            for row in rows.flatten() {
                let inv_id = row.0;
                let raw_json = row.1;
                #[derive(serde::Deserialize)]
                struct MinimalItem {
                    drug_id: i64,
                    #[serde(default)]
                    batch_number: String,
                }
                if let Ok(items) = serde_json::from_str::<Vec<MinimalItem>>(&raw_json) {
                    for it in items {
                        if !it.batch_number.trim().is_empty() {
                            let _ = conn.execute(
                                "UPDATE stock_batches SET invoice_id = ?1 WHERE drug_id = ?2 AND batch_number = ?3 AND invoice_id IS NULL;",
                                rusqlite::params![inv_id, it.drug_id, it.batch_number.trim()],
                            );
                        } else {
                            let _ = conn.execute(
                                "UPDATE stock_batches SET invoice_id = ?1 WHERE drug_id = ?2 AND invoice_id IS NULL;",
                                rusqlite::params![inv_id, it.drug_id],
                            );
                        }
                    }
                }
            }
        }
    }

    let _ = conn.execute(
        "UPDATE stock_batches 
         SET invoice_id = (
             SELECT inv.id FROM imported_invoices inv 
             WHERE date(inv.created_at) = date(stock_batches.received_at)
             ORDER BY inv.id DESC LIMIT 1
         )
         WHERE invoice_id IS NULL;",
        [],
    );

    Ok(())
}
