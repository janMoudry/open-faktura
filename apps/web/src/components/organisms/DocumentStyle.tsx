import {
  Alert,
  Button,
  Checkbox,
  Field,
  Heading,
  TextareaField,
} from "@open-faktura/ui";
import { useDocumentStyle } from "../../hooks/useDocumentStyle";
import { useAppTranslation } from "../../locales";
export function DocumentStyle({
  company,
  editable,
}: {
  company: string;
  editable: boolean;
}) {
  const { t } = useAppTranslation();
  const { data, setData, busy, error, dirty, save, image } =
    useDocumentStyle(company);
  return (
    <section className="mt-group space-y-field border-t border-border pt-group">
      <Heading level={2}>{t("documentStyleTitle")}</Heading>
      <p className="text-sm text-muted">{t("documentStyleHint")}</p>
      {error && <Alert tone="danger">{t("documentStyleError")}</Alert>}
      {data && (
        <form
          className="space-y-field"
          onSubmit={(e) => {
            e.preventDefault();
            void save();
          }}
        >
          <fieldset disabled={!editable || busy} className="space-y-field">
            {(
              [
                ["logo_base64", "documentStyleLogo"],
                ["signature_base64", "documentStyleSignature"],
              ] as const
            ).map(([key, label]) => (
              <div key={key} className="space-y-control">
                <Field
                  label={t(label)}
                  type="file"
                  accept="image/png,image/jpeg"
                  onChange={(e) => {
                    const file = e.target.files?.[0];
                    if (file) void image(file, key);
                  }}
                />
                {data[key] && (
                  <div className="space-y-control">
                    <img
                      className="max-h-24 max-w-full object-contain"
                      src={`data:${data[key]?.startsWith("iVBOR") ? "image/png" : "image/jpeg"};base64,${data[key]}`}
                      alt={t(label)}
                    />
                    <Button
                      variant="secondary"
                      onClick={() => setData({ ...data, [key]: null })}
                    >
                      {t("documentStyleRemoveImage")}
                    </Button>
                  </div>
                )}
              </div>
            ))}
            <TextareaField
              label={t("documentStyleBeforeCs")}
              value={data.before_items_cs ?? ""}
              maxLength={1000}
              rows={3}
              onChange={(e) =>
                setData({ ...data, before_items_cs: e.target.value })
              }
            />
            <TextareaField
              label={t("documentStyleBeforeEn")}
              value={data.before_items_en ?? ""}
              maxLength={1000}
              rows={3}
              onChange={(e) =>
                setData({ ...data, before_items_en: e.target.value })
              }
            />
            <p className="text-sm text-muted">{t("documentStyleBeforeHint")}</p>
            <Field
              label={t("documentStyleFooterCs")}
              value={data.footer_cs}
              maxLength={500}
              onChange={(e) => setData({ ...data, footer_cs: e.target.value })}
            />
            <Field
              label={t("documentStyleFooterEn")}
              value={data.footer_en}
              maxLength={500}
              onChange={(e) => setData({ ...data, footer_en: e.target.value })}
            />
            <Checkbox
              label={t("deliveryPricesDefault")}
              checked={data.delivery_note_prices ?? false}
              onChange={(e) =>
                setData({ ...data, delivery_note_prices: e.target.checked })
              }
            />
            <p className="text-sm text-muted">
              {t("deliveryPricesDefaultHint")}
            </p>
          </fieldset>
          {editable && (
            <Button type="submit" disabled={busy || !dirty}>
              {t("documentStyleSave")}
            </Button>
          )}
        </form>
      )}
    </section>
  );
}
