# P17 — Inbox a příjem dokumentů

Podklad E-051, 15. 10. 2026. Veřejná nápověda dokládá produktový tok;
níže uvedené limity a transakční pravidla jsou vlastní návrh Open Faktury.

## Originál a životní cyklus

Profilový inbox přijímá originál PDF, PNG, JPEG a ISDOC. Originál je neměnný,
má hash, název, typ, velikost, čas a autora. Název nesmí obsahovat cesty,
řídicí či bidi znaky. Limit 2 MiB na soubor, 256 MiB na profil; limity mají
být stejné přes UI, REST a MCP. Zobrazení souboru používá autentizovaný
download, nosniff a attachment; HTML/SVG se nevykonává.

Stavy: pending, processed, archived. Archivace nemaže originál ani přílohu.
Duplicitní upload ve stejném profilu vrací původní položku podle hashe,
nikoli novou fakturu. Jiný profil nesmí zjistit existenci stejného souboru.
Návrh vytěžení není účetní doklad. Potvrzení vytvoří přijatý doklad společným
příkazem, připojí originál a uloží vazbu atomicky. Opakování nevytvoří duplikát.
Člen viewer smí číst, editor/owner smí zpracovat; instalátor členství neobchází.

## Parsery a externí vstupy

ISDOC a QR jsou nedůvěryhodné vstupy. Strukturovaný parser musí mít limit
hloubky, velikosti a počtu položek; žádné načítání externích entit či URL.
Nepodporované měny/režimy se ukážou jako nepodporované, nikoli tiše převedené.
Původní součty se zachovají a kontrolují vůči zdroji. Neověřený podpis nesmí
být prezentován jako ověřený.

Příjem e-mailu a AI jsou samostatné opt-in adaptéry s vlastní konfigurací,
tajemstvími, idempotencí a diagnostikou. Základní ruční tok funguje bez nich.
Výstup modelu nikdy nespouští příkaz ani automatické zaúčtování.

## Akceptace

Originál a hash; upload/opakování/archiv/obnova; profilová izolace a role;
atomický převod a kopie přílohy; souběžný převod; audit; odmítnuté typy/limity;
CS/EN UI a stejné REST/MCP výsledky. Parsery, e-mail a AI mají samostatné
scénáře. Částečnou podporu neoznačovat jako celé P17.

## Implementovaný rozsah 15. 10. 2026

Migrace 041: originál, SHA-256 deduplikace, archiv/obnova a potvrzený převod
do přijatého dokladu s kopií originálu. Připojení k existujícímu dokladu
sdílí stejnou transakční operaci. CS/EN UI má výběr faktury/účtenky a vyhledání.
Při konverzi se neobchází validace, číslování ani cash automatika přijatých
dokladů. Backend má audit; jeho samostatné prohlížení v UI zatím zbývá.

Read-only ISDOC extrakce má explicitní tlačítko v editoru. Limity a rozsah
jsou v E-052. Originál se nemění; parser vyžaduje shodu součtů a zachovává
částky řádků. Nepodporovaný vstup lze dokončit ručně. Nejde o implementovaný
QR, vložený ISDOC.PDF, mail receiver ani AI.

Ověřeno: dva PostgreSQL scénáře pro originál, replay, souběh, autorizaci,
rollback neplatného převodu a archiv; devět regresí přijatých dokladů;
CS/EN desktop/mobile průchod upload → ruční editor → přijatý doklad.
Parser má samostatné syntetické testy přesných částek a odmítnutých vstupů.
