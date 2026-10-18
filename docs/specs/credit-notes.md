# CREDIT-NOTE-01 / P13-04 — dobropisy

Reference E-035: veřejná nápověda iDokladu, nikoli nový experiment v přihlášené
aplikaci. Z běžné vydané faktury vzniká upravitelný koncept s důvodem opravy,
částečnou nebo úplnou korekcí a samostatným zápočtem proti původní faktuře.

## Zvolený rozsah a vlastní pravidla

Návrh → vystavení (řada DB{rok}{0001}) → zápočet nebo ručně evidované vrácení
peněz. Návrh lze upravit/zrušit; vystavený doklad a jeho PDF jsou neměnné.
Vystavení nic neodesílá ani nepřesouvá peníze. Zápočet je samostatná akce pouze
v celé zbývající částce dobropisu, nejvýše do neuhrazené částky původní faktury.
Vratky mohou být částečné; storno evidovaného zápočtu/vratky zachovává audit.

Vlastní vydané faktury of-czk-v1; CZK včetně vyúčtovacích, cizí měny dle P19-01h níže. Importované doklady
nemají zaručené řádkové rozpisy a jsou odmítnuté. Oprava jednotlivých řádků se
zadává kladnou částkou včetně DPH, nejvýše dosud nedobropisovaná částka řádku.
To je naše zjednodušení editoru; přesná shoda s editorem/ceníkem iDokladu není
ověřená. Položky a strany se přebírají z historie, nepřepočítávají z dnešních cen.
Neplátce dostane „Dobropis“, plátce „Opravný daňový doklad“. Režim zdroje se nemění
ani při změně aktuálního režimu profilu. Důvod povinný při vystavení; datum
vystavení mezi datem původní faktury a dneškem, datum opravy mezi původním datem
plnění a vystavením, splatnost ne před vystavením. Automatické vyhodnocení
legislativních lhůt, přiznání DPH a doručení odběrateli nejsou součástí.

## Peníze a návaznost na zálohy

Základ tvoří uložené kladné výsledky řádků plnění původní faktury. U vyúčtování
se nepoužije její již zmenšená rekapitulace DPH: ta obsahuje odpočty DDPP.
Zálohy/DDPP a vazby zůstávají zachované a rezervované; dobropis opravuje cenu
plnění, neuvolňuje je pro druhé použití. Nejde o zrušení samostatného DDPP.

Zaokrouhlení zdroje rozdělujeme samostatně kumulativně poměrem opravované ceny
k součtu původních řádků plnění. Neodečítáme ho z kapacity malé poslední položky.
Základ a DPH dílčí
opravy jsou rozdílem kumulativně poměrně rozdělených uložených částek (na haléře,
půlka od nuly). Poslední oprava tak vrátí přesný zbytek. Rozdíl peněžní částky
proti základu+DPH je korekce zaokrouhlení. Plný dobropis přesně obrací uložené
plnění včetně zaokrouhlení; historické sazby se znovu nepočítají. Částky dokladu
jsou záporné, vyrovnání/vratky kladné. Toto je naše pravidlo, ne pozorovaná
numerická shoda. Souběh při vystavení zamyká zdrojovou fakturu a znovu kontroluje
vyčerpané řádky; starý návrh nesmí překročit zůstatek. Nulová peněžní korekce
se nevystavuje; je nutné ji zahrnout do větší opravy. Změna pořadí vystavení
částečných oprav může přesunout haléř zaokrouhlení, proto se při vystavení
rozpis znovu přepočítává z uložených částek.

Zápočet mění stav úhrady faktury, není peněžním příjmem. Přehled přijatých úhrad
zahrnuje pouze skutečné invoice_payment. Vrácené částky mají vlastní historii
dobropisu; přehled zatím není čistý cash-flow výkaz včetně všech vratek/záloh.
Zápočet i vratka používají revizi a idempotentní UUID, rušení transakce má vlastní
událost a důvod. Původní faktura, PDF a úhrady se nepřepisují.

Migrace 021, company-scoped FK, číslování, snapshoty, audit a události. Stejné
aplikační příkazy pro REST/MCP/UI; owner/editor zapisuje, viewer pouze čte.
Před nasazením zastavit starý server. E-mail/přílohy dobropisů, cizoměnové bankovní párování,
vrubopisy a přímé opravy DDPP zůstávají navazující funkcí.

## Ověření

[Scénáře a výsledky P13-04](../design/credit-note-validation.md).

## P19-01h — Cizoměnové opravy

Běžnou vlastní cizoměnovou fakturu lze dobropisovat v její původní měně.
Kurz se čte z neměnného source snapshotu; nelze ho v opravě přepsat.
Jde o naše explicitní pravidlo, nikoli ověřené chování iDokladu (E-064).
Částky opravy, zápočtu a vratky jsou v měně dokladu. Ruční vratka podporuje
bankovní převod; hotovost a párování na nynější CZK banku/pokladnu jsou
odmítnuté. DPH, bilance a obrat přepočítají dobropis i jeho vyrovnání
uloženým kurzem. PDF i UI uvádějí měnu dokladu. Bez nové migrace, snapshot
kurzu již obsahuje source.data.calculation.exchange_rate z P19-01g.
Historické CZK dobropisy a PDF cache se nemění. Zálohy a vyúčtování v cizích
měnách, kurzové rozdíly a měnové přepočty samotných vratek zůstávají otevřené.

Akceptace: 121 EUR (100 + 21 DPH), kurz 25, plný dobropis −121 EUR,
DPH −525 CZK; zápočet vyrovná původní fakturu bez vzniku cash příjmu.
Částečná bankovní vratka a její storno mění jen zůstatek dobropisu;
hotovost je odmítnuta, cizí profil nemá přístup, PDF obsahuje EUR a kurz.

### Ověření P19-01h (17. 10. 2026)

`credit_note` a `period_report`: 18 testů prošlo v izolované PostgreSQL,
včetně EUR vratky/storna, zápočtu, kurzu, DPH, bilance a profilové izolace.
`pnpm test:e2e tests/e2e/credit-notes.spec.ts`: 8/8 pro CZK/EUR × CS/EN ×
desktop/mobile, včetně PDF, historie a a11y seznamu. Screenshoty credit-*-EUR-*
v docs/design obsahují syntetické údaje. Text CS/EN PDF ověřen lokálně:
−605 EUR, základ −500 EUR, DPH −105 EUR, kurz 25 CZK / 1 EUR.
`pnpm check`, `pnpm build:web` a `git diff --check` prošly; build pouze
upozorňuje na velikost některých JS chunků. Shoda výpočtů s iDokladem
není těmito vlastními testy potvrzena.

Regrese banky: 33/37 prošlo v prvním běhu; 4 connection testy selhaly kvůli
chybějící cestě ke klíči. Po nastavení absolutního BANK_ENCRYPTION_KEY_FILE
prošlo všech 6 connection testů (včetně čtyř původně selhavších). Pokladna:
16/16. Testy použily izolované DB a syntetický bankovní transport.
