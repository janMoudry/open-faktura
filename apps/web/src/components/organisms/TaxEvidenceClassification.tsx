import { useEffect, useState } from "react";
import {
  Alert,
  Button,
  DatePicker,
  Dialog,
  Field,
  Select,
  TextareaField,
} from "@open-faktura/ui";
import {
  taxEvidenceSource,
  setTaxEvidence,
  taxEvidenceSummary,
  SalesFailure,
  type TaxEvidenceSource,
  type TaxEvidenceSummary,
} from "@open-faktura/api-client";
import { useAppTranslation } from "../../locales";
import { apiUrl } from "../../services/status";
import { moneyText } from "../../utils/invoice";
export function TaxClassificationSummary({
  company,
  year,
  refresh,
  onExpired,
}: {
  company: string;
  year: number;
  refresh: number;
  onExpired: () => void;
}) {
  const { t, language } = useAppTranslation();
  const [value, setValue] = useState<TaxEvidenceSummary | null>(null),
    [failed, setFailed] = useState(false);
  useEffect(() => {
    let active = true;
    setValue(null);
    setFailed(false);
    taxEvidenceSummary(apiUrl, company, year)
      .then((v) => {
        if (active) setValue(v);
      })
      .catch((e) => {
        if (active) {
          setFailed(true);
          if (e instanceof SalesFailure && e.status === 401) onExpired();
        }
      });
    return () => {
      active = false;
    };
  }, [company, year, refresh, onExpired]);
  return (
    <section
      className="workspace-panel p-field space-y-field"
      aria-label={t("taxClassificationTitle")}
    >
      <p className="font-semibold">{t("taxClassificationTitle")}</p>
      <p className="text-sm text-muted">{t("taxClassificationScope")}</p>
      {failed && (
        <Alert tone="warning" role="alert">
          {t("taxClassificationLoadError")}
        </Alert>
      )}
      {value && (
        <>
          <dl className="grid gap-field sm:grid-cols-2">
            <div>
              <dt>{t("taxClassifiedTurnover")}</dt>
              <dd className="font-semibold tabular-nums">
                {moneyText(value.included_turnover_czk, language)}
              </dd>
            </div>
            <div>
              <dt>{t("taxClaimedInput")}</dt>
              <dd className="font-semibold tabular-nums">
                {moneyText(value.claimed_input_vat_czk, language)}
              </dd>
            </div>
          </dl>
          <p className="text-sm">
            {t("taxClassificationPending", {
              turnover: value.unclassified_turnover,
              input: value.unclassified_input,
              outdated: value.outdated,
            })}
          </p>
        </>
      )}
    </section>
  );
}
export function TaxEvidenceClassification({
  company,
  kind,
  id,
  editable,
  onChanged,
  onExpired,
}: {
  company: string;
  kind: string;
  id: string;
  editable: boolean;
  onChanged: () => void;
  onExpired: () => void;
}) {
  const { t, language } = useAppTranslation();
  const [open, setOpen] = useState(false),
    [source, setSource] = useState<TaxEvidenceSource | null>(null),
    [loading, setLoading] = useState(false),
    [busy, setBusy] = useState(false),
    [failed, setFailed] = useState(false),
    [turnover, setTurnover] = useState("unknown"),
    [claim, setClaim] = useState(""),
    [claimOn, setClaimOn] = useState(""),
    [reason, setReason] = useState("");
  async function load() {
    setOpen(true);
    setLoading(true);
    setFailed(false);
    setSource(null);
    try {
      const value = await taxEvidenceSource(apiUrl, company, kind, id);
      setSource(value);
      const old = value.classification?.data;
      setTurnover(
        old?.turnover_included === true
          ? "included"
          : old?.turnover_included === false
            ? "excluded"
            : "unknown",
      );
      setClaim(old?.claimed_input_vat_czk ?? "");
      setClaimOn(old?.claim_on ?? value.observation.vat_on ?? "");
      setReason(old?.reason ?? "");
    } catch (e) {
      setFailed(true);
      if (e instanceof SalesFailure && e.status === 401) onExpired();
    } finally {
      setLoading(false);
    }
  }
  async function save() {
    if (!source) return;
    setBusy(true);
    setFailed(false);
    try {
      await setTaxEvidence(apiUrl, company, kind, id, {
        request_id: crypto.randomUUID(),
        revision: source.classification?.revision ?? 0,
        fingerprint: source.observation.fingerprint,
        data: {
          turnover_included:
            source.observation.turnover_net_czk === null
              ? null
              : turnover === "included"
                ? true
                : turnover === "excluded"
                  ? false
                  : null,
          claimed_input_vat_czk: claim.trim()
            ? claim.trim().replace(",", ".")
            : null,
          claim_on: claim.trim() ? claimOn || null : null,
          reason,
        },
      });
      setOpen(false);
      onChanged();
    } catch (e) {
      setFailed(true);
      if (e instanceof SalesFailure && e.status === 401) onExpired();
    } finally {
      setBusy(false);
    }
  }
  return (
    <>
      <Button size="sm" variant="secondary" onClick={load}>
        {t(editable ? "taxClassify" : "taxClassificationView")}
      </Button>
      <Dialog
        open={open}
        onClose={() => {
          if (!busy) setOpen(false);
        }}
        title={t("taxClassificationTitle")}
        description={t("taxClassificationScope")}
      >
        {loading && <p role="status">{t("draftLoading")}</p>}
        {source && (
          <div className="space-y-field">
            <p className="font-semibold">{source.observation.number}</p>
            {source.classification && !source.classification.current && (
              <Alert tone="warning">{t("taxClassificationStale")}</Alert>
            )}
            {source.observation.turnover_net_czk !== null && (
              <>
                <p>
                  {t("taxClassificationRecordedNet", {
                    amount: moneyText(
                      source.observation.turnover_net_czk,
                      language,
                    ),
                  })}
                </p>
                <Select
                  label={t("taxTurnoverTreatment")}
                  value={turnover}
                  disabled={!editable || busy}
                  onChange={setTurnover}
                  options={[
                    { value: "unknown", label: t("taxUnclassified") },
                    { value: "included", label: t("taxInclude") },
                    { value: "excluded", label: t("taxExclude") },
                  ]}
                />
              </>
            )}
            {source.observation.side === "purchases" &&
              source.observation.recorded_vat_czk !== null && (
                <>
                  <p>
                    {t("taxClassificationRecordedVat", {
                      amount: moneyText(
                        source.observation.recorded_vat_czk,
                        language,
                      ),
                    })}
                  </p>
                  <Field
                    label={t("taxClaimAmount")}
                    inputMode="decimal"
                    value={claim}
                    disabled={!editable || busy}
                    onChange={(e) => setClaim(e.target.value)}
                  />
                  <DatePicker
                    label={t("taxClaimDate")}
                    value={claimOn}
                    onChange={setClaimOn}
                    disabled={!editable || busy}
                  />
                  <p className="text-sm text-muted">{t("taxClaimHint")}</p>
                </>
              )}
            <TextareaField
              label={t("taxClassificationReason")}
              value={reason}
              onChange={(e) => setReason(e.target.value)}
              disabled={!editable || busy}
              maxLength={1000}
            />
          </div>
        )}
        {failed && (
          <Alert tone="danger" role="alert">
            {t("taxClassificationSaveError")}{" "}
            <Button
              variant="secondary"
              size="sm"
              onClick={load}
              disabled={busy}
            >
              {t("reloadCompany")}
            </Button>
          </Alert>
        )}
        <div className="mt-field flex gap-control">
          <Button
            variant="secondary"
            disabled={busy}
            onClick={() => setOpen(false)}
          >
            {t("cancelAction")}
          </Button>
          {editable && (
            <Button
              loading={busy}
              disabled={loading || !source || !reason.trim()}
              onClick={save}
            >
              {t("accountSave")}
            </Button>
          )}
        </div>
      </Dialog>
    </>
  );
}
