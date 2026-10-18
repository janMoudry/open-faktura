# Revize exportů a explicitní zámky obsahu (P20-02a)

18. 10. 2026. Potřeba vychází z FEATURES a P20-02. JSON formát a politika níže
jsou návrhem Open Faktury, nikoli dokladem shody s iDokladem či účetním systémem.

## Export revize

Člen firmy může exportovat 1–100 explicitně vybraných faktur (včetně návrhů),
prodejních dokladů nebo přijatých faktur/účtenek. Požadavek obsahuje druh, ID
a aktuální revizi každého zdroje, UUID request_id a explicitní lock boolean.
Doklady jiné firmy nejsou dostupné. Opakované ID je validace, změněná revize
konflikt. Bez lock je dostupný také čtenáři, s lock jen vlastníkovi/editorovi.

Vlastní JSON `open-faktura-documents`, verze 1, obsahuje původní snapshot zdroje
bez binární PDF cache a vstupu initial_data. Desetinné hodnoty nejsou převáděné
přes float. Není to nativní formát účetního systému ani daňové podání. Výstup
má limit 20 MiB; žádné částečné výsledky. Server ukládá přesný obsah, SHA-256,
aktéra, datum a vybrané revize v neměnném document_export. Soubor lze znovu
stáhnout bez přepočtu; pozdější oprava zdroje starý export nezmění.

Export vyžaduje aktuální firemní členství, zamyká zdrojové řádky a serializuje
export/odemknutí firemním advisory lockem. Uložení snapshotů, auditu a případných
zámků je atomické. Stejný aktér+UUID+vstup přehraje původní soubor bez dalšího
exportu. Změněný vstup/aktér klíče je konflikt.

## Co zámek znamená

`lock=true` brání změně obsahu, částek, měny, číslování a stornu zdroje i přes
REST/MCP či jiné sdílené příkazy. PostgreSQL trigger ověřuje skutečnou změnu
obsahových sloupců a vrací pojmenovanou chybu document_export_locked, kterou
adaptéry převádějí na revizní konflikt. Zámek neblokuje úhrady, jejich opravy,
saldo, advance_paid, technické revize, propojení dokumentu nebo PDF cache.
Vystavené faktury nadále chrání také nezávislá původní neměnnost.

Odemknout smí pouze aktuální vlastník. Požadavek obsahuje ID konkrétního exportu
a neprázdný důvod do 500 znaků; důvod a aktér zůstávají v neměnném auditu.
Původní export se nemaže. Replay stejného odemknutí je bezpečný. Odemknutí
nedává povolení měnit vystavenou fakturu a neobchází rezervace záloh/skladu.

Existující čtení CSV/XLSX/ISDOC ani stažení PDF nic automaticky nezamyká.
Uživatel musí zvolit explicitní akci exportu se zámkem; nejde o tichou změnu
stávajících read-only nástrojů. Historie detailu zobrazuje posledních 100 exportů;
úplná historie zůstává v DB a firemním archivu.

## Rozhraní a testy

`document-exports.openapi.json`, generované TS a MCP tools, společný ExportStore.
UI CS/EN umožňuje nový JSON export, potvrzení zámku, historii/stažení původní
verze a owner dialog odemknutí s důvodem. Migrační 202610180083 je aditivní,
před upgradem zastavit staré API kvůli změně chování zápisů.

`document_exports.rs` ověřuje izolaci, role, request replay, neměnný soubor,
revizní konflikt, blokování editace, owner odemknutí s důvodem a pokračující
aktualizaci evidovaných plateb/záloh. Žádné testy nepoužívají reálné zákazníky.
