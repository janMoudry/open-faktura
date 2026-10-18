import { useEffect, useState } from "react";
import { Alert, Button, Dialog, Field, Heading, Input } from "@open-faktura/ui";
import {
  bankSplitCandidates,
  type BankCandidate,
  type BankMovement,
} from "@open-faktura/api-client";
import { useAppTranslation } from "../../locales";
import { useBankCommand } from "../../hooks/useBankCommand";
import { useWorkspace } from "../../hooks/useCompanyWorkspace";
import { apiUrl } from "../../services/status";
import { moneyText } from "../../utils/invoice";
// UI projection only; the backend revalidates amounts and balances in one transaction.
function cents(value: string): bigint | null {
  const v = value.trim().replace(",", ".");
  if (!/^\d{1,12}(\.\d{1,2})?$/.test(v)) return null;
  const [whole, fraction = ""] = v.split(".");
  return BigInt(whole) * 100n + BigInt(fraction.padEnd(2, "0"));
}
function decimal(value: bigint): string {
  const sign = value < 0n ? "-" : "";
  const n = value < 0n ? -value : value;
  return `${sign}${n / 100n}.${String(n % 100n).padStart(2, "0")}`;
}
export function BankSplitDialog({
  companyId,
  movement,
  onClose,
  onSaved,
}: {
  companyId: string;
  movement: BankMovement;
  onClose: () => void;
  onSaved: (m: BankMovement) => void;
}) {
  const { t, language } = useAppTranslation(),
    cmd = useBankCommand(companyId),
    { setPageDirty } = useWorkspace();
  const [rows, setRows] = useState<
    { candidate: BankCandidate; amount: string }[]
  >([]);
  const [input, setInput] = useState(""),
    [query, setQuery] = useState(""),
    [attempt, setAttempt] = useState(0);
  const [choices, setChoices] = useState<BankCandidate[] | null>(null),
    [failed, setFailed] = useState(false);
  useEffect(() => {
    setPageDirty(rows.length > 0);
    return () => setPageDirty(false);
  }, [rows.length, setPageDirty]);
  useEffect(() => {
    const controller = new AbortController();
    setChoices(null);
    setFailed(false);
    bankSplitCandidates(
      apiUrl,
      companyId,
      movement.id,
      query,
      controller.signal,
    )
      .then((v) => {
        if (!controller.signal.aborted) setChoices(v);
      })
      .catch(() => {
        if (!controller.signal.aborted) setFailed(true);
      });
    return () => controller.abort();
  }, [companyId, movement.id, query, attempt]);
  const total = rows.reduce((sum, r) => sum + (cents(r.amount) ?? 0n), 0n),
    remaining = (cents(movement.remaining) ?? 0n) - total;
  const valid =
    rows.length > 0 &&
    remaining >= 0n &&
    rows.every((r) => {
      const n = cents(r.amount);
      return (
        n !== null &&
        n > 0n &&
        n <= (cents(r.candidate.available) ?? 0n) &&
        (!r.candidate.payment_id || n === cents(r.candidate.available))
      );
    });
  return (
    <Dialog
      open
      title={t("bankSplit")}
      description={t("bankSplitHelp")}
      onClose={cmd.busy ? () => {} : onClose}
    >
      <div className="space-y-field">
        <p>
          {t("bankUnallocated")}:{" "}
          <strong>
            {moneyText(movement.remaining, language, movement.data.currency)}
          </strong>
        </p>
        {cmd.error && <Alert tone="danger">{t(cmd.error)}</Alert>}
        <form
          className="flex gap-control"
          onSubmit={(e) => {
            e.preventDefault();
            setQuery(input);
          }}
        >
          <Input
            aria-label={t("bankCandidateSearch")}
            value={input}
            onChange={(e) => setInput(e.target.value)}
            disabled={cmd.busy}
          />
          <Button type="submit" variant="secondary" disabled={cmd.busy}>
            {t("bankSearchAction")}
          </Button>
        </form>
        {failed ? (
          <Alert tone="danger">
            {t("bankFailed")}{" "}
            <Button
              variant="secondary"
              onClick={() => setAttempt((x) => x + 1)}
            >
              {t("bankRetry")}
            </Button>
          </Alert>
        ) : !choices ? (
          <p role="status">{t("bankLoading")}</p>
        ) : (
          <div className="space-y-control">
            {choices.length === 0 && <p>{t("bankNoCandidates")}</p>}
            {choices.map((c) => (
              <div
                key={`${c.target_kind}:${c.document_id}:${c.payment_id ?? "new"}`}
                className="flex items-center justify-between gap-control border-b border-border pb-control"
              >
                <div>
                  <span className="font-medium">
                    {c.number} · {c.partner}
                  </span>
                  <p className="text-sm text-muted">
                    {t(c.payment_id ? "bankUseExisting" : "bankCreatePayment")}{" "}
                    · {moneyText(c.available, language, movement.data.currency)}
                  </p>
                </div>
                <Button
                  variant="secondary"
                  disabled={
                    cmd.busy ||
                    rows.length >= 25 ||
                    rows.some(
                      (r) =>
                        r.candidate.target_kind === c.target_kind &&
                        r.candidate.document_id === c.document_id,
                    )
                  }
                  onClick={() =>
                    setRows((v) => [
                      ...v,
                      { candidate: c, amount: c.available },
                    ])
                  }
                >
                  {t("bankSplitAdd")}
                </Button>
              </div>
            ))}
          </div>
        )}
        {rows.length > 0 && (
          <form
            className="space-y-field"
            onSubmit={async (e) => {
              e.preventDefault();
              if (!valid) return;
              const r = await cmd.execute({
                action: "split",
                request_id: "",
                movement_id: movement.id,
                revision: movement.revision,
                allocations: rows.map(({ candidate: c, amount }) => ({
                  target_kind: c.target_kind,
                  document_id: c.document_id,
                  target_revision: c.revision,
                  payment_id: c.payment_id,
                  amount: decimal(cents(amount)!),
                })),
              });
              if (r?.kind === "movement") onSaved(r.value);
            }}
          >
            <Heading level={3} size="card">
              {t("bankAllocated")}
            </Heading>
            {rows.map((r, i) => (
              <div
                key={`${r.candidate.target_kind}:${r.candidate.document_id}`}
                className="space-y-control border-b border-border pb-field"
              >
                <Field
                  label={t("bankSplitAmount", { number: r.candidate.number })}
                  inputMode="decimal"
                  value={r.amount}
                  readOnly={!!r.candidate.payment_id}
                  disabled={cmd.busy}
                  onChange={(e) =>
                    setRows((v) =>
                      v.map((item, j) =>
                        i === j ? { ...item, amount: e.target.value } : item,
                      ),
                    )
                  }
                />
                <p className="text-sm text-muted">
                  {t("bankSplitLimit", {
                    amount: moneyText(
                      r.candidate.available,
                      language,
                      movement.data.currency,
                    ),
                  })}
                </p>
                <Button
                  variant="ghost"
                  disabled={cmd.busy}
                  onClick={() => setRows((v) => v.filter((_, j) => i !== j))}
                >
                  {t("bankSplitRemove", { number: r.candidate.number })}
                </Button>
              </div>
            ))}
            <p aria-live="polite">
              {t("bankSplitAfter")}:{" "}
              <strong>
                {moneyText(
                  decimal(remaining),
                  language,
                  movement.data.currency,
                )}
              </strong>
            </p>
            {!valid && <Alert tone="warning">{t("bankSplitInvalid")}</Alert>}
            <Button type="submit" disabled={cmd.busy || !valid}>
              {t(cmd.busy ? "bankSaving" : "bankSplitSave")}
            </Button>
          </form>
        )}
        <Button variant="secondary" disabled={cmd.busy} onClick={onClose}>
          {t("bankClose")}
        </Button>
      </div>
    </Dialog>
  );
}
