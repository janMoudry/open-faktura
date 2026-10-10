# Open Faktura

Kompletně otevřená fakturační aplikace pro české podnikatele a firmy, kterou si každý může nainstalovat a provozovat na vlastním serveru.

Cílem je plná funkční alternativa iDokladu: fakturace, nabídky, zálohy, přijaté doklady, sklad, banka, pokladna, automatizace, daňové podklady a integrace. API a MCP budou součástí produktu od začátku.

## Stav projektu

Projekt je ve fázi průzkumu a specifikace. Tento repozitář zatím obsahuje pouze dokumentaci; aplikace ani MCP server zde nejsou implementované.

- [Soupis funkcí iDokladu, zdroje a návrh otevřené alternativy](FEATURES.md)

## Principy

- Celý produkt bude otevřený: frontend, backend, API, MCP, databázové migrace a vlastní konektory.
- Všechny funkční moduly budou dostupné i při provozu na vlastním serveru.
- Uživatel bude mít kontrolu nad daty, přílohami, zálohami a exporty.
- Základní fakturace bude fungovat bez placené služby provozovatele projektu.
- Web, API a MCP budou používat stejná pravidla, výpočty a oprávnění.
- Případný placený hosting nabídne spravovaný provoz stejné otevřené aplikace.

## Plánovaný rozsah

- Vydané faktury, dobropisy, zálohy a jejich vyúčtování.
- Cenové nabídky, šablony a pravidelná fakturace.
- Přijaté faktury, účtenky a inbox s vytěžováním dokumentů.
- Adresář, ARES a ověřování DIČ.
- Ceník, sklad a skladové pohyby.
- Úhrady, banka, párování plateb, pokladna a prodejky.
- E-maily, upomínky a poděkování za úhradu.
- Přehledy, daňové podklady a účetní exporty.
- Více firem, uživatelé, oprávnění a audit změn.
- Mobilní použití, API, integrační události a MCP.
- Instalace přes Docker Compose, zálohování, obnova a aktualizace.

## Postup vývoje

1. Doplnit veřejný soupis o pozorování aplikace, dokumentované API a exporty.
2. Ke každé funkci popsat vstupy, výstupy, stavy, pravidla a hraniční případy.
3. Připravit scénáře a testy pro ověření funkční shody.
4. Implementovat vlastní aplikaci a vlastní uživatelské rozhraní.
5. Postupně pokrýt celý cílový rozsah a připravit samostatnou instalaci.

## Licence

Cílem je kompletně open-source produkt. Konkrétní licence zatím nebyla vybrána a tento repozitář zatím licenci neobsahuje; rozhodnutí bude doplněno před zveřejněním implementace.

## Referenční produkt

iDoklad slouží jako reference funkčního rozsahu. Open Faktura je samostatný projekt bez spojení se společností Seyfor nebo produktem iDoklad.
