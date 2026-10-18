# P19-01o — Osobní varianty widgetů

Vlastní produktové rozhodnutí z 18. 10. 2026 nad existujícími přehledy,
nikoli doložená přesná konfigurace iDokladu. Každý účet si může uspořádat
finanční a daňový widget: zapnout/vypnout, změnit pořadí a zvolit podrobnou
nebo stručnou variantu. Výchozí rozložení je finance + daně, obě podrobné.
Tlačítko pro konfiguraci zůstává dostupné i po skrytí obou widgetů.

Stručná finanční varianta ponechá prodej/nákup a vyrovnání, podrobná navíc
pásma prodlení a partnery. Stručná daňová varianta zobrazuje orientační obrat
a celkovou evidovanou vstupní/výstupní DPH, podrobná navíc období a zdroje.
Skrytí widgetu neodebírá přístup k jeho samostatným dokladům ani nemění
výpočty. Konfigurace neobsahuje účetní částky.

GET/POST `/api/auth/dashboard-widgets`, MCP dashboard_widgets_get/set.
Identita pochází výhradně z přihlášené relace, nikdy z company_id či user_id
ve vstupu. Nastavení platí napříč profily a klienty stejného účtu. Finanční
API si samostatně kontrolují profilová oprávnění. Není nutná role správce.

Migrace 070 přidává user_dashboard_layout s FK na uživatele. Revize 0 bez
zápisu znamená výchozí nastavení. Zápis pod zámkem účtu, optimistic revision;
bezprostřední stejné opakování je idempotentní, odlišný starý vstup konflikt.
Povolené pouze dva známé druhy, každý nejvýše jednou; prázdný seznam je
platný. Neznámá varianta, duplicitní druh nebo záporná revize jsou neplatné.
UI zachová rozpracovanou volbu při chybě a nabídne explicitní načtení aktuální
revize; neprovádí tiché přepsání konkurenčního nastavení. Návrat na výchozí
rozložení vyžaduje uložení stejně jako ostatní úpravy.

Test `dashboard_widgets.rs` používá izolovanou PostgreSQL: default, vlastní
účet, souběžné shodné zápisy, replay, starou revizi, duplicitní widget,
prázdnou konfiguraci, oddělení druhého účtu a REST autentizaci.
