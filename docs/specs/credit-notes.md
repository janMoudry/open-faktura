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

CZK a vlastní vydané faktury of-czk-v1, včetně vyúčtovacích. Importované doklady
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
Před nasazením zastavit starý server. E-mail/přílohy dobropisů, cizí měny,
vrubopisy a přímé opravy DDPP zůstávají navazující funkcí.

## Ověření

[Scénáře a výsledky P13-04](../design/credit-note-validation.md).
