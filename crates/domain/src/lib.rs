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
