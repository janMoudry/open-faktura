---
name: faktura-domain
description: Navrhuje datové struktury, peněžní výpočty a životní cykly Open Faktury podle doložených kontraktů iDokladu. Použij při změně doménového modelu, migrací a účetních vazeb.
---

# Model domény

Pracuj od kořene repozitáře. Načti `docs/DATA_MODEL.md`, dotčenou specifikaci v `docs/specs/` a její důkazy v `docs/research/EVIDENCE.md`.

Pro každé pole rozliš externí kontrakt a náš interní návrh. Změnu popiš typem, povinností/null významem, vazbou, firemním rozsahem a invariantem. Číselníky SDK nemapuj podle jejich pořadí; ukládej explicitní mapování a řeš neznámé hodnoty.

U peněžních změn rozeber vstupní cenu, množství, slevy, DPH, kurz a fáze zaokrouhlení. Nezvolené pravidlo označ jako otevřenou otázku. Referenční očekávané částky musí pocházet z pozorování, ne z implementace testované funkce.

Odděl životní cyklus dokladu, úhrady, splatnost, odesílání a export. Zachovej snapshot historických údajů, vazby na zálohy a opravy a audit. Zkontroluj přístup přes firmu i u navázaných objektů a souběh číslování či párování.

Výstupem je upravený model/specifikace s migračním dopadem a scénáři. Dokud nejsou vybrány stack a databáze, nepředkládej konceptuální typy jako hotové migrace.
