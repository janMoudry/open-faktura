import { useCallback, useEffect, useRef, useState } from "react";
import { getSession, getSetup, type AuthUser } from "@open-faktura/api-client";
import { apiUrl } from "../services/status";

type Access =
  | { kind: "loading" }
  | { kind: "error" }
  | { kind: "ready"; setup: boolean; user: AuthUser | null };
export function useAuthSession() {
  const [access, setAccess] = useState<Access>({ kind: "loading" });
  const [attempt, setAttempt] = useState(0);
  const generation = useRef(0);
  const refresh = useCallback(() => setAttempt((value) => value + 1), []);
  const setUser = useCallback((user: AuthUser | null) => {
    generation.current++;
    setAccess({ kind: "ready", setup: false, user });
  }, []);
  const expire = useCallback(() => setUser(null), [setUser]);
  useEffect(() => {
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), 10000);
    const current = ++generation.current;
    Promise.all([
      getSetup(apiUrl, controller.signal),
      getSession(apiUrl, controller.signal),
    ])
      .then(([setup, user]) => {
        if (generation.current === current)
          setAccess({ kind: "ready", setup: setup.setup_required, user });
      })
      .catch(() => {
        if (generation.current === current) setAccess({ kind: "error" });
      })
      .finally(() => clearTimeout(timer));
    return () => {
      generation.current++;
      clearTimeout(timer);
      controller.abort();
    };
  }, [attempt]);
  const userId = access.kind === "ready" ? access.user?.id : undefined;
  useEffect(() => {
    if (!userId) return;
    const controller = new AbortController();
    const revalidate = () => {
      const current = generation.current;
      void getSession(
        apiUrl,
        AbortSignal.any([controller.signal, AbortSignal.timeout(10000)]),
      )
        .then((user) => {
          if (!controller.signal.aborted && current === generation.current)
            setUser(user);
        })
        .catch(() => {
          /* Temporary network failure must not unmount the workspace. */
        });
    };
    window.addEventListener("focus", revalidate);
    return () => {
      controller.abort();
      window.removeEventListener("focus", revalidate);
    };
  }, [userId, setUser]);
  return { access, refresh, setUser, expire };
}
