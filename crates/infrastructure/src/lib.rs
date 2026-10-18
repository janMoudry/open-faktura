use sqlx::{postgres::PgPoolOptions, PgPool};
use std::time::Duration;

pub fn pool(url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(2))
        .connect_lazy(url)
}

pub async fn migrate(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("../../migrations").run(pool).await
}

pub async fn database_available(pool: &PgPool) -> bool {
    sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(pool)
        .await
        .is_ok()
}

pub mod auth;
pub mod company;

pub mod ares;
pub mod contact;
pub mod draft;

pub mod invoice_pdf;

pub mod payment;

pub mod import;

pub mod import_table;

pub mod catalog;

pub mod sales;

pub mod delivery;

pub mod share;

pub mod invoice_delivery;

pub mod smtp;

pub mod attachment;

pub mod export;

pub mod advance;

pub mod tax_receipt;

pub mod advance_settlement;

pub mod credit_note;

pub mod recurring;

pub mod communication;

pub mod invoice_share;

pub mod received;

pub mod purchase_report;

pub mod banking;

mod bank_statement;

pub mod cash;

mod cash_automation;
mod cash_pdf;

pub mod bank_connection;

mod treasury_documents;

pub mod retail;
mod retail_pdf;

pub mod inbox;

mod inbox_isdoc;

pub mod stock;

pub mod preferences;

pub mod payment_qr;
pub mod period_report;

mod stock_documents;

pub mod stock_report;

pub mod item_sales;

pub mod dashboard;
pub mod money_catalog;

pub mod tax_report;

pub mod exchange_rate;

mod retail_posting_pdf;

pub mod company_admin;

pub mod isdoc;

pub mod vat_registry;

pub mod document_tags;

pub mod bank_notification;

pub mod document_output;

pub mod dashboard_widgets;

pub mod document_style;

pub mod draft_trash;

pub mod inbox_mailbox;

pub mod vies;

pub mod document_exports;

pub mod tax_classification;

pub mod taxpayer_filing_profile;

pub mod contact_anonymization;

pub mod inbox_notification;

pub mod inbox_delivery;

pub mod sales_trash;
