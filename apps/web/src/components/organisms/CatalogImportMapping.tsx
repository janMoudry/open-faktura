import { Select } from "@open-faktura/ui";
import { useAppTranslation, type AppKey } from "../../locales";
import {
  catalogImportFields,
  requiredCatalogFields,
  type CatalogImportMapping as Mapping,
} from "../../utils/catalogImport";
export function CatalogImportMapping({
  headers,
  mapping,
  onChange,
  disabled,
}: {
  headers: string[];
  mapping: Mapping;
  onChange: (value: Mapping) => void;
  disabled: boolean;
}) {
  const { t } = useAppTranslation();
  return (
    <div className="grid gap-field sm:grid-cols-2 lg:grid-cols-3">
      {catalogImportFields.map((k) => (
        <Select
          key={k}
          label={t(`importCatalogField_${k}` as AppKey)}
          required={requiredCatalogFields.includes(k)}
          disabled={disabled}
          value={String(mapping[k])}
          onChange={(v) => onChange({ ...mapping, [k]: Number(v) })}
          options={[
            { value: "-1", label: t("importNotMapped") },
            ...headers.map((h, i) => ({ value: String(i), label: h })),
          ]}
        />
      ))}
    </div>
  );
}
