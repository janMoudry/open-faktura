---
name: react-development
description: Vyvíjí React UI Open Faktury s Tailwind tokeny, atomic designem a tradiční strukturou pages/components. Použij při implementaci obrazovek, formulářů a sdílených komponent.
---

# react-development

Přečti `docs/architecture/DEVELOPMENT_CONVENTIONS.md` a dotčenou specifikaci. Konkrétní nainstalované knihovny ověř v manifestech; doporučení průzkumu není instalovaná závislost.

Pages skládají templates/organisms a napojují hooks/services. Atoms a sdílené UI nevolají API. Dodrž tokeny z packages/design-tokens; nové role přidej centrálně, nevytvářej lokální barevné literály. U dynamických rozměrů lze použít vypočtenou hodnotu. Tailwind třídy sestavuj jako úplné staticky zjistitelné varianty.

Formulář zachovává serverové validační chyby, loading a rozpracovaná data. Peněžní řetězce neposílej přes JS Number pro účetní výpočet. Cache serverových dat odděluj firmou a identitou. Náhled výpočtu neprezentuj jako vystavený doklad.

Ověř klávesnici, labely, focus, chyby a mobilní layout; komponentu zařaď podle role, ne podle počtu řádků. U změny vzhledu zkontroluj relevantní stavy ve skutečně spuštěném UI. Neoznačuj chybějící běhové ověření jako dokončené.
