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
