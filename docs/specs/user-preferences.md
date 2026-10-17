# P20-06a — Vzhled uživatelského rozhraní

Vlastní návrh Open Faktury, 15. 10. 2026: světlý/tmavý/systémový vzhled.
Volba patří přihlášenému uživateli, nikoli fakturačnímu profilu, a ukládá se
do PostgreSQL s revizí. Po přihlášení se načte ze serveru. Lokální cache
slouží jen k rychlému prvnímu vykreslení. Změna systémového tématu reaguje
pouze v režimu system. Přístup má pouze vlastník uživatelského účtu přes
svou relaci; instalační admin cizí preference nečte.

Monochromatický brand a sémantické barvy chyb/varování/úspěchu zůstávají.
Barvy jsou definované v tokenech, nikoli ve stránkách. PDF a jiné dokumentové
výstupy se změnou tématu nemění. Preferované omezení animací se dál řídí OS.
P20-06 jako celek zahrnuje také oznámení a úplný audit přístupnosti; samotné
téma neznamená dokončení celé položky.
