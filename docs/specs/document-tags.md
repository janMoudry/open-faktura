# Štítky dokladů a kontaktů (P20-01a)

18. 10. 2026. Funkční potřeba: FEATURES §14 a P20-01. Následující životní
cyklus je návrh Open Faktury, nikoli doložená přesná shoda s iDokladem.

Štítek patří jedné firmě. Má neprůhledné ID, revizi a oříznutý název o délce
1–80 znaků. Název je v rámci firmy jedinečný bez ohledu na velikost písmen.
Členové jej mohou číst, vlastník/editor vytvářet, přejmenovávat, mazat a
přiřazovat. Instalační správce nesmí obejít členství.

Přiřazení podporuje návrhy i vystavené faktury, nabídky/šablony/zálohy,
přijaté faktury/účtenky a kontakty. Složené cizí klíče zaručují stejnou firmu
na štítku a cíli i při přímém SQL. Přiřazení existuje nejvýše jednou;
opakované přiřazení/odebrání je bezpečné. Nejde o změnu historického obsahu
dokladu, nevzniká nový finanční výpočet ani PDF. Přehled vrací počet použití.

Přejmenování a smazání vyžadují aktuální revizi. Smazání odstraní pouze štítek
a jeho přiřazení; doklady/kontakty se nezmění. Každý zápis má UUID request_id,
aktéra, vstup a výsledek v `document_tag_command`; replay stejného aktéra a
vstupu nezapisuje podruhé, změněný vstup/aktér je konflikt. Práva se ověřují
před replay, takže odebraný člen nemůže číst starou odpověď. Firemní zámek
serializuje změny se správou členství, odstraněním firmy a dalšími zápisy štítků.

REST a MCP sdílejí TagStore. OpenAPI `document-tags.openapi.json` a generované
MCP tools/TS mají stejné příkazy. Formulář firmy spravuje číselník, detaily
kontaktu, prodejního a nákupního dokladu obsahují přiřazení; CS/EN.
Všechny čtyři registry filtrují podle tag_id přímo v SQL před stránkováním,
stejnou podmínkou pro počet i řádky. Filtr je uložený v URL a změna vrací první
stránku. Export CSV/XLSX kontaktů a faktur zachovává stejný filtr. Cizí/neexistující
ID štítku vrátí prázdný výsledek, žádná data jiné firmy. Koš dokladů je oddělený
životní cyklus, nesmí se zaměňovat za odebrání štítku.

Migrace 202610180081 je aditivní, staré doklady nedostávají odhadnuté štítky.
`crates/server/tests/document_tags.rs` ověřuje role, firemní izolaci,
idempotenci, jedinečnost, revizní konflikt a zachování cíle po smazání štítku.

Regrese filtrů ověřuje více zdrojů, stránku 2 při page_size=1 a shodu celkového
počtu. UI scénáře CS/EN v product-administration.spec.ts ověřují přenos tag_id do
všech čtyř API, zachování URL při reloadu a návrat na první stránku.
