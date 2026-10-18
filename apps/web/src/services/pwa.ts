/** Register only the production browser PWA; desktop and development keep normal networking. */
export function registerOnlinePwa(): void {
  if (
    !import.meta.env.PROD ||
    !("serviceWorker" in navigator) ||
    !window.isSecureContext ||
    "__TAURI_INTERNALS__" in window ||
    "__TAURI__" in window ||
    !["https:", "http:"].includes(window.location.protocol)
  )
    return;
  const register = () => {
    void navigator.serviceWorker
      .register("/pwa-sw.js", { scope: "/", updateViaCache: "none" })
      .catch(() => {
        // Installation is optional. Registration failure never blocks the online app.
      });
  };
  if (document.readyState === "complete") register();
  else window.addEventListener("load", register, { once: true });
}
