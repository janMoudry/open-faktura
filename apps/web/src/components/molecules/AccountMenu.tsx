import { useLocation } from "react-router";
import { useEffect, useId, useRef, useState, type ReactNode } from "react";
import { Button, Icon, Select } from "@open-faktura/ui";
import { useAppTranslation } from "../../locales";

import { useAppearance } from "../../hooks/useAppearance";

export function AccountMenu({
  email,
  name,
  onSettings,
  onSmtp,
  actions,
}: {
  email: string;
  name?: string;
  onSettings?: () => void;
  onSmtp?: () => void;
  actions: ReactNode;
}) {
  const location = useLocation();
  const appearance = useAppearance(email);
  const { t, language, setLanguage } = useAppTranslation();
  const [open, setOpen] = useState(false);
  const root = useRef<HTMLDivElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  const id = useId();
  const selectingLanguage = useRef(false);
  useEffect(() => {
    setOpen(false);
    selectingLanguage.current = false;
  }, [location.key, location.pathname, location.search]);
  useEffect(() => {
    if (!open) return;
    const outside = (event: PointerEvent) => {
      if (
        !selectingLanguage.current &&
        !root.current?.contains(event.target as Node)
      )
        setOpen(false);
    };
    const escape = (event: KeyboardEvent) => {
      if (event.key === "Escape" && !selectingLanguage.current) {
        setOpen(false);
        trigger.current?.focus();
      }
    };
    document.addEventListener("pointerdown", outside);
    document.addEventListener("keydown", escape);
    return () => {
      document.removeEventListener("pointerdown", outside);
      document.removeEventListener("keydown", escape);
    };
  }, [open]);
  return (
    <div
      ref={root}
      className="relative"
      onBlur={(event) => {
        if (
          !selectingLanguage.current &&
          event.relatedTarget &&
          !event.currentTarget.contains(event.relatedTarget)
        )
          setOpen(false);
      }}
    >
      <button
        ref={trigger}
        type="button"
        aria-label={t("accountMenu")}
        aria-expanded={open}
        aria-controls={id}
        onClick={() => setOpen(!open)}
        className="workspace-account-trigger"
      >
        <span className="workspace-account-avatar" aria-hidden="true">
          {name?.trim()
            ? name
                .trim()
                .split(/\s+/)
                .map((part) => part[0])
                .slice(0, 2)
                .join("")
            : email.slice(0, 2)}
        </span>
        <span className="workspace-account-name">{name || email}</span>
        <Icon name="chevronDown" />
      </button>
      {open && (
        <div
          id={id}
          role="region"
          aria-label={t("accountMenu")}
          className="account-popover absolute right-0 top-full z-50 mt-inline w-72 max-w-[calc(100vw-2rem)] overflow-hidden rounded-popover border border-border bg-surface shadow-paper"
        >
          <div className="border-b border-border p-field">
            <p className="text-sm font-semibold">{name || t("accountTitle")}</p>
            <p className="mt-inline break-all text-xs text-muted">{email}</p>
          </div>
          <div className="p-inline">
            <Button
              variant="ghost"
              className="account-menu-item"
              onClick={() => {
                setOpen(false);
                trigger.current?.focus();
                onSettings?.();
              }}
            >
              <Icon name="settings" />
              <span>{t("accountSettings")}</span>
            </Button>
          </div>
          {onSmtp && (
            <div className="px-inline pb-inline">
              <Button
                variant="ghost"
                className="account-menu-item"
                onClick={() => {
                  setOpen(false);
                  trigger.current?.focus();
                  onSmtp();
                }}
              >
                <Icon name="settings" />
                <span>{t("smtpTitle")}</span>
              </Button>
            </div>
          )}
          <div
            className="border-t border-border px-field py-control"
            role="group"
            aria-label={t("accountLanguage")}
          >
            <div className="mb-inline flex items-center gap-control text-xs text-muted">
              <Icon name="language" />
              <span>{t("accountLanguage")}</span>
            </div>
            <Select
              aria-label={t("accountLanguage")}
              value={language}
              options={[
                { value: "cs", label: "🇨🇿 Čeština" },
                { value: "en", label: "🇬🇧 English" },
              ]}
              onOpenChange={(open) => {
                selectingLanguage.current = open;
              }}
              onChange={(value) => {
                if (value === "cs" || value === "en") setLanguage(value);
              }}
            />
          </div>
          <div className="border-t border-border px-field py-control space-y-control">
            <Select
              label={t("appearanceTheme")}
              value={appearance.theme}
              disabled={appearance.busy}
              onOpenChange={(open) => {
                selectingLanguage.current = open;
              }}
              onChange={(v) => {
                if (v === "light" || v === "dark" || v === "system")
                  void appearance.change(v);
              }}
              options={[
                { value: "system", label: t("appearanceSystem") },
                { value: "light", label: t("appearanceLight") },
                { value: "dark", label: t("appearanceDark") },
              ]}
            />
            {appearance.error && (
              <p role="alert" className="text-sm text-danger">
                {t("appearanceFailed")}
              </p>
            )}
          </div>
          <div className="border-t border-border p-inline">{actions}</div>
        </div>
      )}
    </div>
  );
}
