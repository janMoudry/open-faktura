//! Calendar recurrence anchored to the first date, never to the previous clamped date.
use chrono::{Datelike, Days, Months, NaiveDate};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Schedule {
    pub first_on: NaiveDate,
    pub frequency: String,
    pub interval: u32,
    pub month_end: bool,
    pub end_on: Option<NaiveDate>,
    pub count: Option<u32>,
    pub timezone: String,
}
impl Schedule {
    pub fn valid(&self) -> bool {
        (2000..=2100).contains(&self.first_on.year())
            && (1..=120).contains(&self.interval)
            && ["day", "week", "month", "year"].contains(&self.frequency.as_str())
            && (!self.month_end || self.frequency == "month")
            && self
                .end_on
                .is_none_or(|d| d >= self.first_on && d.year() <= 2100)
            && self.count.is_none_or(|n| (1..=10000).contains(&n))
            && self.timezone.parse::<chrono_tz::Tz>().is_ok()
    }
    pub fn date(&self, index: u32) -> Option<NaiveDate> {
        if !self.valid() || self.count.is_some_and(|n| index >= n) {
            return None;
        }
        let n = self.interval.checked_mul(index)?;
        let mut date = match self.frequency.as_str() {
            "day" => self.first_on.checked_add_days(Days::new(n.into())),
            "week" => self.first_on.checked_add_days(Days::new(u64::from(n) * 7)),
            "month" => self.first_on.checked_add_months(Months::new(n)),
            "year" => self
                .first_on
                .checked_add_months(Months::new(n.checked_mul(12)?)),
            _ => None,
        }?;
        if self.month_end {
            date = date
                .with_day(1)?
                .checked_add_months(Months::new(1))?
                .pred_opt()?;
        }
        (date.year() <= 2100 && self.end_on.is_none_or(|end| date <= end)).then_some(date)
    }
    pub fn preview(&self, index: u32) -> Vec<NaiveDate> {
        (index..index.saturating_add(12))
            .map_while(|i| self.date(i))
            .collect()
    }
    pub fn same_anchor(&self, other: &Self) -> bool {
        self.first_on == other.first_on
            && self.frequency == other.frequency
            && self.interval == other.interval
            && self.month_end == other.month_end
            && self.timezone == other.timezone
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn rule() -> Schedule {
        Schedule {
            first_on: NaiveDate::from_ymd_opt(2024, 1, 31).unwrap(),
            frequency: "month".into(),
            interval: 1,
            month_end: false,
            end_on: None,
            count: None,
            timezone: "Europe/Prague".into(),
        }
    }
    #[test]
    fn month_clamping_and_limits() {
        let mut s = rule();
        assert_eq!(s.preview(0)[1].to_string(), "2024-02-29");
        assert_eq!(s.preview(0)[2].to_string(), "2024-03-31");
        s.count = Some(2);
        assert_eq!(s.preview(0).len(), 2);
        s.count = None;
        s.end_on = Some(s.first_on);
        assert_eq!(s.preview(0).len(), 1);
        s.interval = 0;
        assert!(!s.valid());
    }
    #[test]
    fn year_leap_and_dst_dates() {
        let mut s = rule();
        s.first_on = NaiveDate::from_ymd_opt(2024, 2, 29).unwrap();
        s.frequency = "year".into();
        assert_eq!(s.date(1).unwrap().to_string(), "2025-02-28");
        assert_eq!(s.date(4).unwrap().to_string(), "2028-02-29");
        s.frequency = "day".into();
        s.first_on = NaiveDate::from_ymd_opt(2026, 3, 28).unwrap();
        assert_eq!(s.date(2).unwrap().to_string(), "2026-03-30");
        s.frequency = "month".into();
        s.month_end = true;
        assert_eq!(s.date(1).unwrap().to_string(), "2026-04-30");
    }
}
