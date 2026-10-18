use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompanySettings {
    pub name: String,
    pub entity_type: String,
    pub registration_number: String,
    pub vat_number: String,
    pub street: String,
    pub city: String,
    pub postal_code: String,
    pub country: String,
    pub email: String,
    pub phone: String,
    pub tax_mode: String,
    pub tax_effective_from: String,
    pub currency: String,
    pub timezone: String,
    pub maturity_days: i32,
    pub payment_method: String,
    pub iban: String,
    pub number_prefix: String,
    pub number_year: i32,
    pub next_number: i32,
}
impl Default for CompanySettings {
    fn default() -> Self {
        Self {
            name: String::new(),
            entity_type: "individual".into(),
            registration_number: String::new(),
            vat_number: String::new(),
            street: String::new(),
            city: String::new(),
            postal_code: String::new(),
            country: "CZ".into(),
            email: String::new(),
            phone: String::new(),
            tax_mode: "non_payer".into(),
            tax_effective_from: String::new(),
            currency: "CZK".into(),
            timezone: "Europe/Prague".into(),
            maturity_days: 14,
            payment_method: "cash".into(),
            iban: String::new(),
            number_prefix: "FV".into(),
            number_year: 2026,
            next_number: 1,
        }
    }
}
impl CompanySettings {
    pub fn normalize(&mut self) {
        for value in [
            &mut self.name,
            &mut self.registration_number,
            &mut self.vat_number,
            &mut self.street,
            &mut self.city,
            &mut self.postal_code,
            &mut self.email,
            &mut self.phone,
            &mut self.number_prefix,
        ] {
            *value = value.trim().to_owned();
        }
        self.iban = self
            .iban
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>()
            .to_uppercase();
    }
    pub fn issues(&self, complete: bool) -> Vec<String> {
        let mut errors = Vec::new();
        let mut check = |field: &str, valid: bool| {
            if !valid {
                errors.push(field.to_owned());
            }
        };
        for (field, value, max) in [
            ("name", &self.name, 200),
            ("street", &self.street, 200),
            ("city", &self.city, 100),
            ("postal_code", &self.postal_code, 16),
            ("registration_number", &self.registration_number, 8),
            ("vat_number", &self.vat_number, 32),
            ("email", &self.email, 254),
            ("phone", &self.phone, 40),
        ] {
            check(
                field,
                value.len() <= max && !value.chars().any(char::is_control),
            );
        }
        check(
            "entity_type",
            ["individual", "business"].contains(&self.entity_type.as_str()),
        );
        check("country", self.country == "CZ");
        check("currency", self.currency == "CZK");
        check("timezone", self.timezone.parse::<chrono_tz::Tz>().is_ok());
        check(
            "tax_mode",
            ["non_payer", "vat_payer"].contains(&self.tax_mode.as_str()),
        );
        check(
            "tax_effective_from",
            (self.tax_effective_from.is_empty() && !complete)
                || (self.tax_effective_from.len() == 10
                    && chrono::NaiveDate::parse_from_str(&self.tax_effective_from, "%Y-%m-%d")
                        .is_ok()),
        );
        check(
            "registration_number",
            self.registration_number.is_empty()
                || (self.registration_number.len() == 8
                    && self.registration_number.bytes().all(|b| b.is_ascii_digit())),
        );
        check(
            "email",
            self.email.is_empty()
                || (self.email.split('@').count() == 2
                    && !self.email.starts_with('@')
                    && self.email.split('@').nth(1).is_some_and(|s| {
                        s.contains('.') && !s.starts_with('.') && !s.ends_with('.')
                    })
                    && !self.email.chars().any(char::is_whitespace)),
        );
        check("maturity_days", (0..=365).contains(&self.maturity_days));
        check(
            "payment_method",
            ["cash", "bank_transfer"].contains(&self.payment_method.as_str()),
        );
        check("iban", self.iban.is_empty() || valid_iban(&self.iban));
        check(
            "number_prefix",
            self.number_prefix.len() <= 12
                && self
                    .number_prefix
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-'),
        );
        check("number_year", (2000..=9999).contains(&self.number_year));
        check("next_number", (1..=1000000).contains(&self.next_number));
        if complete {
            check("name", !self.name.is_empty());
            check("street", !self.street.is_empty());
            check("city", !self.city.is_empty());
            check(
                "postal_code",
                self.postal_code
                    .chars()
                    .filter(|c| !c.is_whitespace())
                    .collect::<String>()
                    .len()
                    == 5
                    && self
                        .postal_code
                        .chars()
                        .all(|c| c.is_ascii_digit() || c == ' '),
            );
            check(
                "registration_number",
                self.entity_type != "business" || !self.registration_number.is_empty(),
            );
            check(
                "vat_number",
                self.tax_mode != "vat_payer"
                    || (self.vat_number.starts_with("CZ")
                        && (10..=12).contains(&self.vat_number.len())
                        && self.vat_number[2..].bytes().all(|b| b.is_ascii_digit())),
            );
            check(
                "iban",
                self.payment_method != "bank_transfer" || !self.iban.is_empty(),
            );
        }
        errors.sort();
        errors.dedup();
        errors
    }
    pub fn number_preview(&self) -> String {
        format!(
            "{}{}{:06}",
            self.number_prefix, self.number_year, self.next_number
        )
    }
}
pub fn valid_iban(value: &str) -> bool {
    value.parse::<iban::Iban>().is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn incomplete_and_invalid_profiles() {
        let mut s = CompanySettings::default();
        assert!(s.issues(false).is_empty());
        assert!(s.issues(true).contains(&"name".into()));
        s.iban = "CZ6508000000192000145399".into();
        assert!(s.issues(false).is_empty());
        s.iban = "de89 3704 0044 0532 0130 00".into();
        s.normalize();
        assert_eq!(s.iban, "DE89370400440532013000");
        assert!(s.issues(false).is_empty());
        s.iban = "CZ6508000000192000145390".into();
        assert!(s.issues(false).contains(&"iban".into()));
        s.tax_effective_from = "2026-02-30".into();
        assert!(s.issues(false).contains(&"tax_effective_from".into()));
    }
}
