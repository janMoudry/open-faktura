---
name: faktura-testing
description: Ověřuje změny Open Faktury referenčními, integračními a uživatelskými scénáři. Použij při návrhu testů a validaci implementovaných funkcí.
---

# faktura-testing

Přečti dotčenou specifikaci, AGENTS.md a podle potřeby `docs/research/SCENARIOS.md`. Spouštěj pouze příkazy skutečně dostupné v projektu.

Referenční částky musí pocházet z doloženého pozorování, ne z testovaného kalkulátoru. Syntetický očekávaný výsledek označ jako vlastní pravidlo. Odděl úspěšný technický test od potvrzené shody s iDokladem.

Podle rizika změny vyber doménový test, PostgreSQL integrační test nebo UI scénář. Pro peníze zahrň hranice přesnosti, pro zápisy souběh/opakování, pro firmu negativní přístup přes navázaná ID. Neopakuj všechny testy po čistě textové změně.

Pro UI ověř uživatelský výsledek, ne interní strukturu komponenty. Emulace prohlížeče nedokládá instalaci PWA nebo systémový tisk desktopu. Výsledek uveď s prostředím, skutečným příkazem a zbývajícími mezerami; nevytvářej zdání běhového testu statickou kontrolou.
