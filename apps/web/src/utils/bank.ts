import { SalesFailure } from "@open-faktura/api-client";
import type { AppKey } from "../locales";
export function bankError(e: unknown): AppKey {
  if (!(e instanceof SalesFailure)) return "bankFailed";
  if (e.status === 409) return "bankConflict";
  if (e.status !== 422) return "bankFailed";
  if (e.fields.includes("iban")) return "bankIbanInvalid";
  if (e.fields.includes("bank_rate")) return "bankForeignRateInvalid";
  if (e.fields.includes("currency")) return "bankCurrencyMismatch";
  if (e.fields.includes("currency_matching"))
    return "bankForeignAccountMatching";
  if (e.fields.some((f) => f.startsWith("split_"))) return "bankSplitInvalid";
  if (e.fields.includes("statement_account")) return "bankWrongAccount";
  if (e.fields.includes("statement_mixed_format")) return "bankMixedFormat";
  if (e.fields.some((f) => f.startsWith("statement_")))
    return "bankStatementInvalid";
  return "bankInvalid";
}
