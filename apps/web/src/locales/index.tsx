import {
  createContext,
  useContext,
  useEffect,
  useState,
  type ReactNode,
} from "react";
import { createInstance, type TOptions } from "i18next";
import { LocaleProvider, useLocale, type Language } from "@open-faktura/ui";
import cs from "./cs.json";
import en from "./en.json";
export type AppKey = keyof typeof cs;
export const languageStorageKey = "open-faktura.language";
const translations = createInstance();
void translations.init({
  resources: { cs: { translation: cs }, en: { translation: en } },
  lng: "cs",
  fallbackLng: "cs",
  initImmediate: false,
  interpolation: { escapeValue: false },
});
const supported = (value: string | null): Language =>
  value === "en" ? "en" : "cs";
function readLanguage(): Language {
  try {
    return supported(localStorage.getItem(languageStorageKey));
  } catch {
    return "cs";
  }
}
const SetLanguage = createContext<(language: Language) => void>(() => {});
export function AppLanguageProvider({ children }: { children: ReactNode }) {
  const [language, setLanguage] = useState(readLanguage);
  useEffect(() => {
    document.documentElement.lang = language;
  }, [language]);
  useEffect(() => {
    const sync = (event: StorageEvent) => {
      if (event.key === languageStorageKey || event.key === null)
        setLanguage(readLanguage());
    };
    window.addEventListener("storage", sync);
    return () => window.removeEventListener("storage", sync);
  }, []);
  const changeLanguage = (next: Language) => {
    setLanguage(next);
    try {
      localStorage.setItem(languageStorageKey, next);
    } catch {
      /* Session-only choice when storage is unavailable. */
    }
  };
  return (
    <SetLanguage.Provider value={changeLanguage}>
      <LocaleProvider language={language}>{children}</LocaleProvider>
    </SetLanguage.Provider>
  );
}
export function useAppTranslation() {
  const { language, locale } = useLocale();
  const setLanguage = useContext(SetLanguage);
  return {
    language,
    locale,
    setLanguage,
    t: (key: AppKey, options?: TOptions) =>
      String(translations.t(key, { ...options, lng: language })),
  };
}
