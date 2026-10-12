# Audit design systému · 10. 10. 2026

Rozsah: všechny exporty packages/ui, CSS, design-tokens a jejich katalog;
regresně seznam faktur a editor. Jde o kontrolu současné sady, nikoli příslib
kompletního rozsahu Ant Designu. Opravy jsou součástí pracovního stromu.

## Opravené nálezy

| Místo | Before | After | Důvod |
| --- | --- | --- | --- |
| packages/ui/src/components/atoms/Button.tsx:48 | IconButton při loading zobrazoval původní ikonu i spinner | Jediný spinner, zachovaný přístupný název a disabled | Ikona nepřetéká z pevné šířky |
| packages/ui/src/components/atoms/Checkbox.tsx:4 | Pouze checked/unchecked | Indeterminate se skutečným DOM stavem a aria-checked=mixed | Hromadný výběr umí označit částečnou shodu |
| packages/ui/src/components/molecules/FilterTabs.tsx:59 | Chyběla zakázaná volba | disabled, styling a nativní vyloučení z interakce | Nedostupný filtr nelze aktivovat |
| packages/ui/src/components/molecules/FilterTabs.tsx:36 | ResizeObserver zapisoval i totožnou geometrii | Aktualizace jen při změně rozměrů | Zbytečné rendery |
| packages/ui/src/components/molecules/Dialog.tsx | Lokální velikost nadpisu, žádná vstupní odezva | Sdílená typografická třída, jemný vstup při otevření ukazatelem | Jednotná hierarchie a čitelná změna kontextu |
| packages/ui/styles.css | Press měnil transform skokově | Přerušitelná odezva 120 ms, vypnutí pro focus-visible a reduced-motion | Plynulost bez zdržování klávesnice |
| packages/ui/styles.css | Pevné kalendářní buňky přetékaly v úzkém popupu | Šířka podle dostupného viewportu, zachovaná výška 44 px | Použití při 320 px |
| packages/ui/styles.css | Hover i na zakázaném/selected kalendářním dni | Hover pouze pro dostupné nevybrané dny na zařízení s hover | Žádná falešná interaktivita |
| packages/ui/styles.css | Focus Radio/Checkbox spoléhal na webovou aplikaci | Sdílená třída v UI balíčku | Použitelné také v jiném konzumentovi |
| packages/ui/src/components/molecules/Selection.tsx | Chybové zprávy bez explicitního oznámení; dlouhé volby bez lámání | Alert, zalomení dlouhých textů, český výchozí placeholder | Čitelnost a zpětná vazba |
| packages/ui/src/components/organisms/DataChart.tsx:118 | Data nebo prázdný stav | Loading, chyba, retry; při chybě/načítání žádné staré hodnoty ani tabulka | Bez zavádějícího zobrazení |
| packages/ui/src/components/organisms/DataChart.tsx | Sloupcové legendy napodobovaly čáry; tooltip měl výchozí animaci | Legenda podle typu grafu, tooltip bez animace | Konzistence a reduced-motion |
| apps/web/src/pages/DesignSystemPage.tsx:100 | Implicitní minimum grid sloupce přetékalo při 320 px | Explicitní minmax sloupec a min-w-0 | Opravené přetékání celé stránky |

Panel, control a popover mají explicitní radius tokeny. Nadpisy zůstávají podle
Heading, dialog používá stejné CSS role se sémantikou Radix Title.
Rozestupy geometrie atomů mohou používat standardní Tailwind škálu; rozložení
obrazovek používá pojmenované role. Nejde o zákaz všech číselných utilit.

## Matice komponent

| Komponenty | Zkontrolované stavy / kontrakt |
| --- | --- |
| Button, IconButton | varianty, hover, press, focus, disabled, loading, ikonový název |
| Input, Textarea, Field, TextareaField | běžný, hint, required, error, disabled, readonly, vazba label/error |
| Checkbox, Radio, Switch | checked, unchecked, disabled; Checkbox mixed; Switch také disabled+checked |
| Select / SelectField | placeholder, výběr, disabled položka i pole, error, klávesnice a návrat focusu |
| Autocomplete | filtrování, volný text, výběr, loading, prázdné výsledky, error, disabled, Tab/Escape |
| DatePicker | kalendář, editace segmentů, min/max, disabled, readonly, error, úzký viewport |
| FilterTabs | aktivní, neaktivní, disabled, změna šířky, zalomení, klávesnice, reduced-motion |
| Dialog | title/description, focus trap, Escape, návrat focusu, scroll containment, motion |
| Alert, Badge, EmptyState | neutrální a sémantické tóny; význam kromě barvy; volitelná akce |
| Card, Heading, Separator | hierarchie, tokeny, dlouhé nadpisy, sémantické oddělení |
| Icon, Spinner | dekorativní ikony, status spinneru, reduced-motion |
| Table a části | nativní struktura, scope, pojmenování, lokální horizontální posuv |
| DataChart | line/bar, nula/null/záporné hodnoty, prázdný, loading/error/retry, tabulka, klávesnice |

Animace nejsou automaticky přidány všem komponentám. Select/Autocomplete/
kalendář zůstávají okamžité kvůli opakovanému zadávání a focusu. Alerty ani
tabulkové řádky neskáčou při změně dat. Dekorativní pohyb má reduced-motion
variantu; změna barev zachovává význam stavu.

## Ověření a omezení

- pnpm build:web, pnpm format:check, node scripts/check-conventions.mjs.
- Playwright Chromium: 38/38 scénářů prošlo (desktop a mobilní emulace), axe kontroly katalogu,
  faktur, editoru a otevřených výběrů; regresní scénáře v design-system.spec.ts,
  selection.spec.ts a design-audit.spec.ts. Nové testy pokrývají mixed/disabled/
  loading, obnovu grafu, celkovou šířku při 320 px, readonly a reduced-motion.
- Autocomplete navíc prošel 16 opakováními. Test před otevřením nabídky
  synchronizuje dokončení vykreslení po scrollu; React Aria při scrollu rodiče
  nemodální nabídku záměrně zavírá.
- Vizuální kontrola běžící aplikace v in-app browseru.
- Známá výjimka aria-hidden-focus je omezena na otevřený ComboBox, viz UI README;
  po zavření se znovu kontrolují všechna pravidla. Nejde o úplný audit se čtečkou.
- Nativní Tauri, fyzický mobil a další browser enginy nebyly v tomto auditu
  ověřeny. Produkční build hlásí větší vstupní JS chunk; další dělení podle
  obrazovek zůstává výkonovou optimalizací, ne dokončenou vlastností.

Zdroje metodiky:
[Web Interface Guidelines](https://raw.githubusercontent.com/vercel-labs/web-interface-guidelines/main/command.md)
a lokální skill emil-design-eng. Výsledky se vztahují k výše uvedenému rozsahu.
