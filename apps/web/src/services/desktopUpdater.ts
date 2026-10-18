import { invoke, isTauri } from "@tauri-apps/api/core";
export interface DesktopUpdateStatus {
  enabled: boolean;
  current_version: string;
}
export interface DesktopCandidate {
  version: string;
  current_version: string;
  notes: string;
  token: string;
}
export const isDesktopClient = () => isTauri();
export async function desktopUpdateStatus(): Promise<DesktopUpdateStatus> {
  const v = await invoke<DesktopUpdateStatus>("desktop_updater_status");
  if (typeof v?.enabled !== "boolean" || typeof v.current_version !== "string")
    throw new Error("updater_failed");
  return v;
}
export async function desktopUpdateCheck(): Promise<DesktopCandidate | null> {
  const v = await invoke<DesktopCandidate | null>("desktop_updater_check");
  if (
    v !== null &&
    (!v ||
      [v.version, v.current_version, v.notes, v.token].some(
        (s) => typeof s !== "string",
      ))
  )
    throw new Error("updater_failed");
  return v;
}
export const desktopUpdateInstall = (v: DesktopCandidate, confirmed: boolean) =>
  invoke<void>("desktop_updater_install", {
    version: v.version,
    token: v.token,
    confirmed,
  });
