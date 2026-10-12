import { createContext, useContext, useEffect, useRef, useState } from "react";
import { useBlocker, useLocation, useNavigate } from "react-router";
import {
  CompanyFailure,
  createCompany,
  getCompany,
  listCompanies,
  updateCompany,
  deleteCompany,
  type Company,
  type CompanySettings,
} from "@open-faktura/api-client";
import { useAppTranslation, type AppKey } from "../locales";
import { apiUrl } from "../services/status";

const defaults = (): CompanySettings => ({
  name: "",
  entity_type: "individual",
  registration_number: "",
  vat_number: "",
  street: "",
  city: "",
  postal_code: "",
  country: "CZ",
  email: "",
  phone: "",
  tax_mode: "non_payer",
  tax_effective_from: "",
  currency: "CZK",
  timezone: "Europe/Prague",
  maturity_days: 14,
  payment_method: "cash",
  iban: "",
  number_prefix: "FV",
  number_year: new Date().getFullYear(),
  next_number: 1,
});
export function useCompanyWorkspace(userId: string, onExpired: () => void) {
  const route = useLocation().pathname;
  const navigate = useNavigate();
  const bypassNavigation = useRef(false);
  const { t } = useAppTranslation();
  const [pageDirty, setPageDirty] = useState(false);
  const [companies, setCompanies] = useState<Company[]>([]);
  const [activeProfile, setActiveProfile] = useState<string>();
  const [company, setCompany] = useState<Company | null>(null);
  const [settings, setSettings] = useState<CompanySettings>(defaults);
  const [loading, setLoading] = useState(true);
  const [loadedRoute, setLoadedRoute] = useState<string>();
  const [deleteTarget, setDeleteTarget] = useState<Company | null>(null);
  const [deleteName, setDeleteName] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<AppKey | "">("");
  const [fields, setFields] = useState<string[]>([]);
  const [editing, setEditing] = useState(false);
  const [panel, setPanel] = useState<
    "identity" | "billing" | "advanced" | "branding"
  >("identity");
  const [saved, setSaved] = useState(false);
  const [attempt, setAttempt] = useState(0);
  const pending = useRef<{
    request_id: string;
    settings: CompanySettings;
  } | null>(null);
  const selectionKey = `open-faktura.active-profile.${userId}`;
  const requestKey = `open-faktura.company-create.${userId}`;
  const report = (e: unknown) => {
    setSaved(false);
    if (e instanceof CompanyFailure) {
      if (e.status === 401) {
        onExpired();
        return;
      }
      setFields(e.fields);
      const first = e.fields[0];
      if (first)
        setPanel(
          [
            "tax_mode",
            "tax_effective_from",
            "vat_number",
            "payment_method",
            "maturity_days",
            "iban",
          ].includes(first)
            ? "billing"
            : [
                  "country",
                  "currency",
                  "timezone",
                  "number_prefix",
                  "number_year",
                  "next_number",
                ].includes(first)
              ? "advanced"
              : "identity",
        );
      setError(
        e.status === 409
          ? "companyConflict"
          : e.status === 404
            ? "companyNotFound"
            : e.status === 403
              ? "companyForbidden"
              : e.status === 422
                ? "companyInvalid"
                : "requestFailed",
      );
    } else setError("requestFailed");
  };
  useEffect(() => {
    const controller = new AbortController();
    let active = true;
    bypassNavigation.current = false;
    setLoading(true);
    setCompany(null);
    setError("");
    setFields([]);
    setSaved(false);
    setEditing(false);
    setPanel("identity");
    const selected = route.match(/^\/companies\/([^/]+)(?:\/|$)/)?.[1];
    Promise.all([
      listCompanies(apiUrl, controller.signal),
      selected
        ? getCompany(apiUrl, selected, controller.signal)
        : Promise.resolve(null),
    ])
      .then(([list, selected]) => {
        if (active) {
          if (!selected && (route === "/" || route === "/workspace")) {
            let remembered: string | null = null;
            try {
              remembered = localStorage.getItem(selectionKey);
            } catch {
              /* Storage optional. */
            }
            selected =
              list.find((profile) => profile.id === remembered) ??
              (list.length === 1 ? list[0] : null);
          }
          let previous: string | null = null;
          try {
            previous = localStorage.getItem(selectionKey);
          } catch {
            /* Storage optional. */
          }
          setActiveProfile(
            selected?.id ??
              list.find((p) => p.id === previous)?.id ??
              (list.length === 1 ? list[0].id : undefined),
          );
          setCompanies(list);
          setCompany(selected);
          if (selected) {
            try {
              localStorage.setItem(selectionKey, selected.id);
            } catch {
              /* Storage optional. */
            }
            setSettings(selected.settings);
          }
        }
      })
      .catch((e) => {
        if (active) report(e);
      })
      .finally(() => {
        if (active) {
          setLoading(false);
          setLoadedRoute(route);
        }
      });
    return () => {
      active = false;
      controller.abort();
    };
    // Identity/route changes discard previous company data before the next load.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [route, userId, attempt]);
  async function start() {
    setBusy(true);
    setError("");
    try {
      if (!pending.current) {
        try {
          const raw = sessionStorage.getItem(requestKey);
          if (raw) pending.current = JSON.parse(raw);
        } catch {
          /* Private mode: retain retry key in memory. */
        }
        pending.current ??= {
          request_id: crypto.randomUUID(),
          settings: defaults(),
        };
        try {
          sessionStorage.setItem(requestKey, JSON.stringify(pending.current));
        } catch {
          /* Session-only fallback. */
        }
      }
      const next = await createCompany(apiUrl, pending.current);
      pending.current = null;
      try {
        sessionStorage.removeItem(requestKey);
      } catch {
        /* Storage optional. */
      }
      bypassNavigation.current = true;
      void navigate(`/companies/${next.id}`);
    } catch (e) {
      report(e);
    } finally {
      setBusy(false);
    }
  }
  async function removeProfile() {
    if (
      !deleteTarget ||
      deleteName !== (deleteTarget.settings.name || t("untitledCompany"))
    )
      return;
    setBusy(true);
    setError("");
    try {
      await deleteCompany(apiUrl, deleteTarget.id, deleteTarget.revision);
      try {
        if (localStorage.getItem(selectionKey) === deleteTarget.id)
          localStorage.removeItem(selectionKey);
      } catch {
        /* Optional storage. */
      }
      setDeleteTarget(null);
      setDeleteName("");
      if (route === "/companies") setAttempt((v) => v + 1);
      else {
        bypassNavigation.current = true;
        void navigate("/companies");
      }
    } catch (e) {
      report(e);
    } finally {
      setBusy(false);
    }
  }
  function confirmDelete(target: Company) {
    setError("");
    setDeleteName("");
    setDeleteTarget(target);
  }
  async function save(complete = false) {
    if (!company) return;
    setBusy(true);
    setError("");
    setFields([]);
    setSaved(false);
    try {
      const next = await updateCompany(apiUrl, company.id, {
        revision: company.revision,
        step: 0,
        complete,
        settings,
      });
      setCompany(next);
      setSettings(next.settings);
      setSaved(true);
      setCompanies((v) => v.map((c) => (c.id === next.id ? next : c)));
      if (complete) setEditing(false);
    } catch (e) {
      report(e);
    } finally {
      setBusy(false);
    }
  }

  const dirty =
    pageDirty ||
    (company?.role === "owner" &&
      JSON.stringify(settings) !== JSON.stringify(company.settings));
  const blocker = useBlocker(
    ({ currentLocation, nextLocation }) =>
      !!dirty &&
      !bypassNavigation.current &&
      currentLocation.pathname !== nextLocation.pathname,
  );
  useEffect(() => {
    if (!dirty) return;
    const warn = (event: BeforeUnloadEvent) => {
      event.preventDefault();
      event.returnValue = "";
    };
    window.addEventListener("beforeunload", warn);
    return () => window.removeEventListener("beforeunload", warn);
  }, [dirty]);
  return {
    navigateAfterSave: (path: string) => {
      bypassNavigation.current = true;
      void navigate(path, { replace: true });
    },
    onExpired,
    setPageDirty,
    companies,
    activeProfile,
    company,
    settings,
    setSettings,
    loading: loading || loadedRoute !== route,
    deleteTarget,
    setDeleteTarget,
    deleteName,
    setDeleteName,
    busy,
    error,
    fields,
    editing,
    setEditing,
    panel,
    setPanel,
    saved,
    setSaved,
    setAttempt,
    start,
    removeProfile,
    confirmDelete,
    save,
    blocker,
  };
}
export const CompanyWorkspaceContext = createContext<ReturnType<
  typeof useCompanyWorkspace
> | null>(null);
export function useWorkspace() {
  const value = useContext(CompanyWorkspaceContext);
  if (!value) throw new Error("CompanyWorkspaceContext missing");
  return value;
}
