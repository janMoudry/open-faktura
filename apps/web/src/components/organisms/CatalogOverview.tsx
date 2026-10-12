import type { CatalogData } from "@open-faktura/api-client";
import { Heading } from "@open-faktura/ui";
import { useAppTranslation } from "../../locales";
import { moneyText } from "../../utils/invoice";
export function CatalogOverview({ data }: { data: CatalogData }) {
  const { t, language } = useAppTranslation();
  return (
    <div className="workspace-panel p-form-inset">
      <Heading level={2} size="card">
        {t("catalogIdentity")}
      </Heading>
      <dl className="mt-field grid gap-inline max-w-3xl">
        {[
          [t("catalogName"), data.name],
          [t("catalogCode"), data.code],
          [t("catalogBarcode"), data.barcode],
          [t("catalogQuantity"), data.quantity],
          [t("draftUnit"), data.unit],
          [t("catalogPrice"), moneyText(data.unit_price, language)],
          [
            t("draftPriceMode"),
            t(data.price_mode === "net" ? "draftNetPrice" : "draftGrossPrice"),
          ],
          [t("draftVat"), `${data.vat_percent} %`],
        ].map(([label, value]) => (
          <div key={label} className="grid sm:grid-cols-2 gap-inline text-sm">
            <dt className="text-muted">{label}</dt>
            <dd className="break-words">{value || "—"}</dd>
          </div>
        ))}
      </dl>
    </div>
  );
}
