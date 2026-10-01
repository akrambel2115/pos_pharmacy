mod db;
mod commands;

use db::{DbState, init_db};
use std::sync::Mutex;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            let conn = init_db(app.handle())
                .map_err(|e| Box::<dyn std::error::Error>::from(e))?;
            app.manage(DbState {
                conn: Mutex::new(conn),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::is_pin_configured,
            commands::configure_pin,
            commands::verify_pin,
            commands::get_settings,
            commands::generate_random_barcode,
            commands::get_drug_by_barcode,
            commands::get_stock_batches,
            commands::add_drug_with_batch,
            commands::search_customers,
            commands::add_customer,
            commands::delete_customer,
            commands::get_last_invoice,
            commands::get_sale_by_id,
            commands::checkout_sale,
            commands::update_settings,
            commands::get_sales_history,
            commands::backup_database,
            commands::restore_database,
            commands::get_drugs_stock_list,
            commands::search_drugs,
            commands::delete_drug,
            commands::get_notifications,
            commands::get_patients_list,
            commands::update_customer,
            commands::get_customer_sales,
            commands::pick_invoice_pdf_file,
            commands::scan_and_extract_invoice,
            commands::extract_invoice_with_gemini,
            commands::get_gemini_api_key,
            commands::save_gemini_api_key,
            commands::delete_gemini_api_key,
            commands::test_gemini_api_key,
            commands::validate_and_match_invoice,
            commands::ai_match_invoice_drugs,
            commands::commit_imported_invoice,
            commands::get_imported_invoices,
            commands::get_imported_invoice_by_id,
            commands::get_imported_invoice_drugs,
            commands::get_last_imported_invoice_drugs,
            commands::delete_imported_invoice,
            commands::open_invoice_file,
            commands::delete_stock_batch,
            commands::update_drug,
            commands::get_loans,
            commands::settle_loan,
            commands::get_patient_active_loans,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

