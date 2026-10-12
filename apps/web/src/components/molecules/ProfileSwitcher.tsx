import { Link } from "react-router";
import { useEffect, useId, useRef, useState } from "react";
import { Button, Icon } from "@open-faktura/ui";
import { useAppTranslation } from "../../locales";
export function ProfileSwitcher({
  profiles,
  selected,
  busy,
  onCreate,
  onSelect,
}: {
  profiles: { value: string; label: string }[];
  selected?: string;
  busy: boolean;
  onCreate: () => void;
  onSelect: (id: string) => void;
}) {
  const { t } = useAppTranslation();
  const [open, setOpen] = useState(false);
  const root = useRef<HTMLDivElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  const id = useId();
  useEffect(() => {
    if (!open) return;
    const outside = (event: PointerEvent) => {
      if (!root.current?.contains(event.target as Node)) setOpen(false);
    };
    const key = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setOpen(false);
        trigger.current?.focus();
      }
    };
    document.addEventListener("pointerdown", outside);
    document.addEventListener("keydown", key);
    return () => {
      document.removeEventListener("pointerdown", outside);
      document.removeEventListener("keydown", key);
    };
  }, [open]);
  return (
    <div
      ref={root}
      className="relative min-w-0"
      onBlur={(event) => {
        if (
          event.relatedTarget &&
          !event.currentTarget.contains(event.relatedTarget)
        )
          setOpen(false);
      }}
    >
      <button
        ref={trigger}
        type="button"
        disabled={busy}
        aria-expanded={open}
        aria-controls={id}
        className="flex max-w-full items-center gap-control rounded-control px-control py-inline text-left text-sm hover:bg-canvas"
        onClick={() => setOpen(!open)}
      >
        <span className="hidden h-8 w-8 shrink-0 sm:flex items-center justify-center rounded-control bg-primary-soft">
          <Icon name="invoice" />
        </span>
        <span className="min-w-0">
          <span className="block text-xs text-muted">{t("switchCompany")}</span>
          <span className="block max-w-48 truncate font-semibold">
            {profiles.find((p) => p.value === selected)?.label ||
              t("chooseProfile")}
          </span>
        </span>
        <Icon name="chevronDown" />
      </button>
      {open && (
        <div
          id={id}
          role="region"
          aria-label={t("switchCompany")}
          className="fixed left-field right-field top-workspace-appbar z-50 mt-inline sm:absolute sm:left-0 sm:right-auto sm:top-full sm:w-80 sm:max-w-[calc(100vw-2rem)] rounded-popover border border-border bg-surface shadow-paper"
        >
          <p className="px-field pt-field text-xs font-semibold text-muted">
            {t("profileSwitcherHint")}
          </p>
          <div className="max-h-64 overflow-y-auto p-inline">
            {profiles.map((profile) => (
              <Button
                key={profile.value}
                variant="ghost"
                className="w-full justify-between"
                aria-pressed={selected === profile.value}
                onClick={() => {
                  setOpen(false);
                  onSelect(profile.value);
                  trigger.current?.focus();
                }}
              >
                <span className="min-w-0 truncate">{profile.label}</span>
                {selected === profile.value && <Icon name="check" />}
              </Button>
            ))}
          </div>
          <div className="space-y-inline border-t border-border p-inline">
            <Link
              to="/companies"
              onClick={() => setOpen(false)}
              className="flex items-center gap-control rounded-control p-control text-sm hover:bg-canvas"
            >
              <Icon name="settings" />
              {t("manageProfiles")}
            </Link>
            <Button
              variant="ghost"
              className="w-full justify-start"
              onClick={() => {
                setOpen(false);
                onCreate();
              }}
            >
              <Icon name="plus" />
              {t("newCompany")}
            </Button>
          </div>
        </div>
      )}
    </div>
  );
}
