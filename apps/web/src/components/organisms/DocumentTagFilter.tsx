import { Select } from "@open-faktura/ui";
import { useDocumentTags } from "../../hooks/useDocumentTags";
import { useAppTranslation } from "../../locales";
export function DocumentTagFilter({
  company,
  value,
  onChange,
}: {
  company: string;
  value: string;
  onChange: (value: string) => void;
}) {
  const { all, error } = useDocumentTags(company);
  const { t } = useAppTranslation();
  return (
    <div className="w-full sm:max-w-xs">
      <Select
        label={t("documentTagsFilter")}
        value={value}
        onChange={onChange}
        disabled={error}
        hint={error ? t("documentTagsError") : undefined}
        options={[
          { value: "", label: t("documentTagsAll") },
          ...all.map((tag) => ({ value: tag.id, label: tag.name })),
        ]}
      />
    </div>
  );
}
