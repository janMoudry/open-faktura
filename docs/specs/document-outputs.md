# Doplňkové výstupy faktur (P20-03a)

18. 10. 2026. Podklad: FEATURES a P20-03. Formáty, limity a texty níže jsou
návrhem Open Faktury; tato dodávka netvrdí numerickou ani vizuální shodu s iDokladem.

Člen aktivní firmy může stáhnout hromadné PDF až 50 vystavených faktur. Pořadí
odpovídá výběru; opakované ID se odmítá. Zpracují se původní uložená PDF,
bez přepsání již existující cache. Chybějící cache se nejdříve vytvoří společnou cestou PgDraft.pdf ze snapshotu
mimo čtecí transakci; návrhy a doklady jiné firmy nejsou dostupné.
Limit vstupních PDF je 50 MiB; nevzniká tichý částečný výstup. Sloučení zachová
vnořené stromy stránek i zděděné fonty/zdroje každého původního dokumentu.

Pro jednu vystavenou fakturu jsou dostupné:

- **RTF**: editovatelná kopie ze snapshotu, s Unicode UTF-16 escapes a bezpečným
  escapováním řídicích znaků RTF. Jde o doplňkový výstup, nikoli přepsání
  původního PDF. Základ/DPH/k úhradě jsou uložené součty, žádný přepočet přes float.
- **Dodací list PDF**: číslo a datum zdrojové faktury, původní strany a položky
  s množstvím/jednotkou; volitelně uložené ceny a DPH podle firemní předvolby
  nebo jednorázového výběru (viz níže). Nevytváří další skladový pohyb ani finanční
  doklad, nepotvrzuje fyzické předání.
- **Potvrzení evidovaných úhrad PDF**: původní strany a seznam aktivních peněžních
  úhrad s datem, částkou v měně dokladu a metodou. Stornované úhrady se vynechají;
  při žádné aktivní úhradě konflikt. Nezahrnuje zápočty a platby záloh, není
  daňovým dokladem. Vzniká z aktuální evidence, nepředstírá historickou cache.

Doplňkové výstupy mají explicitní jazyk cs/en. Čtou jediný REPEATABLE READ
snapshot; autorizují před daty a znovu po omezeném CPU vykreslení na blocking
poolu (nejvýše dvě souběžná renderování). Nemění doklad, úhrady ani cache.
Žádné externí I/O. HTTP a MCP volají `OutputStore`; odpovědi jsou no-store.

UI registru nabízí výběr aktuální stránky; detail vydané faktury nabízí
jednotlivé doplňkové výstupy. Kontrakt `document-output.openapi.json`, generovaný
klient a MCP `document_output`. Testy `document_output.rs` ověřují izolaci,
počet a obsah stránek, odmítnutí návrhů/duplicit, RTF, dodací list a storna úhrad.
Doménová pravidla faktury ani úhrad se nemění. Podpora ostatních druhů dokladů,
vlastní tiskové šablony a podrobná právní sazba RTF zůstávají samostatně otevřené.

Pro integrační textové ověření PDF je potřeba Poppler pdftotext. Lom textu
nenahrazuje vizuální kontrolu; testovací OUTPUT_REVIEW_DIR uloží pouze syntetická
PDF pro rasterizaci a kontrolu sazby.

## P20 — Ceny na dodacím listu (18. 10. 2026)

Veřejná nápověda [Nastavení – Prodej](https://www.idoklad.cz/podpora/nastaveni-prodej),
aktualizovaná 23. 4. 2024, přečtená 18. 10. 2026, výslovně uvádí tisk
dodacího listu s cenami nebo bez nich. Jde o veřejnou dokumentaci, nikoli
pozorování aktuálního UI/API nebo důkaz přesného rozložení cen.

Open Faktura rozšiřuje společné nastavení vzhledu o `delivery_note_prices`.
Výchozí hodnota je false; dřívější JSON a jeho UUID replay zůstávají platné.
Uložit preferenci smí vlastník firmy s revizí; ostatní členové ji čtou.
Při novém vyžádání dodacího listu se přečte aktuální firemní preference
ve stejném snapshotu jako faktura. Požadavek může výslovným true/false
jednorázově přepsat výchozí volbu; u jiného druhu výstupu je tato volba chyba.
CS/EN UI nabízí uloženou předvolbu i jednotlivý výběr před stažením.

Položky vždy obsahují původní název, množství a jednotku. Varianta s cenami
přidává uložený základ, DPH a celkovou částku každé položky v měně faktury.
Částky se nepřepočítávají; chybějící uložené částky znamenají konflikt.
Nedochází k odpočtu úhrad či záloh ani k přebírání dnešního zůstatku.
Doklad výslovně uvádí, že nejde o výzvu k úhradě. Varianta bez cen neobsahuje
cenové údaje ani měnu. Volba nemění finanční obsah, původní PDF ani jeho cache.

Jde o vlastní konkrétní prezentaci doložené volby; netvrdíme vizuální shodu
iDokladu. Nová databázová migrace není potřeba (verzovaný JSON nastavení).
Test `document_output` ověřuje CS/EN oba směry override, izolaci firem,
uložené částky, odmítnutí jiné operace a byteovou neměnnost původního PDF.
Regrese `document_style` zachovává role, revize/replay a historické snapshoty.
