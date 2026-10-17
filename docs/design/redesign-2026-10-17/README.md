# UI-REDESIGN-01 — vlastní identita Open Faktury

17. 10. 2026. Vlastní návrhové rozhodnutí uživatele, nikoli požadavek na kopii iDokladu.
              Schválené byly světlé produktové návrhy s dvouřádkovým logem, neutrálními plochami,
              bílými panely a rumělkovými akcemi. Dřívější výrazně patkový papírový návrh je historický.

## Implementovaný rozsah

- Sdílené sémantické tokeny, umírněné zaoblení, Manrope, Lucide, světlý/tmavý režim.
- Primární a destruktivní akce mají vlastní token popředí; barvy stavů zůstávají oddělené.
- Lokální logo v křivkách pro světlý i tmavý podklad; žádné externí fontové či obrazové I/O.
- Shell bez vnějšího černého rámu, logo a přepínání profilu vlevo, skládací navigace,
  aktivní položka, drobečková navigace a účet dole. Navigační stav zůstává zachován při změně stránky.
- Přehled: pohledávky a úhrady z existujícího overview, prodej/nákup a měsíční graf
  z period-report, poslední faktury, upozornění a počty dokladů. Čtení respektuje
  profil a stávající přístupová práva.
- Faktury: souhrn skutečných pohledávek/úhrad, filtry, hledání, export, přeuspořádané
  sloupce, textové stavy, kopírování a PDF. Koncept nadále nemá konečné číslo.
- Editor: údaje dokladu a položky vlevo, platba/nastavení vpravo na široké obrazovce,
  na menší šířce průchozí formulář. Zřetelná daňová sazba v řádku; výběr z ceníku
  se otevře explicitní akcí. Objednávka/poznámka jsou rozbalitelné. Uložení a vystavení
  jsou v horní liště, pomocí nativní vazby tlačítka na formulář.
- Vystavení stále vyžaduje uložený nezměněný návrh a samostatné potvrzení.
  Neodesílá e-mail. Změny výpočtů, backendu, migrací ani veřejných kontraktů tato dodávka nedělá.

## Odchylky od obrázkového konceptu

Předloha je návrh směru, nikoli doménová specifikace. Implementace proto záměrně používá
jen existující a doložená data:

- Přehled prodeje/nákupu je od začátku roku po kalendářní datum profilu, po měsících,
  nikoli vymyšlený denní peněžní tok. Hodnota dokladů neznamená zisk ani peněžní příjem.
- Součty pohledávek a úhrad pokrývají celou historii; jsou tak i označené.
- Bez nového API nevytváříme globální časovou osu aktivity nebo součet přijatých závazků.
  Místo ukázkové aktivity je přehled počtů dokladů.
- Výpadek reportu není nula; ostatní úspěšně načtené části lze používat.
- Zůstávají všechny současné moduly a funkční skupiny navigace, ochrana neuložených změn,
  oprávnění, lokalizace a dostupné akce. Primární barva je proti obrázku mírně tmavší
  kvůli kontrastu bílého textu.
- Starší Tauri distribuční ikona ani PDF značkování se touto dodávkou nemění.

## Ověření

Výsledek 17. 10. 2026: produkční build, formátování, kontrola tokenů/lokalizace
a všech 34 vybraných prohlížečových scénářů prošly.

Automatizované scénáře běží nad syntetickým API zachyceným na úrovni prohlížeče.
Všechny požadavky jsou interceptované; žádné skutečné účetní údaje ani odeslání.

- `pnpm build:web` — TypeScript a produkční Vite build, včetně zákazu úniku sandboxu.
- `pnpm format:check`, `node scripts/check-conventions.mjs`, `node scripts/check-locales.mjs`.
- `E2E_WEB_PORT=5174 pnpm exec playwright test tests/e2e/design-system.spec.ts tests/e2e/design-audit.spec.ts tests/e2e/routing.spec.ts tests/e2e/redesign.spec.ts --workers=2`.
- Katalog: validační vazby, loading/disabled, dialog a focus, taby, grafová data,
  zmenšený pohyb, kalendář; přístupnost axe WCAG 2 A/AA a 2.1 AA.
- Produkt: Přehled, faktury a editor CS/EN, desktop/mobilní šířka, tmavý režim,
  viewer, výpadek reportu, nativní submit horním tlačítkem, zachování vstupů při chybě uložení.
- Navigace: zachování shellu, rozbalených skupin, identity a počtu načítání profilu.

UI scénáře s vlastní databází/API z `invoices.spec.ts` se v této izolované dodávce
nespouštějí: sdílený backend a jeho rozpracovaná implementace patří souběžné práci.
Tyto kontroly neprokazují kompletní účetní regresi ani ověření nativního Tauri buildu.
Před společným začleněním je nutná regrese nad aktuálním backendem.

Snímky `*-desktop.png` a `*-mobile.png` jsou skutečný výstup implementovaného UI
se syntetickými fixtures, nikoli generované obrázky. Referenční `reference-*.png`
jsou schválené generované návrhy. Reprodukce snímků:

```sh
E2E_WEB_PORT=5174 DESIGN_CAPTURE=1 pnpm exec playwright test tests/e2e/redesign.spec.ts --workers=2
```

Alternativní port odděluje náhled a testy od současně běžícího hlavního frontendového serveru.
UI je připravené v oddělené pracovní kopii; pracovní rozpracování jiného agenta se nepřepisuje.
