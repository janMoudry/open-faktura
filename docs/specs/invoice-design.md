# Seznam a editor faktur — interaktivní návrh

Datum: 10. 10. 2026. Stav: designový prototyp, nikoli implementace INV-001 až INV-009. Vlastní návrh Open Faktury podle E-008 až E-010 v evidenci výzkumu.

## Původní směr (nahrazen monochromatickým základem)

Aktualizace 10. 10. 2026: aktuální UI používá neutrální šedou škálu se zeleným success, jantarovým warning a červeným danger, Manrope včetně nadpisů a ikony stavů. Dřívější barevnost a patková typografie níže dokumentují původní návrh; aktuální autoritou jsou DEVELOPMENT_CONVENTIONS.md a živý katalog komponent.

Klidná pracovní plocha s teplým světlým pozadím, lesní zelenou, jemnými hranami a tabulárními číslicemi. Patkové nadpisy dávají produktu vlastní charakter, malé ovládací prvky a texty zůstávají bezpatkové. Brand, barevnost, rozložení a komponenty jsou vlastní. Reference používáme pro pořadí informací a pracovní postupy.

- Z iDokladu: filtry zaplacení a prodlení, hledání, přehled dokladů a otevření detailu. Úhrada a prodlení jsou samostatné informace; částečně zaplacený doklad může být po splatnosti.
- Z FakturaOnline: srozumitelné sekce formuláře, přidávání položek, poznámka a jasná závěrečná akce.
- Vlastní rozhodnutí: zákazník před technickými údaji, sticky souhrn vedle editoru, tři souhrnné částky nad seznamem, mobilní seznam jako karty, vlastní vzhled místo repliky značek.

## Rozsah a akceptace

| ID | Prototyp | Ověření |
| --- | --- | --- |
| UX-001 | Seznam hledá bez diakritiky, kombinuje stav, období a řazení; prázdný výsledek má reset | Hledání `atelier` + prodlení vrátí částečně uhrazený Ateliér Forma |
| UX-002 | Editor zákazníka, adresy, dat, položek a poznámky, průběžný souhrn a náhled | Změna množství aktualizuje součet; přidání/odebrání položky funguje |
| UX-003 | Ukázkový koncept lze uložit a znovu otevřít | Koncept je v seznamu a zachová zadané hodnoty |
| UX-004 | Mobil bez horizontálního přetékání stránky; dostupné labely, focus a textové stavy | Desktop a mobil, seznam i editor |

## Omezení návrhu

Všechna jména a částky jsou syntetické. Referenční datum je pevně 10. 10. 2026. Editor nabízí jen CZK a neplátce DPH. Neřeší skutečné IČO, bankovní údaje, daňové režimy, slevy, kurzy, číslování, platby, PDF ani odeslání. Náhled je označen jako neúčetní dokument. Uložené koncepty existují pouze v paměti této otevřené aplikace; reload je zruší. Editor chrání rozepsaná pole během zobrazení; nová faktura zakládá nový koncept. Vystavené demo řádky otevírají pouze souhrnný detail, nepředstírají editaci historie.

Výpočty slouží pouze demonstraci interakce: `decimal.js`, řetězcové vstupy do 9 celých a 2 desetinných míst, kladné množství, nezáporná cena; součin zaokrouhlený na 2 místa HALF_UP po řádku a součet řádků. Bez daní, slev, kurzů a zaokrouhlení celého dokladu. Jde o naše explicitní demo pravidlo, nikoli ověřený algoritmus iDokladu. Finální výpočty musí přijít ze společného Rust aplikačního jádra po uzavření Q-01. Žádná migrace ani změna produkčního datového modelu.

Další návrhová iterace: rozšířené filtry, hromadné akce, plátce DPH, adresář a bankovní účet, historie dokladu. Tyto nehotové ovládací prvky zatím nejsou v UI.

## Ověření a náhledy

`pnpm build:web`, `pnpm typecheck`, `pnpm format:check` a `node scripts/check-conventions.mjs` úspěšné. `pnpm test:e2e`: 10 úspěšných testů (Chromium desktop a mobilní emulace), včetně původních kontrol spojení na API/PostgreSQL. UI scénáře v `tests/e2e/invoices.spec.ts` ověřují kombinované hledání, prodlení s částečnou platbou, prázdný výsledek, desetinný přepočet, neplatný vstup, přidání/odebrání položky, náhled, uložení a znovuotevření konceptu i šířku stránky.

Vizuální kontrola v prohlížeči: desktop 1440 px a mobil 390 px. Tato iterace nebyla samostatně ověřena v nativním Tauri buildu. Snímky: [seznam](../design/invoice-list.jpg), [editor](../design/invoice-editor.jpg).

## Zdroje

- [FakturaOnline — veřejný editor](https://www.fakturaonline.cz/faktura), přímé pozorování nepřihlášeného formuláře.
- [iDoklad — vydané faktury](https://www.idoklad.cz/podpora/faktury-vydane), veřejná dokumentace.
- [iDoklad — karta faktury](https://www.idoklad.cz/podpora/karta-faktury), veřejná dokumentace.
- [iDoklad — snímek filtrů](https://www.idoklad.cz/wp-content/uploads/2022/12/fv-filter.png), ilustrační snímek z nápovědy, cesta obsahuje 2022/12; není dokladem aktuální podoby přihlášeného UI.
