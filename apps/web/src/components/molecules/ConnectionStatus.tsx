import { useAppTranslation } from "../../locales";
export function ConnectionStatus({
  label,
  connected,
  loading,
}: {
  label: string;
  connected: boolean;
  loading: boolean;
}) {
  const { t } = useAppTranslation();
  return (
    <div className="flex items-center justify-between border-b border-border py-5 last:border-0">
      <span className="font-medium">{label}</span>
      <span
        className={connected ? "text-sm text-primary" : "text-sm text-muted"}
      >
        {loading
          ? t("checking")
          : connected
            ? t("connected")
            : t("disconnected")}
      </span>
    </div>
  );
}
