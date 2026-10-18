/// Availability is infrastructure state, separate from accounting document states.
#[derive(Debug, PartialEq, Eq)]
pub enum Availability {
    Ready,
    Unavailable,
}
pub mod company;

pub mod contact;
pub mod invoice;

pub mod payment;

pub mod import;

pub mod catalog;

pub mod tax_receipt;

pub mod advance_settlement;

pub mod credit_note;

pub mod recurring;

pub mod received;

pub mod banking;

pub mod cash;

pub mod amount_words;

pub mod payment_qr;
pub mod retail;

pub mod stock_cost;

pub mod exchange;
