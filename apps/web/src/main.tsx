import { AppLanguageProvider } from "./locales";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { ProductRoutes } from "./routes/ProductRoutes";
import "./styles/app.css";
import { initializeTheme } from "./utils/theme";
initializeTheme();
createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <AppLanguageProvider>
      <ProductRoutes />
    </AppLanguageProvider>
  </StrictMode>,
);
