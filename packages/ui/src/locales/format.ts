import { locales, type Language } from "./index";
/** Formatting only: accounting arithmetic belongs to the domain. */
export function formatNumber(
  value: number | bigint,
  language: Language,
  options?: Intl.NumberFormatOptions,
) {
  return new Intl.NumberFormat(locales[language], options).format(value);
}
/** Keep an exact decimal string; never round-trip money through Number. */
export function formatMoney(
  value: string,
  currency: string,
  language: Language,
) {
  if (!/^-?\d+(\.\d+)?$/.test(value))
    throw new RangeError("Expected decimal money");
  const formatter = new Intl.NumberFormat(locales[language], {
    style: "currency",
    currency,
  });
  // Modern Intl accepts decimal strings losslessly; TS's ES2022 lib omits that overload.
  return (formatter.format as unknown as (value: string) => string)(value);
}
/** ISO calendar dates have no timezone. UTC is only used as a formatting carrier. */
export function formatCalendarDate(value: string, language: Language) {
  if (!/^\d{4}-\d{2}-\d{2}$/.test(value))
    throw new RangeError("Expected ISO calendar date");
  const date = new Date(`${value}T00:00:00Z`);
  if (
    !Number.isFinite(date.getTime()) ||
    date.toISOString().slice(0, 10) !== value
  )
    throw new RangeError("Invalid calendar date");
  return new Intl.DateTimeFormat(locales[language], {
    timeZone: "UTC",
    day: "numeric",
    month: "numeric",
    year: "numeric",
  }).format(date);
}
