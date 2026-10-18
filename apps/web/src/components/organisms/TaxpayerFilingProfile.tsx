import { useEffect, useState } from "react";
import { Alert, Button, Dialog, Field, Select } from "@open-faktura/ui";
import {
  epoOffices,
  taxpayerFilingProfile,
  SalesFailure,
  type TaxpayerProfileData,
} from "@open-faktura/api-client";
import { useAppTranslation } from "../../locales";
import { apiUrl } from "../../services/status";
export function TaxpayerFilingProfile({
  company,
  editable,
  onExpired,
}: {
  company: string;
  editable: boolean;
  onExpired: () => void;
}) {
  const { t } = useAppTranslation();
  const [open, setOpen] = useState(false),
    [busy, setBusy] = useState(false),
    [failed, setFailed] = useState(false),
    [revision, setRevision] = useState(0),
    [data, setData] = useState<TaxpayerProfileData | null>(null);
  useEffect(() => {
    setOpen(false);
    setData(null);
    setFailed(false);
  }, [company]);
  const empty: TaxpayerProfileData = {
    tax_office: "",
    tax_office_workplace: null,
    taxpayer_number: "",
    taxpayer_kind: "F",
    vat_filer_kind: "P",
  };
  async function load() {
    setOpen(true);
    setBusy(true);
    setFailed(false);
    setData(null);
    try {
      const v = await taxpayerFilingProfile(apiUrl, company);
      setRevision(v.revision);
      setData(v.data ?? empty);
    } catch (e) {
      setFailed(true);
      if (e instanceof SalesFailure && e.status === 401) onExpired();
    } finally {
      setBusy(false);
    }
  }
  async function save() {
    if (!data) return;
    setBusy(true);
    setFailed(false);
    try {
      const v = await taxpayerFilingProfile(apiUrl, company, {
        revision,
        request_id: crypto.randomUUID(),
        data,
      });
      setRevision(v.revision);
      setData(v.data);
      setOpen(false);
    } catch (e) {
      setFailed(true);
      if (e instanceof SalesFailure && e.status === 401) onExpired();
    } finally {
      setBusy(false);
    }
  }
  return (
    <>
      <Button variant="secondary" size="sm" onClick={load}>
        {t("taxpayerProfileTitle")}
      </Button>
      <Dialog
        open={open}
        onClose={() => {
          if (!busy) setOpen(false);
        }}
        title={t("taxpayerProfileTitle")}
        description={t("taxpayerProfileScope")}
      >
        <div className="space-y-field">
          {failed && (
            <Alert tone="warning" role="alert">
              {t("taxpayerProfileError")}
            </Alert>
          )}
          {data && (
            <>
              <Select
                label={t("taxpayerOffice")}
                value={data.tax_office}
                disabled={!editable || busy}
                onChange={(v) =>
                  setData({
                    ...data,
                    tax_office: v,
                    tax_office_workplace: null,
                  })
                }
                options={[
                  { value: "", label: t("taxpayerChoose") },
                  ...epoOffices.offices.map((o) => ({
                    value: o.code,
                    label: `${o.code} – ${o.name}`,
                  })),
                ]}
              />
              <Select
                label={t("taxpayerWorkplace")}
                value={data.tax_office_workplace ?? ""}
                disabled={!editable || busy}
                onChange={(v) =>
                  setData({ ...data, tax_office_workplace: v || null })
                }
                options={[
                  { value: "", label: t("taxpayerNoWorkplace") },
                  ...epoOffices.workplaces
                    .filter((o) => o.office === data.tax_office)
                    .map((o) => ({
                      value: o.code,
                      label: `${o.code} – ${o.name}`,
                    })),
                ]}
              />
              <Field
                label={t("taxpayerNumber")}
                inputMode="numeric"
                value={data.taxpayer_number}
                disabled={!editable || busy}
                onChange={(e) =>
                  setData({ ...data, taxpayer_number: e.target.value })
                }
              />
              <Select
                label={t("taxpayerKind")}
                value={data.taxpayer_kind}
                disabled={!editable || busy}
                onChange={(v) =>
                  setData({ ...data, taxpayer_kind: v as "F" | "P" })
                }
                options={[
                  { value: "F", label: t("taxpayerNatural") },
                  { value: "P", label: t("taxpayerLegal") },
                ]}
              />
              <Select
                label={t("taxpayerVatKind")}
                value={data.vat_filer_kind}
                disabled={!editable || busy}
                onChange={(v) =>
                  setData({
                    ...data,
                    vat_filer_kind: v as TaxpayerProfileData["vat_filer_kind"],
                  })
                }
                options={[
                  { value: "P", label: t("taxpayerPayer") },
                  { value: "I", label: t("taxpayerIdentified") },
                  { value: "S", label: t("taxpayerGroup") },
                  { value: "N", label: t("taxpayerNonpayer108") },
                  { value: "R", label: t("taxpayerNdpPurchase") },
                  { value: "D", label: t("taxpayerNdpSale") },
                ]}
              />
              {editable && (
                <Button
                  onClick={save}
                  disabled={busy || !data.tax_office || !data.taxpayer_number}
                >
                  {t("accountSave")}
                </Button>
              )}
            </>
          )}
        </div>
      </Dialog>
    </>
  );
}
