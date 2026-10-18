import { useState } from "react";
import { Alert, Button, Dialog, Select } from "@open-faktura/ui";
import { useAppTranslation } from "../../locales";
import { useDocumentOutput } from "../../hooks/useDocumentOutput";
export function DocumentOutput({
  company,
  id,
}: {
  company: string;
  id: string;
}) {
  const { t, language } = useAppTranslation();
  const { busy, error, download } = useDocumentOutput(company);
  const [delivery, setDelivery] = useState(false);
  const [prices, setPrices] = useState("default");
  return (
    <div className="space-y-control">
      <div className="flex flex-wrap gap-control">
        {(
          [
            ["rtf", "documentOutputRtf"],
            ["delivery_note", "documentOutputDelivery"],
            ["payment_confirmation", "documentOutputPayment"],
          ] as const
        ).map(([kind, label]) => (
          <Button
            key={kind}
            variant="secondary"
            disabled={busy}
            onClick={() => {
              if (kind === "delivery_note") {
                setPrices("default");
                setDelivery(true);
              } else void download({ ids: [id], kind, language });
            }}
          >
            {t(label)}
          </Button>
        ))}
      </div>
      <Dialog
        open={delivery}
        title={t("documentOutputDelivery")}
        description={t("deliveryPricesHint")}
        onClose={() => {
          if (!busy) setDelivery(false);
        }}
      >
        <div className="space-y-field">
          <Select
            label={t("deliveryPricesChoice")}
            value={prices}
            onChange={setPrices}
            disabled={busy}
            options={[
              { value: "default", label: t("deliveryPricesCompany") },
              { value: "yes", label: t("deliveryPricesWith") },
              { value: "no", label: t("deliveryPricesWithout") },
            ]}
          />
          {error && <Alert tone="danger">{t("documentOutputError")}</Alert>}
          <div className="flex justify-end gap-control">
            <Button
              variant="secondary"
              disabled={busy}
              onClick={() => setDelivery(false)}
            >
              {t("cancelAction")}
            </Button>
            <Button
              disabled={busy}
              onClick={() =>
                void download({
                  ids: [id],
                  kind: "delivery_note",
                  language,
                  ...(prices === "default"
                    ? {}
                    : { delivery_note_prices: prices === "yes" }),
                })
              }
            >
              {t("documentOutputDownload")}
            </Button>
          </div>
        </div>
      </Dialog>
      {error && <Alert tone="danger">{t("documentOutputError")}</Alert>}
    </div>
  );
}
export function BulkDocumentOutput({
  company,
  available,
}: {
  company: string;
  available: { id: string; number: string }[];
}) {
  const { t, language } = useAppTranslation();
  const { busy, error, download } = useDocumentOutput(company);
  const [open, setOpen] = useState(false);
  const [selected, setSelected] = useState<string[]>([]);
  return (
    <>
      <Button
        variant="secondary"
        disabled={!available.length}
        onClick={() => {
          setSelected(available.slice(0, 50).map((i) => i.id));
          setOpen(true);
        }}
      >
        {t("documentOutputBulk")}
      </Button>
      <Dialog
        open={open}
        onClose={() => {
          if (!busy) setOpen(false);
        }}
        title={t("documentOutputBulk")}
        description={t("documentOutputBulkHint")}
      >
        <div className="space-y-field">
          {available.map((item) => (
            <label key={item.id} className="flex items-center gap-control">
              <input
                type="checkbox"
                disabled={
                  busy || (!selected.includes(item.id) && selected.length >= 50)
                }
                checked={selected.includes(item.id)}
                onChange={(e) =>
                  setSelected((v) =>
                    e.target.checked
                      ? [...v, item.id]
                      : v.filter((id) => id !== item.id),
                  )
                }
              />
              {item.number}
            </label>
          ))}
          {error && <Alert tone="danger">{t("documentOutputError")}</Alert>}
          <div className="flex justify-end gap-control">
            <Button
              variant="secondary"
              disabled={busy}
              onClick={() => setOpen(false)}
            >
              {t("cancelAction")}
            </Button>
            <Button
              disabled={busy || !selected.length}
              onClick={() =>
                void download({ ids: selected, kind: "bulk_pdf", language })
              }
            >
              {t("documentOutputDownload")}
            </Button>
          </div>
        </div>
      </Dialog>
    </>
  );
}
