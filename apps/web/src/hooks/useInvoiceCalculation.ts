import { useEffect, useState } from "react";
import {
  calculateInvoice,
  previewSettlement,
  InvoiceCalculationFailure,
  type DraftData,
  type InvoiceTotals,
} from "@open-faktura/api-client";
import { apiUrl } from "../services/status";
export function useInvoiceCalculation(
  company: string,
  input: DraftData["calculation"],
  onExpired: () => void,
  enabled = true,
  settlementId?: string,
  revision?: number,
) {
  const key = JSON.stringify([input, settlementId, revision]);
  const [result, setResult] = useState<{
    key: string;
    totals: InvoiceTotals | null;
    failed: boolean;
    fields: string[];
  } | null>(null);
  useEffect(() => {
    if (!enabled) return;
    const controller = new AbortController();
    const timer = setTimeout(() => {
      (settlementId
        ? previewSettlement(
            apiUrl,
            company,
            settlementId,
            JSON.parse(key)[0],
            controller.signal,
          )
        : calculateInvoice(
            apiUrl,
            company,
            JSON.parse(key)[0],
            controller.signal,
          )
      )
        .then((totals) => {
          if (!controller.signal.aborted)
            setResult({ key, totals, failed: false, fields: [] });
        })
        .catch((error) => {
          if (!controller.signal.aborted) {
            if (
              error instanceof InvoiceCalculationFailure &&
              error.status === 401
            )
              onExpired();
            setResult({
              key,
              totals: null,
              failed: true,
              fields:
                error instanceof InvoiceCalculationFailure ? error.fields : [],
            });
          }
        });
    }, 250);
    return () => {
      clearTimeout(timer);
      controller.abort();
    };
  }, [company, key, onExpired, enabled, settlementId]);
  return {
    loading: result?.key !== key,
    totals: result?.key === key ? result.totals : null,
    failed: result?.key === key && result.failed,
    fields: result?.key === key ? result.fields : [],
  };
}
