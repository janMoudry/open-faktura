import { useEffect, useState } from "react";
import { loadStatus } from "../services/status";
export function useStatus() {
  const [attempt, setAttempt] = useState(0);
  const [state, setState] = useState<{
    loading: boolean;
    api: boolean;
    database: boolean;
    error?: boolean;
  }>({ loading: true, api: false, database: false });
  useEffect(() => {
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), 5000);
    let active = true;
    setState({ loading: true, api: false, database: false });
    loadStatus(controller.signal)
      .then((result) => {
        if (active) setState({ ...result, loading: false });
      })
      .catch(() => {
        if (active)
          setState({
            loading: false,
            api: false,
            database: false,
            error: true,
          });
      })
      .finally(() => clearTimeout(timeout));
    return () => {
      active = false;
      clearTimeout(timeout);
      controller.abort();
    };
  }, [attempt]);
  return { ...state, refresh: () => setAttempt((value) => value + 1) };
}
