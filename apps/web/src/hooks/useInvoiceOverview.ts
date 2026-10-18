import { useEffect, useState } from "react";
import { apiUrl } from "../services/status";
import {
  getOverview,
  InvoiceCalculationFailure,
  type Overview,
} from "@open-faktura/api-client";
export function useInvoiceOverview(company: string, onExpired: () => void) {
  const [result, setResult] = useState<{
    company: string;
    value: Overview;
  } | null>(null);
  const [failed, setFailed] = useState(false);
  const [attempt, setAttempt] = useState(0);
  useEffect(() => {
    const controller = new AbortController();
    setFailed(false);
    getOverview(apiUrl, company, controller.signal)
      .then((value) => {
        if (!controller.signal.aborted) setResult({ company, value });
      })
      .catch((error) => {
        if (controller.signal.aborted) return;
        if (error instanceof InvoiceCalculationFailure && error.status === 401)
          onExpired();
        setFailed(true);
      });
    return () => controller.abort();
  }, [company, onExpired, attempt]);
  useEffect(() => {
    const refresh = () => setAttempt((v) => v + 1);
    window.addEventListener("focus", refresh);
    const timer = setInterval(refresh, 60000);
    return () => {
      window.removeEventListener("focus", refresh);
      clearInterval(timer);
    };
  }, []);
  return {
    value: result?.company === company && !failed ? result.value : null,
    failed,
    refresh: attempt,
    retry: () => setAttempt((v) => v + 1),
  };
}
