# EXPORT-01 / P10-04 — CSV a XLSX export seznamů

Rozsah: faktury a odběratelé aktivního profilu. Reference E-031 potvrzuje formáty a výběr sloupců; konkrétní sloupce, typy buněk, limity a bezpečnostní pravidla níže jsou vlastní návrh Open Faktury, nikoli ověřené schéma souboru iDokladu.

## Chování

- Export z hlavičky seznamu otevře dialog s formátem XLSX/CSV a výběrem sloupců. Výchozí jsou všechny dostupné sloupce. Aspoň jeden musí zůstat vybraný.
- Přenáší aplikované hledání, stav, odběratele, datumy, částky a řazení faktur; u kontaktů hledání a archiv. Rozpracovaný text hledání bez potvrzení není aktivní filtr.
- Zahrnuje všechny odpovídající řádky, nikoli pouze aktuální stránku. Nejvýše 10 000; větší výběr vrátí 422 s polem `export_limit`, žádný částečný soubor. Prázdný výsledek obsahuje záhlaví.
- Viewer má stejné právo exportovat data, která smí číst. Instalátor neobchází členství; smazaný/cizí profil je nedostupný.
- Používá uložené hodnoty a součty, nic nepřepočítává. Neúplný návrh má chybějící hodnoty prázdné. Stav dokladu rozlišuje návrh/vystavení; uhrazené částky a zůstatek mají vlastní sloupce.
- Výběr sloupců zachovává pořadí požadavku. Neznámé a duplicitní sloupce se odmítají. Interní ID nejsou výstupní sloupec.

## Soubory a konzistence

CSV: UTF-8 BOM, středník, CRLF, standardní uvozování. Potenciální vzorce (=, +, -, @ po úvodních mezerách nebo úvodní tab/CR/LF) mají bezpečnostní apostrof. Ten je záměrnou změnou exportovaného textu, nikoli původních dat.

XLSX: skutečný OOXML ZIP sešitu, všechny buňky explicitně textové, bez vzorců, maker, hyperlinků a externích vztahů. To zachovává dlouhá čísla, desetinnou přesnost a úvodní nuly identifikátorů. Částky proto před tabulkovými výpočty vyžadují převod na čísla. Sloupce jsou pro CS/EN přeložené; CS datum DD.MM.RRRR a desetinná čárka, EN DD/MM/YYYY a desetinná tečka. Měna je samostatný sloupec, text zákazníka se nepřekládá.

Načtení používá stejnou validaci, SQL filtry a autorizaci jako seznam. Count a všechny řádky se čtou v jedné repeatable-read transakci. Kódování probíhá až po uvolnění DB spojení přes `spawn_blocking`. Export nic nepersistuje, nemění revize ani audit; nevyžaduje migraci. Nejde o účetní export, zálohu ani úplný migrační formát (bez položek, platebních událostí a příloh).

## Kontrakty a akceptace

POST `/api/companies/{company_id}/exports`, session + kontrola originu; JSON `{format, language, columns, selection:{kind, search}}`. Validovaná stránka/velikost ze search nemění rozsah exportu. Limit těla 16 KiB. Odpověď attachment, správné MIME, no-store a nosniff. `contracts/export.openapi.json` je autorita klienta i generovaného MCP schématu.

MCP `register_export` volá stejný příkaz, vrací filename/media_type/base64; pouze čtení. UI ruší požadavek při odchodu z profilu; chybu zobrazí s možností opakování.

Testy: `crates/infrastructure/src/export.rs` (nezávislé načtení přes calamine, CSV round-trip, formule a úvodní nuly), `crates/server/tests/export.rs` (izolovaná DB, filtry, stránkování, limity, role, REST/MCP), `tests/e2e/contacts.spec.ts` a `invoices.spec.ts` (CS/EN, desktop/mobil, dialog a stažení obou formátů). Běhové výsledky uvádí [validační záznam](../design/register-export-validation.md); není tvrzena kontrola v nativním Microsoft Excelu.
