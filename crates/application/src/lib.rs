use faktura_domain::Availability;

pub fn readiness(database_available: bool) -> Availability {
    if database_available {
        Availability::Ready
    } else {
        Availability::Unavailable
    }
}

pub mod auth;
pub mod company;

pub mod ares;
pub mod contact;
pub mod draft;
pub mod invoice;

pub mod payment;

pub mod import;

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

pub mod received;

pub mod purchase_report;

pub mod banking;

pub mod cash;

pub mod bank_connection;

pub mod retail;

pub mod inbox;

pub mod stock;

pub mod preferences;

pub mod period_report;

pub mod stock_report;

pub mod dashboard;
pub mod item_sales;

pub mod tax_report;

pub mod exchange_rate;

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
