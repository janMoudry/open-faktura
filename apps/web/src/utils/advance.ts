import type { SalesDocument } from "@open-faktura/api-client";
import type { AppKey } from "../locales";
export function advanceStatus(d: SalesDocument): AppKey {
  const cents = (v: string) => {
    const [a, b = ""] = v.split(".");
    return BigInt(a || "0") * 100n + BigInt(b.padEnd(2, "0"));
  };
  const paid = cents(d.advance_paid ?? "0"),
    total = cents(d.totals?.payable ?? "0");
  return paid > total
    ? "paymentOverpaid"
    : paid === total && total > 0n
      ? "paymentPaid"
      : paid > 0n
        ? "paymentPartial"
        : "paymentOutstanding";
}
