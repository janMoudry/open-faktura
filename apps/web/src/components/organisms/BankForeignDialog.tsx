import { useEffect, useState } from "react";
import { Alert, Button, Dialog, Field, Select } from "@open-faktura/ui";
import {
  bankForeignCandidates,
  type BankForeignCandidate,
  type BankMovement,
} from "@open-faktura/api-client";
import { useAppTranslation } from "../../locales";
import { useBankCommand } from "../../hooks/useBankCommand";
import { apiUrl } from "../../services/status";
import { moneyText } from "../../utils/invoice";
export function BankForeignDialog({
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
    cmd = useBankCommand(companyId);
  const [choices, setChoices] = useState<BankForeignCandidate[]>([]);
  const [query, setQuery] = useState(""),
    [failed, setFailed] = useState(false),
    [loading, setLoading] = useState(true);
  const [selected, setSelected] = useState<BankForeignCandidate | null>(null);
  const [bankAmount, setBankAmount] = useState(movement.remaining),
    [documentAmount, setDocumentAmount] = useState("");
  const [bankRate, setBankRate] = useState(""),
    [bankUnits, setBankUnits] = useState("1");
  useEffect(() => {
    const c = new AbortController();
    setLoading(true);
    setFailed(false);
    const timer = setTimeout(
      () =>
        void bankForeignCandidates(
          apiUrl,
          companyId,
          movement.id,
          query,
          c.signal,
        )
          .then((v) => {
            if (!c.signal.aborted) setChoices(v);
          })
          .catch(() => {
            if (!c.signal.aborted) setFailed(true);
          })
          .finally(() => {
            if (!c.signal.aborted) setLoading(false);
          }),
      200,
    );
    return () => {
      clearTimeout(timer);
      c.abort();
    };
  }, [companyId, movement.id, query]);
  return (
    <Dialog
      open
      title={t("bankForeignTitle")}
      description={t("bankForeignHelp")}
      onClose={cmd.busy ? () => {} : onClose}
    >
      <form
        className="space-y-field"
        onSubmit={async (e) => {
          e.preventDefault();
          if (!selected || cmd.busy) return;
          const r = await cmd.execute({
            request_id: crypto.randomUUID(),
            action: "foreign_match",
            movement_id: movement.id,
            revision: movement.revision,
            target_kind: selected.target_kind as
              "invoice" | "received" | "advance" | "credit",
            document_id: selected.document_id,
            payment_id: selected.payment_id,
            target_revision: selected.revision,
            ...(movement.data.currency !== "CZK"
              ? {
                  bank_rate: {
                    rate: bankRate.replace(",", "."),
                    units: bankUnits,
                  },
                }
              : {}),
            bank_amount: bankAmount.replace(",", "."),
            document_amount: documentAmount.replace(",", "."),
          });
          if (r?.kind === "movement") onSaved(r.value);
        }}
      >
        <p className="text-sm text-muted">
          {t(
            selected?.payment_id
              ? "bankForeignExistingHelp"
              : "bankForeignChooseHelp",
          )}
        </p>
        {cmd.error && <Alert tone="danger">{t(cmd.error)}</Alert>}
        <Field
          label={t("bankForeignSearch")}
          value={query}
          disabled={cmd.busy}
          onChange={(e) => setQuery(e.target.value)}
        />
        {failed && <Alert tone="danger">{t("bankForeignLoadFailed")}</Alert>}
        {loading ? (
          <p role="status">{t("bankForeignLoading")}</p>
        ) : (
          <Select
            label={t("bankForeignDocument")}
            value={
              selected
                ? `${selected.target_kind}:${selected.document_id}:${selected.payment_id ?? "new"}`
                : ""
            }
            disabled={cmd.busy}
            options={choices.map((c) => ({
              value: `${c.target_kind}:${c.document_id}:${c.payment_id ?? "new"}`,
              label: `${c.number} · ${c.partner} · ${moneyText(c.payment_amount ?? c.outstanding, language, c.currency)} · ${c.payment_id ? t("bankPaymentId", { id: c.payment_id }) : t("bankCreatePayment")}`,
            }))}
            onChange={(id) => {
              const c =
                choices.find(
                  (c) =>
                    `${c.target_kind}:${c.document_id}:${c.payment_id ?? "new"}` ===
                    id,
                ) ?? null;
              setSelected(c);
              setDocumentAmount(c?.payment_amount ?? c?.outstanding ?? "");
            }}
          />
        )}
        {!loading && !failed && choices.length === 0 && (
          <p>{t("bankNoCandidates")}</p>
        )}
        <Field
          label={t("bankForeignBankAmount", {
            currency: movement.data.currency,
          })}
          value={bankAmount}
          inputMode="decimal"
          disabled={cmd.busy}
          onChange={(e) => setBankAmount(e.target.value)}
        />
        {movement.data.currency !== "CZK" && (
          <div className="space-y-field">
            <Field
              label={t("bankForeignRate", { currency: movement.data.currency })}
              value={bankRate}
              inputMode="decimal"
              required
              disabled={cmd.busy}
              onChange={(e) => setBankRate(e.target.value)}
            />
            <Field
              label={t("bankForeignUnits", {
                currency: movement.data.currency,
              })}
              value={bankUnits}
              inputMode="numeric"
              required
              disabled={cmd.busy}
              onChange={(e) => setBankUnits(e.target.value)}
            />
          </div>
        )}
        {selected && (
          <Field
            label={t("bankForeignDocumentAmount", {
              currency: selected.currency,
            })}
            value={documentAmount}
            inputMode="decimal"
            disabled={cmd.busy || !!selected.payment_id}
            onChange={(e) => setDocumentAmount(e.target.value)}
          />
        )}
        <p className="text-sm text-muted">{t("bankForeignDifferenceHelp")}</p>
        <Button
          type="submit"
          disabled={cmd.busy || !selected || !bankAmount || !documentAmount}
        >
          {t(cmd.busy ? "bankSaving" : "bankMatch")}
        </Button>
      </form>
    </Dialog>
  );
}
