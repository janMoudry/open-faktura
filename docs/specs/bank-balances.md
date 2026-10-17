# P16-10 — bankovní počáteční stavy a saldo

Implementační pravidla 14. 10. 2026. Veřejný podklad E-047 dokládá nastavení
počátečního stavu k datu; níže uvedené transakční a časové detaily jsou naše.

Počáteční stav je zůstatek na začátku kalendářního dne. Starší pohyby zůstávají
v historii, do salda nevstoupí. Počátek může vlastník/editor opravit s povinným
důvodem a shodnou revizí účtu; příkaz `opening` má UUID, audit a ochranu proti
opakování. Archivovaný účet nelze změnit. Datum není v budoucnu, částka má
nejvýše dvě desetinná místa v rozsahu NUMERIC(14,2), záporný stav je dovolený.

`bank_balance` vrací počáteční konfiguraci, počátek období, příjmy, výdaje,
konečný stav a počet pohybů. Součty z přesných NUMERIC zahrnují všechny stránky,
neovlivňuje je párování. Období je oboustranně inkluzivní, nesmí předcházet
počátku. Příjmy/výdaje před from se přesunou do počátku období. Neuvedené from
znamená konfigurovaný počátek, neuvedené to znamená bez horního omezení.

Bez konfigurace jsou opening/closing null, nikoli nula. Příjmy/výdaje jsou
stále dostupné. Jde o evidované pohyby, ne potvrzení aktuálního zůstatku banky.
REST a MCP používají shodný port s profilovým oprávněním, viewer pouze čte.
Migrace 032 přidává nullable opening_on/opening_balance, historickým účtům
počátek nevymýšlí. Nasazení vyžaduje restart API (také kvůli připraveným SQL
plánům). Úhrady a jejich historické PDF se nemění.
