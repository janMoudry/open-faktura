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
