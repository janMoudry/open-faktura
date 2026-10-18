import { AppLanguageProvider } from "./locales";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { ProductRoutes } from "./routes/ProductRoutes";
import "./styles/app.css";
import { registerOnlinePwa } from "./services/pwa";
import { initializeTheme } from "./utils/theme";
initializeTheme();
registerOnlinePwa();
createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <AppLanguageProvider>
      <ProductRoutes />
    </AppLanguageProvider>
  </StrictMode>,
);
