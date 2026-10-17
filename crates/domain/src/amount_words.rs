//! Czech and English CZK wording. Decimal strings only; never floating point.
use crate::received::amount;
use rust_decimal::prelude::ToPrimitive;

fn form(n: u64, one: &'static str, few: &'static str, many: &'static str) -> &'static str {
    match if (11..=14).contains(&(n % 100)) {
        0
    } else {
        n % 10
    } {
        1 => one,
        2..=4 => few,
        _ => many,
    }
}
fn group_cs(n: u64, feminine: bool) -> String {
    let ones = [
        "",
        if feminine { "jedna" } else { "jeden" },
        if feminine { "dvě" } else { "dva" },
        "tři",
        "čtyři",
        "pět",
        "šest",
        "sedm",
        "osm",
        "devět",
    ];
    let teens = [
        "deset",
        "jedenáct",
        "dvanáct",
        "třináct",
        "čtrnáct",
        "patnáct",
        "šestnáct",
        "sedmnáct",
        "osmnáct",
        "devatenáct",
    ];
    let tens = [
        "",
        "",
        "dvacet",
        "třicet",
        "čtyřicet",
        "padesát",
        "šedesát",
        "sedmdesát",
        "osmdesát",
        "devadesát",
    ];
    let hundreds = [
        "",
        "sto",
        "dvě stě",
        "tři sta",
        "čtyři sta",
        "pět set",
        "šest set",
        "sedm set",
        "osm set",
        "devět set",
    ];
    let mut parts = Vec::new();
    if n >= 100 {
        parts.push(hundreds[(n / 100) as usize]);
    }
    let n = n % 100;
    if (10..20).contains(&n) {
        parts.push(teens[(n - 10) as usize]);
    } else {
        if n >= 20 {
            parts.push(tens[(n / 10) as usize]);
        }
        if !n.is_multiple_of(10) {
            parts.push(ones[(n % 10) as usize]);
        }
    }
    parts.join(" ")
}
fn group_en(n: u64) -> String {
    let ones = [
        "",
        "one",
        "two",
        "three",
        "four",
        "five",
        "six",
        "seven",
        "eight",
        "nine",
        "ten",
        "eleven",
        "twelve",
        "thirteen",
        "fourteen",
        "fifteen",
        "sixteen",
        "seventeen",
        "eighteen",
        "nineteen",
    ];
    let tens = [
        "", "", "twenty", "thirty", "forty", "fifty", "sixty", "seventy", "eighty", "ninety",
    ];
    let mut parts = Vec::new();
    if n >= 100 {
        parts.push(format!("{} hundred", ones[(n / 100) as usize]));
    }
    let n = n % 100;
    if n > 0 {
        if n < 20 {
            parts.push(ones[n as usize].into());
        } else if n.is_multiple_of(10) {
            parts.push(tens[(n / 10) as usize].into());
        } else {
            parts.push(format!(
                "{}-{}",
                tens[(n / 10) as usize],
                ones[(n % 10) as usize]
            ));
        }
    }
    parts.join(" ")
}
fn integer(mut n: u64, en: bool, feminine: bool) -> String {
    if n == 0 {
        return if en { "zero" } else { "nula" }.into();
    }
    let mut parts = vec![];
    for (scale, eng, one, few, many, female) in [
        (
            1_000_000_000,
            "billion",
            "miliarda",
            "miliardy",
            "miliard",
            true,
        ),
        (1_000_000, "million", "milion", "miliony", "milionů", false),
        (1000, "thousand", "tisíc", "tisíce", "tisíc", false),
    ] {
        let group = n / scale;
        n %= scale;
        if group > 0 {
            parts.push(if en {
                format!("{} {eng}", group_en(group))
            } else {
                format!(
                    "{} {}",
                    group_cs(group, female),
                    form(group, one, few, many)
                )
            });
        }
    }
    if n > 0 {
        parts.push(if en {
            group_en(n)
        } else {
            group_cs(n, feminine)
        });
    }
    parts.join(" ")
}
pub fn czk(value: &str, language: &str) -> Option<String> {
    if !["cs", "en"].contains(&language) {
        return None;
    }
    let amount = amount(value)?;
    let cents = (amount.abs() * rust_decimal::Decimal::from(100)).to_u64()?;
    let whole = cents / 100;
    let fractional = cents % 100;
    let en = language == "en";
    let sign = if amount.is_sign_negative() && !amount.is_zero() {
        "minus "
    } else {
        ""
    };
    Some(if en {
        format!(
            "{sign}{} {} and {} {}",
            integer(whole, true, false),
            if whole == 1 {
                "Czech crown"
            } else {
                "Czech crowns"
            },
            integer(fractional, true, false),
            if fractional == 1 { "heller" } else { "hellers" }
        )
    } else {
        format!(
            "{sign}{} {} a {} {}",
            integer(whole, false, true),
            form(whole, "koruna česká", "koruny české", "korun českých"),
            integer(fractional, false, false),
            form(fractional, "haléř", "haléře", "haléřů")
        )
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn czk_words_exact() {
        for (value, cs, en) in [
            (
                "0",
                "nula korun českých a nula haléřů",
                "zero Czech crowns and zero hellers",
            ),
            (
                "1.01",
                "jedna koruna česká a jeden haléř",
                "one Czech crown and one heller",
            ),
            (
                "2.02",
                "dvě koruny české a dva haléře",
                "two Czech crowns and two hellers",
            ),
            (
                "21.12",
                "dvacet jedna koruna česká a dvanáct haléřů",
                "twenty-one Czech crowns and twelve hellers",
            ),
            (
                "-121.05",
                "minus sto dvacet jedna koruna česká a pět haléřů",
                "minus one hundred twenty-one Czech crowns and five hellers",
            ),
            (
                "1002003004.00",
                "jedna miliarda dva miliony tři tisíce čtyři koruny české a nula haléřů",
                "one billion two million three thousand four Czech crowns and zero hellers",
            ),
        ] {
            assert_eq!(czk(value, "cs").as_deref(), Some(cs));
            assert_eq!(czk(value, "en").as_deref(), Some(en));
        }
        assert!(czk("999999999999.99", "cs").unwrap().contains("miliard"));
        for value in ["1.001", "1e3", "NaN", "1000000000000"] {
            assert!(czk(value, "cs").is_none());
        }
        assert!(czk("1", "de").is_none());
    }
}
