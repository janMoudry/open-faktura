use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Address {
    pub street: String,
    pub city: String,
    pub postal_code: String,
    pub country: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeliveryAddress {
    pub id: String,
    pub name: String,
    pub address: Address,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Person {
    pub name: String,
    pub email: String,
    pub phone: String,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContactData {
    pub name: String,
    pub registration_number: String,
    pub vat_number: String,
    pub billing_address: Address,
    pub delivery_addresses: Vec<DeliveryAddress>,
    pub default_delivery_id: Option<String>,
    pub email: String,
    pub phone: String,
    pub persons: Vec<Person>,
    pub recipients: Vec<String>,
    pub maturity_days: Option<i32>,
    pub payment_method: String,
    pub bank_account: String,
    pub iban: String,
    pub note: String,
}
fn text(value: &mut String, limit: usize, key: &str, errors: &mut Vec<String>) {
    *value = value.trim().to_owned();
    if value.chars().count() > limit
        || value
            .chars()
            .any(|c| c.is_control() && !(key == "note" && c == '\n'))
    {
        errors.push(key.into());
    }
}
fn address(value: &mut Address, key: &str, errors: &mut Vec<String>) {
    text(&mut value.street, 200, key, errors);
    text(&mut value.city, 120, key, errors);
    text(&mut value.postal_code, 30, key, errors);
    value.country = value.country.trim().to_uppercase();
    if value.country.len() != 2 || !value.country.bytes().all(|b| b.is_ascii_uppercase()) {
        errors.push(key.into());
    }
}
fn email(value: &mut String, key: &str, errors: &mut Vec<String>) {
    *value = value.trim().to_lowercase();
    if value.is_empty() {
        return;
    }
    let parts: Vec<_> = value.split('@').collect();
    if value.len() > 254
        || parts.len() != 2
        || parts[0].is_empty()
        || !parts[1].contains('.')
        || parts[1].starts_with('.')
        || parts[1].ends_with('.')
        || value.chars().any(|c| c.is_control() || c.is_whitespace())
    {
        errors.push(key.into());
    }
}
impl ContactData {
    pub fn normalize_validate(&mut self) -> Vec<String> {
        let mut errors = vec![];
        text(&mut self.name, 200, "name", &mut errors);
        if self.name.is_empty() {
            errors.push("name".into());
        }
        text(
            &mut self.registration_number,
            32,
            "registration_number",
            &mut errors,
        );
        text(&mut self.vat_number, 32, "vat_number", &mut errors);
        address(&mut self.billing_address, "billing_address", &mut errors);
        email(&mut self.email, "email", &mut errors);
        text(&mut self.phone, 50, "phone", &mut errors);
        text(&mut self.bank_account, 80, "bank_account", &mut errors);
        text(&mut self.iban, 34, "iban", &mut errors);
        text(&mut self.note, 2000, "note", &mut errors);
        if self.delivery_addresses.len() > 20 {
            errors.push("delivery_addresses".into());
        }
        let mut ids = std::collections::HashSet::new();
        for item in &mut self.delivery_addresses {
            text(&mut item.name, 120, "delivery_addresses", &mut errors);
            if item.id.is_empty() || item.id.len() > 64 || !ids.insert(item.id.clone()) {
                errors.push("delivery_addresses".into());
            }
            address(&mut item.address, "delivery_addresses", &mut errors);
        }
        if self
            .default_delivery_id
            .as_ref()
            .is_some_and(|id| !ids.contains(id))
        {
            errors.push("default_delivery_id".into());
        }
        if self.persons.len() > 20 {
            errors.push("persons".into());
        }
        for person in &mut self.persons {
            text(&mut person.name, 120, "persons", &mut errors);
            if person.name.is_empty() {
                errors.push("persons".into());
            }
            email(&mut person.email, "persons", &mut errors);
            text(&mut person.phone, 50, "persons", &mut errors);
        }
        if self.recipients.len() > 20 {
            errors.push("recipients".into());
        }
        for recipient in &mut self.recipients {
            email(recipient, "recipients", &mut errors);
            if recipient.is_empty() {
                errors.push("recipients".into());
            }
        }
        let mut recipients = std::collections::HashSet::new();
        self.recipients
            .retain(|value| recipients.insert(value.clone()));
        if self
            .maturity_days
            .is_some_and(|days| !(0..=365).contains(&days))
        {
            errors.push("maturity_days".into());
        }
        if !["", "cash", "bank_transfer"].contains(&self.payment_method.as_str()) {
            errors.push("payment_method".into());
        }
        errors.sort();
        errors.dedup();
        errors
    }
}
