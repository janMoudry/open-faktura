---
name: database-migrations
description: Navrhuje a ověřuje PostgreSQL migrace Open Faktury, transakce a izolaci firem. Použij při změnách schématu, indexů a persistovaných dat.
---

# database-migrations

Přečti `docs/DATA_MODEL.md`, `docs/architecture/DEVELOPMENT_CONVENTIONS.md` a dotčené požadavky. Pro změnu doménových pravidel načti také sourozenecký skill faktura-domain.

Migrace mají jediný zdroj v migrations. Již vydanou migraci nepřepisuj; přidej novou. Popiš dopad na existující data, zámky a souběžně běžící starší aplikaci. Destruktivní změnu rozděl na přidání, převod a pozdější odstranění, pokud je potřebná kompatibilita.

Přesnosti NUMERIC musí odpovídat limitům Rust typů a mezivýpočtů. Firemní FK nesmí odkazovat do jiné firmy. Pokud je zapnuté RLS, testuj běžnou aplikační rolí, chybějící kontext i recyklované spojení; úspěch pod vlastníkem DB izolaci nedokládá.

Ověř migraci na čisté DB a relevantní předchozí verzi se syntetickými daty. Použij skutečný PostgreSQL pro zámky, RLS a transakční testy. Migraci nespouštěj na produkci jen proto, že vznikl SQL soubor.
