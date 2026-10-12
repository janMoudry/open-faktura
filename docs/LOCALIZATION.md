# Lokalizace — L10N-01

Implementováno 10. 10. 2026. Produktový vstup, setup, přihlášení, stav připojení,
pracovní prostor před založením firmy a obecné texty sdílených komponent mají
češtinu a angličtinu. Vývojové popisy a syntetické faktury sandboxu zůstávají česky.

## Použití

Přepínač „Čeština / English“ je v hlavičce i před přihlášením. Výchozí jazyk
je `cs`; `en` používá formáty `en-GB`, čeština `cs-CZ`. Volba se ukládá do
`localStorage` pod `open-faktura.language` a synchronizuje mezi kartami stejného
originu. Pokud úložiště není dostupné, přepnutí platí do obnovení stránky.
Neznámá uložená hodnota použije češtinu. Formulář se při přepnutí neremountuje.

Tauri sdílí frontend i katalogy, ale má vlastní úložiště originu. Uložení jazyka
do serverového profilu naváže na P03; nyní se mezi zařízeními nesynchronizuje.
Jazyk UI nemění jazyk dokladu, měnu, časové pásmo ani účetní pravidla.

## Přidání textu

- Produkt: `apps/web/src/locales/cs.json` a `en.json`, hook `useAppTranslation()`.
- Obecné UI: `packages/ui/src/locales/cs.json` a `en.json`, hook `useLocale()`.
  UI balíček neimportuje aplikaci; doménové popisky přicházejí přes props.
- `t` přijímá pouze klíče českého katalogu. Celé věty patří do jednoho klíče,
  parametry přes `{{name}}`, počet přes `{ count }` a varianty `_one`, `_few`,
  `_many`, `_other`. Katalogy mají stejnou sadu klíčů, i když angličtina některé
  české pluralizační kategorie nepoužívá.
- Překlady vykresluje React jako text, bez `dangerouslySetInnerHTML`. Nevkládat
  HTML ani skládané věty do API chyb. Ukládat chybový kód, překládat při renderu.
- `pnpm check:locales` kontroluje shodu klíčů, neprázdné hodnoty a interpolace.
  Je součástí `pnpm check`, tedy i CI. Typová kontrola zachytí neznámé klíče.

Jádro překladů je připnutý i18next 25.6.3, bez vzdáleného načítání. UI i aplikace
mají izolované instance se stejnou konfigurací a společným jazykovým kontextem;
pluralizaci a fallback zajišťuje knihovna. React Aria dostává stejné locale.
Zdroje ověřené 10. 10. 2026: [typované klíče](https://www.i18next.com/overview/typescript),
[pluralizace](https://www.i18next.com/translation-function/plurals).

## Formátování

`@open-faktura/ui` exportuje `formatNumber`, `formatMoney` a `formatCalendarDate`.
Částky předávat jako přesný desetinný řetězec; helper je nepřevádí přes `Number`.
Formát měny zaokrouhluje pouze zobrazení podle měny, nenahrazuje účetní výpočet.
ISO datum `YYYY-MM-DD` nemá časové pásmo a helper jej neposune při změně locale.
Pro časové okamžiky je nutné výslovně určit časové pásmo; pro budoucí číselné
editory specifikovat locale parser, nepoužívat `parseFloat` na lokalizovaný text.

## Ověření a hranice

`tests/e2e/localization.spec.ts`: oba jazyky setup/přihlášení s mockovaným API,
uchování formuláře a chyb při přepnutí, reload, bezpečný fallback, nedostupný
storage, klávesnicová nápověda, přístupnost, anglický kalendář, česká pluralizace,
přesná vysoká částka a kalendářní datum. Běží na Chromium desktop a mobilní emulaci.
Skutečné Rust/PostgreSQL přihlášení má samostatný regresní test `auth.spec.ts`.
Tato změna neověřuje nativní instalátor ani starší systémové WebView.
