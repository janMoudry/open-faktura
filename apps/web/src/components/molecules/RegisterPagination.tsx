import { Button, Icon, Select } from "@open-faktura/ui";
import { useAppTranslation } from "../../locales";

/** Shared register navigation; the caller keeps filters and paging in the URL. */
export function RegisterPagination({
  page,
  pageSize,
  total,
  onPage,
  onPageSize,
}: {
  page: number;
  pageSize: number;
  total: number;
  onPage: (page: number) => void;
  onPageSize: (size: string) => void;
}) {
  const { t } = useAppTranslation();
  const pages = Math.max(1, Math.ceil(total / pageSize));
  return (
    <div className="register-pagination">
      <p className="text-sm text-muted" aria-live="polite">
        {t("invoiceRange", {
          from: total ? (page - 1) * pageSize + 1 : 0,
          to: Math.min(page * pageSize, total),
          total,
        })}
      </p>
      <nav
        aria-label={t("registerPages")}
        className="flex items-center gap-inline"
      >
        <Button
          size="sm"
          variant="ghost"
          className="ui-icon-button"
          aria-label={t("registerFirst")}
          title={t("registerFirst")}
          disabled={page <= 1}
          onClick={() => onPage(1)}
        >
          <Icon name="firstPage" />
        </Button>
        <Button
          size="sm"
          variant="ghost"
          className="ui-icon-button"
          aria-label={t("contactPrevious")}
          title={t("contactPrevious")}
          disabled={page <= 1}
          onClick={() => onPage(page - 1)}
        >
          <Icon name="chevronLeft" />
        </Button>
        <span
          className="register-current-page"
          aria-current="page"
          aria-label={t("registerCurrent", { page, pages })}
        >
          {page}
        </span>
        <Button
          size="sm"
          variant="ghost"
          className="ui-icon-button"
          aria-label={t("contactNext")}
          title={t("contactNext")}
          disabled={page >= pages}
          onClick={() => onPage(page + 1)}
        >
          <Icon name="chevronRight" />
        </Button>
        <Button
          size="sm"
          variant="ghost"
          className="ui-icon-button"
          aria-label={t("registerLast")}
          title={t("registerLast")}
          disabled={page >= pages}
          onClick={() => onPage(pages)}
        >
          <Icon name="lastPage" />
        </Button>
      </nav>
      <Select
        label={t("invoicePageSize")}
        value={String(pageSize)}
        options={[10, 25, 50, 100].map((n) => ({
          value: String(n),
          label: String(n),
        }))}
        onChange={onPageSize}
      />
    </div>
  );
}
