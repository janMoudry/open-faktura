---
name: brand-redesign
description: Převádí brand nebo vizuální reference do tokenů a sdílených komponent Open Faktury. Použij při rebrandingu, změně vizuálního jazyka nebo sjednocení identity webu a desktopu, nikoli pro běžnou opravu funkce.
---

# Redesign Open Faktury podle brandu

Pracuj od kořene repozitáře. Přečti `docs/architecture/DEVELOPMENT_CONVENTIONS.md`, `packages/ui/README.md`, aktuální `packages/design-tokens/tokens.css` a pro implementaci `.agents/skills/react-development/SKILL.md`. Katalog komponent běží na `/#/design-system`; není samostatnou kopií komponent.

Výchozí identita má monochromatické plochy a primární akce, Manrope + Lucide. Chyby a destruktivní akce jsou červené, upozornění jantarová a úspěšné stavy zelené. Zachovej tyto sémantické barvy, pokud zadání výslovně nemění i tento princip; monochromatický brand neznamená odstranění stavových barev. Nové zadání uživatele nebo dodaný brand má přednost; monochromatickou podobu nezaměňuj za trvalý zákaz barev. Pokud uživatel žádá jen skill nebo návrh, není to automaticky pokyn přestavět aplikaci.

## Převod brandu do produktu

- Z dodaného brandu či referencí odděl doložené požadavky (font, paleta, logo, pravidla) od vlastních návrhových rozhodnutí. Chybějící detail lze navrhnout; ptej se jen na rozhodnutí, které skutečně blokuje práci. Cizí aplikace je reference, nikoli identita Open Faktury.
- Nejprve uprav sémantické tokeny: plochy, text, primární akce, interaktivní okraje, focus, stavy, typografii a případně radius/rozměry. Neměň význam rolí podle konkrétní barvy. Success, warning a danger musí mít vlastní tokeny nezávislé na primary.
- Uprav sdílené komponenty v `packages/ui` a jejich varianty. Rozložení doménových obrazovek měň jen tam, kde je to součástí nového jazyka. Nevytvářej druhé sady tlačítek či stylů přímo v pages.
- Zachovej labely, focus, disabled/loading/error/readonly stavy a čitelnost částek. V monochromatické variantě rozlišuj stavy textem a ikonou. Neodstraňuj účetní informace kvůli estetice a neměň při redesignu doménové výpočty ani kontrakty.
- Fonty a ikony vybírej z existujících knihoven. Nové externí zdroje ověř u autora, eviduj licence a přibal fonty lokálně pro web i Tauri. Neměň název produktu, logo ani distribuční identifikátory bez opory v zadání.

## Dokončení

Propaguj změny do živého katalogu, seznamu faktur a editoru. Ověř skutečné zobrazení alespoň na desktopové a mobilní šířce, dlouhé popisky, částky, chyby, dialog a prázdný stav. Spusť relevantní existující kontroly: `pnpm build:web`, `pnpm format:check`, `node scripts/check-conventions.mjs` a UI scénáře `pnpm test:e2e`. Testy přístupnosti v `tests/e2e/design-system.spec.ts` nesmí být umlčeny kvůli brandu.

Aktualizuj přijatá rozhodnutí a příklady; zastaralý směr označ jako historický. Ulož snímek výsledku do `docs/design/` a uveď, co je implementováno a co pouze navrženo. Browserová emulace sama neprokazuje ověření nativního Tauri buildu. Publikace, release nebo externí odesílání nejsou součástí běžného redesignu.
