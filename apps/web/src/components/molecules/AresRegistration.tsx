import { Button, Field } from "@open-faktura/ui";
import { useAppTranslation, type AppKey } from "../../locales";
export function AresRegistration({
  value,
  compact = false,
  onChange,
  disabled,
  error,
  ares,
}: {
  value: string;
  compact?: boolean;
  onChange: (value: string) => void;
  disabled: boolean;
  error?: string;
  ares: { loading: boolean; message: AppKey | ""; lookup: () => void };
}) {
  const { t } = useAppTranslation();
  return (
    <div
      className={
        compact
          ? "space-y-control"
          : "space-y-control rounded-control border border-border bg-canvas p-field"
      }
    >
      {!compact && (
        <div>
          <p className="text-sm font-semibold">{t("aresLookup")}</p>
          <p className="mt-inline text-sm text-muted">{t("aresIntro")}</p>
        </div>
      )}
      <div className="flex flex-wrap items-end gap-control">
        <div className="min-w-0 flex-1">
          <Field
            label={t("registrationNumber")}
            value={value}
            readOnly={disabled}
            maxLength={32}
            error={error}
            onChange={(e) => onChange(e.target.value)}
            onBlur={!disabled ? ares.lookup : undefined}
          />
        </div>
        {!disabled && (
          <Button
            type="button"
            variant="secondary"
            loading={ares.loading}
            disabled={!value.trim()}
            onClick={ares.lookup}
          >
            {t("aresLookup")}
          </Button>
        )}
      </div>
      {!disabled && (
        <p
          role="status"
          className={`text-sm ${ares.message && !["aresLoading", "aresFilled"].includes(ares.message) ? "text-danger" : "text-muted"}`}
        >
          {t(ares.message || "aresHint")}
        </p>
      )}
    </div>
  );
}
