# Logo Open Faktura

Vektorová rekonstrukce loga ze schválených obrázkových návrhů, 17. 10. 2026.
Obsahuje otevřený rám a dvouřádkový slovní znak „open faktura“.
Jde o připravené grafické podklady; logo zatím není zapojené do aplikace.

## Soubory

Každá varianta má skutečné vektorové SVG a bezeztrátové WebP s průhledným pozadím.

| Název souboru bez přípony | Použití |
| --- | --- |
| `open-faktura` | Hlavní logo na světlém pozadí; rumělkový symbol a tmavý nápis. |
| `open-faktura-inverse` | Tmavé pozadí; rumělkový symbol a bílý nápis. |
| `open-faktura-mono` | Celé logo v tmavé barvě. |
| `open-faktura-white` | Celé logo v bílé barvě pro tmavé pozadí. |
| `open-faktura-symbol` | Samostatný rumělkový symbol. |
| `open-faktura-symbol-mono` | Samostatný tmavý symbol. |
| `open-faktura-symbol-white` | Samostatný bílý symbol. |

`preview.png` a `preview.svg` jsou přehledy variant na světlém a tmavém pozadí,
nikoli soubory pro vložení samotného loga.

## Technické parametry

- SVG logotypu: `viewBox="0 0 512 192"`; všechny znaky jsou v křivkách.
- WebP logotypu: 2048 × 768 px, lossless, RGBA.
- SVG symbolu: `viewBox="0 0 160 160"`.
- WebP symbolu: 512 × 512 px, lossless, RGBA.
- Rumělková: `#D84B30`; tmavá: `#17191C`; bílá: `#FFFFFF`.
- SVG neobsahuje vložený bitmapový obrázek, text závislý na fontu,
  skripty ani externí odkazy. Vhodné pro web, desktop i tisk.

Pro web upřednostnit SVG. Zachovat poměr stran a vestavěný ochranný prostor;
logo nedeformovat. Doporučená minimální zobrazovaná šířka celého loga je 144 px,
samostatného symbolu 20 px. Pro menší místa použít symbol.
Bílé varianty mají průhledné pozadí a vyžadují dostatečně tmavý podklad.

Slovní znak vychází z místního Manrope ExtraBold (800), balíček
`@fontsource-variable/manrope` 5.3.0. Přiložen je původní licenční text
`Manrope-OFL.txt`; výsledná SVG nepotřebují načítat žádné písmo.

Reprodukční zdroj je v `.skill-archive/logo-creator/2026-10-17-open-faktura/build.py`
v kořeni repozitáře. SVG jsou samostatně upravitelná ve vektorovém editoru.
