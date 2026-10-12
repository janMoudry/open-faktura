import { AppLanguageProvider } from "./locales";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { InvoicesPage } from "./pages/InvoicesPage";
import "./styles/app.css";
// This entry is served by Vite dev only; it is not a production build input.
createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <AppLanguageProvider>
      <InvoicesPage />
    </AppLanguageProvider>
  </StrictMode>,
);
