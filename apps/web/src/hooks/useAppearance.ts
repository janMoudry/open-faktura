import { useEffect, useState } from "react";
import {
  getUserPreferences,
  setUserPreferences,
  type UserPreferences,
} from "@open-faktura/api-client";
import { apiUrl } from "../services/status";
import { applyTheme, type Theme } from "../utils/theme";
export function useAppearance(identity: string) {
  const [preferences, setPreferences] = useState<UserPreferences | null>(null),
    [busy, setBusy] = useState(false),
    [error, setError] = useState(false);
  useEffect(() => {
    const c = new AbortController();
    setPreferences(null);
    setError(false);
    getUserPreferences(apiUrl, c.signal)
      .then((p) => {
        if (!c.signal.aborted) {
          setPreferences(p);
          applyTheme(p.theme);
        }
      })
      .catch(() => {
        if (!c.signal.aborted) setError(true);
      });
    return () => c.abort();
  }, [identity]);
  async function change(theme: Theme) {
    if (!preferences || busy) return;
    setBusy(true);
    setError(false);
    try {
      const p = await setUserPreferences(apiUrl, { ...preferences, theme });
      setPreferences(p);
      applyTheme(p.theme);
    } catch {
      setError(true);
      try {
        const p = await getUserPreferences(apiUrl);
        setPreferences(p);
        applyTheme(p.theme);
      } catch {
        /* Keep current appearance when the connection is unavailable. */
      }
    } finally {
      setBusy(false);
    }
  }
  return {
    theme: preferences?.theme ?? "system",
    busy: busy || !preferences,
    error,
    change,
  };
}
