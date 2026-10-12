import { createContext, useContext, type ReactNode } from "react";
import { I18nProvider } from "react-aria-components";
import { createInstance, type TOptions } from "i18next";
import cs from "./cs.json";
import en from "./en.json";
export type Language = "cs" | "en";
export const locales = { cs: "cs-CZ", en: "en-GB" } as const;
const LanguageContext = createContext<Language>("cs");
const translations = createInstance();
void translations.init({
  resources: { cs: { translation: cs }, en: { translation: en } },
  lng: "cs",
  fallbackLng: "cs",
  initImmediate: false,
  interpolation: { escapeValue: false },
});
export function LocaleProvider({
  language,
  children,
}: {
  language: Language;
  children: ReactNode;
}) {
  return (
    <LanguageContext.Provider value={language}>
      <I18nProvider locale={locales[language]}>{children}</I18nProvider>
    </LanguageContext.Provider>
  );
}
export function translateUi(
  language: Language,
  key: keyof typeof cs,
  options?: TOptions,
) {
  return String(translations.t(key, { ...options, lng: language }));
}
export function useLocale() {
  const language = useContext(LanguageContext);
  return {
    language,
    locale: locales[language],
    t: (key: keyof typeof cs, options?: TOptions) =>
      translateUi(language, key, options),
  };
}
