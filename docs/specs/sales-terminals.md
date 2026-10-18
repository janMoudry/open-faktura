# P16-09 — Prodejní terminály

Stav: P16-09a konfigurace implementovaná 15. 10. 2026; návaznosti otevřené. Referenční podklad E-047, znovu ověřená
veřejná nápověda https://www.idoklad.cz/podpora/nastaveni-pokladna.
Nejde o certifikaci platebního zařízení ani integraci bankovního terminálu.

Terminál je profilové označení místa/zařízení, ze kterého pochází prodejka.
Správa obsahuje vytvoření, přejmenování a archivaci; kód je v profilu unikátní,
název uživatelsky čitelný. Archivované zařízení nelze vybrat u nového prodeje.
Historické prodejky uloží kopii názvu i kódu; přejmenování jejich obsah nezmění.

Naše rozhodnutí: kód 1–32 ASCII písmen/číslic/pomlček/podtržítek, název 1–100
znaků bez řídicích znaků. Změny vyžadují editor/owner, revizi a idempotentní
příkaz v historii profilu. Čtení je dostupné členům včetně viewer.
Tato konfigurace sama nevytváří pokladní pohyb ani úhradu a nepřipojuje externí
platební službu. Vazby a historické snapshoty dokončí P16-07/P16-08.

Akceptace: revize, replay, duplicitní kód, archiv/obnova, profilová izolace,
viewer, REST/MCP, formulář CS/EN a následně snapshot v prodejce.


Migrace 038: sales_terminal s profilovou jedinečností kódu. Změny používají
cash_execute a cash_command audit/replay, čtení cash_terminals. UI je v sekci
Pokladna → Pokladny. Ověření: cash.rs profilové role/revize/replay a REST/MCP,
cash.spec.ts CS/EN desktop/mobile vytvoření a archivace/obnova.
Celá P16-09 není dokončená bez použití a snapshotů v prodejkách.
