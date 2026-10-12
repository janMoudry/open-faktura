import { useLocale } from "../../locales";
import { Icon } from "./Icon";
export function Spinner({ label }: { label?: string }) {
  const { t } = useLocale();
  return (
    <span
      role="status"
      className="inline-flex items-center gap-2 text-sm text-muted"
    >
      <Icon name="loading" className="h-4 w-4 motion-safe:animate-spin" />
      <span>{label ?? t("loading")}</span>
    </span>
  );
}
