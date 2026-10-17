//! Conservative moving-average valuation; no inferred acquisition prices.
use rust_decimal::{Decimal, RoundingStrategy};
#[derive(Debug)]
pub struct Balance {
    pub quantity: Decimal,
    pub value: Option<Decimal>,
}
impl Default for Balance {
    fn default() -> Self {
        Self {
            quantity: Decimal::ZERO,
            value: Some(Decimal::ZERO),
        }
    }
}
impl Balance {
    pub fn unit_cost(&self) -> Option<Decimal> {
        if self.quantity <= Decimal::ZERO {
            return None;
        }
        self.value.map(|v| {
            (v / self.quantity).round_dp_with_strategy(4, RoundingStrategy::MidpointAwayFromZero)
        })
    }
}
/// Allocate whole cents by largest remainder; stable ties follow input order.
pub fn allocate(total: Decimal, weights: &[Decimal]) -> Option<Vec<Decimal>> {
    if total < Decimal::ZERO || total.scale() > 2 || weights.iter().any(|w| *w < Decimal::ZERO) {
        return None;
    }
    if total.is_zero() {
        return Some(vec![Decimal::ZERO; weights.len()]);
    }
    let sum: Decimal = weights.iter().sum();
    if sum <= Decimal::ZERO {
        return None;
    }
    let cents = total * Decimal::from(100);
    let exact: Vec<Decimal> = weights.iter().map(|w| cents * *w / sum).collect();
    let mut parts: Vec<Decimal> = exact.iter().map(|v| v.floor()).collect();
    let mut rest = cents - parts.iter().sum::<Decimal>();
    let mut order: Vec<usize> = (0..weights.len()).collect();
    order.sort_by(|a, b| {
        (exact[*b] - parts[*b])
            .cmp(&(exact[*a] - parts[*a]))
            .then(a.cmp(b))
    });
    for i in order {
        if rest <= Decimal::ZERO {
            break;
        }
        parts[i] += Decimal::ONE;
        rest -= Decimal::ONE;
    }
    if !rest.is_zero() {
        return None;
    }
    Some(parts.into_iter().map(|v| v / Decimal::from(100)).collect())
}
#[derive(Clone)]
pub enum Flow {
    Receipt {
        source: String,
        price: Option<Decimal>,
        total_quantity: Decimal,
        extra: Decimal,
    },
    Issue {
        source: String,
    },
    Return {
        source: String,
    },
    Reverse {
        movement: i64,
    },
    Unknown,
}
pub struct Event {
    pub id: i64,
    pub quantity: Decimal,
    pub flow: Flow,
}
pub struct Valued {
    pub balance: Balance,
    pub issued: Option<Decimal>,
    pub correction_stock: Option<Decimal>,
    pub correction_issued: Option<Decimal>,
}
fn rounded(v: Decimal) -> Decimal {
    v.round_dp_with_strategy(12, RoundingStrategy::MidpointAwayFromZero)
}
fn run(
    events: &[Event],
    prices: &std::collections::BTreeMap<String, Option<Decimal>>,
) -> (Balance, Option<Decimal>) {
    use std::collections::BTreeMap;
    let mut balance = Balance::default();
    let mut issued = Some(Decimal::ZERO);
    let mut outstanding = BTreeMap::<String, (Decimal, Option<Decimal>)>::new();
    let mut movement_prices = BTreeMap::<i64, Option<Decimal>>::new();
    let mut movement_values = BTreeMap::<i64, Option<Decimal>>::new();
    for e in events {
        if e.quantity.is_zero() {
            continue;
        }
        let average = if balance.quantity > Decimal::ZERO {
            balance.value.map(|v| rounded(v / balance.quantity))
        } else {
            None
        };
        let (price, is_issue, source) = match &e.flow {
            Flow::Receipt { source, .. } => (prices.get(source).copied().flatten(), false, None),
            Flow::Issue { source } => (average, true, Some(source.clone())),
            Flow::Return { source } => {
                let price = outstanding.get(source).and_then(|(q, v)| {
                    if *q >= e.quantity && *q > Decimal::ZERO {
                        v.map(|v| rounded(v / *q))
                    } else {
                        None
                    }
                });
                (price, true, Some(source.clone()))
            }
            Flow::Reverse { movement } => {
                (movement_prices.get(movement).copied().flatten(), true, None)
            }
            Flow::Unknown => (
                if e.quantity < Decimal::ZERO {
                    average
                } else {
                    None
                },
                true,
                None,
            ),
        };
        // Outbound at average consumes the exact fraction, including the final remainder.
        let delta = if matches!(&e.flow, Flow::Issue { .. } | Flow::Unknown)
            && e.quantity < Decimal::ZERO
            && balance.quantity > Decimal::ZERO
        {
            balance
                .value
                .map(|v| rounded(v * e.quantity / balance.quantity))
        } else {
            price.map(|p| rounded(p * e.quantity))
        };
        let delta = match &e.flow {
            Flow::Reverse { movement } => {
                movement_values.get(movement).copied().flatten().map(|v| -v)
            }
            Flow::Return { source } => outstanding.get(source).and_then(|(q, v)| {
                if *q >= e.quantity && *q > Decimal::ZERO {
                    v.map(|v| rounded(v * e.quantity / *q))
                } else {
                    None
                }
            }),
            _ => delta,
        };
        movement_values.insert(e.id, delta);
        movement_prices.insert(e.id, delta.map(|v| rounded(v / e.quantity)));
        if is_issue {
            issued = issued.zip(delta).map(|(v, d)| v - d);
            if let Some(source) = source {
                let entry = outstanding
                    .entry(source)
                    .or_insert((Decimal::ZERO, Some(Decimal::ZERO)));
                entry.0 -= e.quantity;
                entry.1 = entry.1.zip(delta).map(|(v, d)| v - d);
                if entry.0.is_zero() {
                    entry.1 = Some(Decimal::ZERO);
                }
            }
        }
        let next = balance.quantity + e.quantity;
        let value = balance.value.zip(delta).map(|(v, d)| v + d);
        if value
            .is_some_and(|v| v < Decimal::ZERO || (next.is_zero() && v.abs() > Decimal::new(1, 10)))
        {
            issued = None;
        }
        balance.value = if next.is_zero() {
            Some(Decimal::ZERO)
        } else if next < Decimal::ZERO || balance.quantity < Decimal::ZERO {
            None
        } else {
            value.filter(|v| *v >= Decimal::ZERO)
        };
        balance.quantity = next;
    }
    (balance, issued)
}
/// Restate the cost basis using immutable receipt revisions available at the report date.
pub fn value(events: &[Event]) -> Valued {
    // Removing then re-adding a line is a new acquisition cycle, not a price
    // correction to receipts that have already been cancelled.
    let mut cycles = std::collections::BTreeMap::<String, (Decimal, u64)>::new();
    let normalized: Vec<Event> = events
        .iter()
        .map(|e| {
            let mut flow = e.flow.clone();
            if let Flow::Receipt {
                source,
                total_quantity,
                ..
            } = &mut flow
            {
                let cycle = cycles.entry(source.clone()).or_insert((Decimal::ZERO, 0));
                if cycle.0.is_zero() && e.quantity > Decimal::ZERO {
                    cycle.1 += 1;
                }
                cycle.0 = *total_quantity;
                *source = format!("{source}/{}", cycle.1);
            }
            Event {
                id: e.id,
                quantity: e.quantity,
                flow,
            }
        })
        .collect();
    let events = normalized.as_slice();
    let mut first = std::collections::BTreeMap::new();
    let mut latest = std::collections::BTreeMap::new();
    for e in events {
        if let Flow::Receipt {
            source,
            price,
            total_quantity,
            extra,
        } = &e.flow
        {
            if *total_quantity > Decimal::ZERO {
                let unit = price.map(|p| p + rounded(*extra / *total_quantity));
                first.entry(source.clone()).or_insert(unit);
                latest.insert(source.clone(), unit);
            }
        }
    }
    let (baseline, base_issued) = run(events, &first);
    let (balance, issued) = run(events, &latest);
    Valued {
        correction_stock: balance.value.zip(baseline.value).map(|(a, b)| a - b),
        correction_issued: issued.zip(base_issued).map(|(a, b)| a - b),
        balance,
        issued,
    }
}
#[cfg(test)]
mod allocation_tests {
    use super::*;
    fn d(v: i64) -> Decimal {
        Decimal::from(v)
    }
    fn receipt(id: i64, q: i64, price: i64) -> Event {
        Event {
            id,
            quantity: d(q),
            flow: Flow::Receipt {
                source: "p1".into(),
                price: Some(d(price)),
                total_quantity: d(10),
                extra: Decimal::ZERO,
            },
        }
    }
    #[test]
    fn corrections_and_returns_split_remaining_and_issued_cost() {
        let mut e = vec![
            receipt(1, 10, 20),
            Event {
                id: 2,
                quantity: d(-4),
                flow: Flow::Issue {
                    source: "i1".into(),
                },
            },
            receipt(3, 0, 30),
        ];
        let v = value(&e);
        assert_eq!(v.balance.value, Some(d(180)));
        assert_eq!(v.issued, Some(d(120)));
        assert_eq!(v.correction_stock, Some(d(60)));
        assert_eq!(v.correction_issued, Some(d(40)));
        e.push(Event {
            id: 4,
            quantity: d(2),
            flow: Flow::Return {
                source: "i1".into(),
            },
        });
        let v = value(&e);
        assert_eq!(v.balance.value, Some(d(240)));
        assert_eq!(v.issued, Some(d(60)));
    }
    #[test]
    fn incidental_cost_exact_cents_and_zero_weights() {
        assert_eq!(
            allocate(Decimal::new(5, 2), &[d(1), d(1), d(1)]),
            Some(vec![
                Decimal::new(2, 2),
                Decimal::new(2, 2),
                Decimal::new(1, 2)
            ])
        );
        assert!(allocate(d(1), &[d(0)]).is_none());
        let mut e = vec![receipt(1, 10, 20)];
        if let Flow::Receipt { extra, .. } = &mut e[0].flow {
            *extra = d(50);
        }
        assert_eq!(value(&e).balance.value, Some(d(250)));
    }
    #[test]
    fn supplier_return_and_manual_reversal() {
        let e = vec![
            receipt(1, 10, 20),
            Event {
                id: 2,
                quantity: d(-3),
                flow: Flow::Issue {
                    source: "m2".into(),
                },
            },
            Event {
                id: 3,
                quantity: d(3),
                flow: Flow::Reverse { movement: 2 },
            },
            receipt(4, -2, 20),
        ];
        let v = value(&e);
        assert_eq!(v.balance.value, Some(d(160)));
        assert_eq!(v.issued, Some(d(0)));
    }
}

#[cfg(test)]
mod return_tests {
    use super::*;
    fn d(n: i64) -> Decimal {
        Decimal::from(n)
    }
    fn receipt(id: i64, key: &str, q: i64, price: Option<i64>, total: i64) -> Event {
        Event {
            id,
            quantity: d(q),
            flow: Flow::Receipt {
                source: key.into(),
                price: price.map(d),
                total_quantity: d(total),
                extra: Decimal::ZERO,
            },
        }
    }
    #[test]
    fn retail_returns_weighted_original_issues() {
        let e = vec![
            receipt(1, "a", 10, Some(20), 10),
            Event {
                id: 2,
                quantity: d(-2),
                flow: Flow::Issue {
                    source: "sale".into(),
                },
            },
            receipt(3, "b", 8, Some(40), 8),
            Event {
                id: 4,
                quantity: d(-2),
                flow: Flow::Issue {
                    source: "sale".into(),
                },
            },
            Event {
                id: 5,
                quantity: d(3),
                flow: Flow::Return {
                    source: "sale".into(),
                },
            },
        ];
        let v = value(&e);
        assert_eq!(v.balance.value, Some(d(495)));
        assert_eq!(v.issued, Some(d(25)));
    }
    #[test]
    fn removing_and_readding_source_does_not_reprice_old_cycle() {
        let e = vec![
            receipt(1, "a", 10, Some(20), 10),
            receipt(2, "a", -10, None, 0),
            receipt(3, "a", 5, Some(30), 5),
        ];
        let v = value(&e);
        assert_eq!(v.balance.value, Some(d(150)));
        assert_eq!(v.correction_stock, Some(d(0)));
    }
    #[test]
    fn unknown_returns_and_negative_inventory_remain_unknown() {
        let e = vec![
            receipt(1, "a", 3, None, 3),
            Event {
                id: 2,
                quantity: d(-4),
                flow: Flow::Issue {
                    source: "sale".into(),
                },
            },
            Event {
                id: 3,
                quantity: d(2),
                flow: Flow::Return {
                    source: "sale".into(),
                },
            },
        ];
        assert_eq!(value(&e).balance.value, None);
        assert_eq!(
            value(&[Event {
                id: 1,
                quantity: d(1),
                flow: Flow::Return {
                    source: "missing".into()
                }
            }])
            .balance
            .value,
            None
        );
    }
    #[test]
    fn full_return_preserves_fractional_cost_exactly() {
        let e = vec![
            receipt(1, "a", 3, Some(1), 3),
            receipt(2, "b", 4, Some(2), 4),
            Event {
                id: 3,
                quantity: d(-3),
                flow: Flow::Issue {
                    source: "sale".into(),
                },
            },
            Event {
                id: 4,
                quantity: d(1),
                flow: Flow::Return {
                    source: "sale".into(),
                },
            },
            Event {
                id: 5,
                quantity: d(2),
                flow: Flow::Return {
                    source: "sale".into(),
                },
            },
        ];
        let v = value(&e);
        assert_eq!(v.balance.value, Some(d(11)));
        assert_eq!(v.issued, Some(d(0)));
    }
}
