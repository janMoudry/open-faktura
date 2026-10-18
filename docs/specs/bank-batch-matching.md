# P16-03 — hromadné potvrzení jednoznačných bankovních shod

14. 10. 2026, důkaz E-043. Navazuje na P16-01/02. Základem je dokumentované
párování podle VS a částky; hromadný náhled a transakční hranice jsou náš návrh.
Nejde o potvrzenou kopii konkrétního hromadného dialogu iDokladu.

## Rozsah

Uživatel spustí „Spárovat zobrazené“ v seznamu banky. Rozsah tvoří nespárované
pohyby aktuální stránky a filtrů (nejvýše 25), nikoli skryté stránky nebo účty.
Náhled vypíše každý vybraný pohyb, cílový doklad, částku a důvod přeskočení.
Úhrady vzniknou až po výslovném potvrzení.

Automatický výběr musí mít jediného způsobilého kandidáta podle P16-02 a přesnou
částku: buď zůstatek dokladu, nebo existující bankovní úhradu. Částečné úhrady,
přeplatky, chybějící VS, nejednoznačnost, archivovaný účet a již spárovaný pohyb
zůstávají k ručnímu řešení. Účtenky bez VS se automaticky nevybírají.

Více vybraných pohybů pro stejný doklad vyřadí všechny tyto návrhy. Stejně tak
jiný nespárovaný pohyb stejného profilu se stejným normalizovaným VS a podepsanou
částkou (i na jiné stránce/účtu) znamená nejednoznačnost. Nepředpokládáme, že
první z více možných plateb je správná. Toto konzervativní pravidlo je náš návrh.

## Kontrakt a transakce

`POST /bank/batch-preview`, MCP `bank_batch_preview`, vstup movement_ids (1–25
unikátních ID). Všechny pohyby musí patřit dostupnému profilu. Výstup items,
ready_count a fingerprint SHA-256 kanonického náhledu včetně revizí.
Fingerprint není oprávnění ani bearer secret; server jej vždy znovu vypočítá.

`bank_execute action=batch_match` přebírá stejná ID, fingerprint a UUID.
Owner/editor zapisuje, viewer může pouze zobrazit náhled. Pod profilem a
transakčním zámkem server přepočítá náhled. Změna proti potvrzenému náhledu je
409: žádné částečné provedení, uživatel musí načíst nový náhled. Každý zápis dále
ověří revize i stav dokladu společným příkazem ručního párování. Jediný neúspěch
vrátí celou dávku. Stejné UUID vrací uložený výsledek bez dalších úhrad.

Bank_match a platební audit uchovávají aktéra; bank_command uchovává původní
vstup a výsledek celé dávky. Databázové schéma se nemění; používají se tabulky
migrace 027. Odpojení funguje na každém pohybu stávajícím způsobem. Existující
úhradu pouze připojujeme. Poděkování P14 může reagovat na skutečně nové úhrady,
pokud je uživatel předem povolil. Funkce žádnou automatizaci sama nezapíná.

## Akceptační scénáře

- Dvě jednoznačné shody + bez VS: náhled nic nezapíše; potvrzení spáruje pouze dvě.
- Existující platba se nezdvojí; opakování UUID a souběh různých UUID nezdvojí dávku.
- Druhá platba stejného VS/částky mimo výběr zablokuje automatickou volbu.
- Změna zůstatku/revize, archivace nebo nové párování po náhledu znamená konflikt.
- Selhání druhé úhrady vrátí první úhradu i bankovní vazbu v celé transakci.
- Záměna profilu, cizí ID, viewer, duplicitní ID, příliš dlouhá dávka: odmítnout.
- REST a MCP shodné; UI CS/EN desktop/mobile, loading, retry, prázdný náhled,
  vysvětlené vynechání a potvrzení. Žádné testování na skutečných účtech.

Ruční dávka zůstává operací s náhledem a potvrzením. P16-14 přidává samostatně
povolený [průběžný worker](bank-auto-matching.md), který sdílí evaluátor i
platební příkazy. Bankovní notifikace/PSD2 zůstávají otevřené.
