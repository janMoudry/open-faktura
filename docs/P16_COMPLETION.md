# Dokončení celé P16

Aktivní požadavek uživatele: dokončit P16 až P20, potvrzeno 15. 10. 2026.
Tento soubor sleduje pouze část P16; navazující rozsah zůstává v implementačním
plánu a FEATURES.md. Toto není
potvrzení dokončení. Dřívější P16-01 až P16-06 mají vymezený rozsah ve specifikacích.

## Aktuální měnová návaznost

P16-13h doplňuje zahraniční IBAN bankovní evidence bez migrace: normalizace,
validace formátu/checksumu, profilová unikátnost a zachování kontroly účtu/měny
importu. [Rozsah](specs/bank-currencies.md#p16-13h--zahraniční-iban-bankovních-účtů).
P16-13i doplňuje zahraniční IBAN dokladů a P16-14b opt-in automatiku záloh
a vratek. P16-13j nyní doplňuje i jejich párování mezi různými měnami (058), včetně připojení existující úhrady a ochrany DDPP. QR Faktura/Platba+F se řeší ve společném dokumentovém výstupu. Historické záznamy níže popisují stav v době dodání.

## Akceptační celky

- [ ] P16-07 Prodejky: položky, nepovinný odběratel, skutečné plné úhrady
  hotovost/karta/stravenka včetně kombinace; číslování, opravy, historie a PDF.
- [ ] P16-08 Souhrnné zaúčtování prodejek: pokladní doklad za hotovost,
  interní doklad za kartu/stravenku; uložené daňové rozpisy, storno ze zdroje,
  zabránění dvojímu zaúčtování i dvojímu započítání do přehledů.
- [ ] P16-09 Terminály: profilové nastavení, archivace, volba u prodejek,
  zachování historické identifikace zařízení; externí integrace dle kontraktu.
  P16-09a: profilová konfigurace terminálů, revize, archivace/obnova, REST/MCP
  a CS/EN jsou implementované. Volba a snapshot prodejky jsou implementované;
  externí integrace zůstává otevřená. [Specifikace](specs/sales-terminals.md).
- [x] P16-10 Bankovní počáteční stavy a saldo: období, úplné součty nezávislé
  na stránkování, import historických pohybů, revize a audit změny počátku.
- [x] P16-11 Rozdělení pohybu: více dokladů, přesný nerozdělený zbytek,
  přeplatek, opakování a atomické odpojení pouze vytvořených úhrad.
  [Pravidla a odchylka od reference](specs/bank-split-matching.md).
- [x] P16-12 Zálohy/dobropisy: bankovní a pokladní vazby přes jejich vlastní
  platební operace; bez obcházení rezervací DDPP a vyúčtování.
  [Pravidla a ověření](specs/treasury-documents.md).
- [x] P16-13 Cizí měny: měna účtu/pohybu, uložený kurz a datum, přesná částka
  v měně dokladu; explicitní omezení při chybějícím více-měnovém doménovém základu.
- [x] P16-14 Průběžné párování: opt-in, omezení přesných shod, oprávnění
  aktivujícího uživatele, idempotence, audit, chyby a vypnutí; externí vstupy
  nesmějí automatiku zapnout. [Rozsah a pravidla](specs/bank-auto-matching.md).
- [ ] P16-15 Bankovní notifikace/API: vlastní konfigurace uživatele pro více bank,
  průvodce připojením a test spojení, ověření autenticity, ochrana před replay,
  správa tajemství, diagnostika a ověření jednotlivých adaptérů.
  P16-15a implementuje nastavení, test a ruční synchronizaci pro Fio a vlastní
  gateway, včetně REST/MCP a CS/EN. [Specifikace a limity](specs/bank-connections.md).
  P16-15b přidává opt-in plánování synchronizace a worker s oprávněním aktéra.
  Notifikace, další nativní adaptéry a živé ověření zůstávají otevřené.
  [Rozhodnutí a akceptace](#vlastní-bankovní-napojení-p16-15).
- [ ] P16-16 Tisk: částka slovy CS/EN, konfigurace sestav, kontrola PDF,
  archivované a stornované dokumenty; fyzický tisk není browser emulace.
  Částka slovy CS/EN je hotová pro nově generovaná pokladní PDF; konfigurace sestav zbývá.
- [ ] P16-17 Členění DPH: doložený číselník, uložené hodnoty, konzistentní
  přenos mezi prodejkami a souhrnnými doklady. Daňové výkazy navazují v P19.
- [ ] P16-18 Ověření bankovních adaptérů na anonymizovaných skutečných výpisech.
  Syntetické fixtures nelze přeznačit za originální exporty.

Každý celek vyžaduje společný backend REST/MCP, oprávnění aktivního profilu,
CS/EN, monochromatické UI, testy a aktualizaci specifikace. Finální stav P16
lze změnit na dokončeno teprve po splnění všech přijatých celků nebo výslovné
změně rozsahu uživatelem. Nepřesouvat chybějící funkce do jiné etapy pouze
za účelem formálního uzavření.

## Externí závislosti

Uživatel nemá poskytovatele PSD2/API ani připojenou schránku. Podle rozhodnutí
ze 14. 10. 2026 se jeho osobní přístup nevyžaduje pro vývoj: bankovní napojení
si nastaví každý provozovatel/uživatel sám. Import výpisů zůstává dostupný bez něj.
Reálné anonymizované výpisy a bankovní zprávy nejsou v repozitáři doložené;
syntetické testy nepotvrzují živou kompatibilitu všech bank.

## Vlastní bankovní napojení P16-15

Produktové rozhodnutí uživatele, 14. 10. 2026 (nejde o pozorování iDokladu):
živé napojení nabídnout napříč bankami, vlastní nastavení a přístupy zajistí
uživatel nebo provozovatel instalace. Neomezovat návrh na Air Bank ani na
jednoho poskytovatele. Toto rozhodnutí neznamená, že jsou adaptéry implementované.

Akceptace navrženého řešení:

- Profilové nastavení bankovního účtu nabídne banku a dostupné způsoby napojení:
  přímé API, podporovaného poskytovatele nebo bankovní e-mailové notifikace.
  Volby se řídí skutečnými schopnostmi konkrétního adaptéru.
- Průvodce v CS/EN vysvětlí získání přístupů, požadovaná oprávnění a nastavení
  na straně banky/poskytovatele; účty a případné smlouvy zajišťuje uživatel.
- Přihlašovací tajemství se ukládají chráněně na backendu; UI ani MCP je po
  uložení nevrací. Podpora více bank nesmí znamenat sběr hesel do internetového
  bankovnictví mimo oficiální autorizační tok poskytovatele.
- Test spojení, stav autorizace, poslední úspěšná synchronizace, srozumitelná
  chyba, opakování, odpojení a obnova přístupů jsou součástí UI.
- Synchronizace má profilovou izolaci, deduplikaci, ochranu proti replay a audit.
  Připojení samo nezapíná automatické párování; to zůstává samostatný opt-in.
- U každého adaptéru uvést podporované banky, vstupy a stav ověření. Banku bez
  implementovaného adaptéru neprezentovat jako funkční živé napojení; nabídnout
  import výpisu a možnost doplnění adaptéru.
- Vývoj a automatické testy používají syntetická data a lokální simulátory.
  Skutečné integrační ověření evidovat samostatně pro konkrétní banku a metodu;
  osobní bankovní přístupy zadavatele nejsou podmínkou implementace.

P16-13c: účty českých bank v podporovaných cizích měnách, importy a saldo
v měně účtu; migrace 054. Párování z těchto účtů, zahraniční IBAN a RB ABO
v jiných měnách navazují. [Specifikace](specs/bank-currencies.md).

P16-13d: ruční párování a rozdělení cizoměnových pohybů s běžnými fakturami
ve stejné měně, existující převodní úhrady bez duplikace, odpojení a CS/EN.
Bez nové migrace; REST/MCP sdílejí bankovní příkazy. Automatiku doplňuje P16-13e,
zálohy/vratky P16-13f. Různé měny a zahraniční IBAN zbývají.
[Specifikace](specs/bank-currencies.md#p16-13d--ruční-párování-ve-shodné-měně).

P16-13e (18. 10. 2026): dávkové a opt-in automatické párování běžných faktur
ve shodné měně, konkurenční pohyby se porovnávají jen v téže měně. CS/EN,
stejné REST/MCP příkazy a worker, bez migrace. Ruční non-CZK párování záloh/vratek
doplňuje P16-13f; různé měny a zahraniční IBAN zbývají. [Specifikace](specs/bank-currencies.md).

P16-13f (18. 10. 2026): ruční a dělené párování záloh a vratek dobropisů
z účtů ve shodné měně, připojení existujících převodních úhrad bez duplicity,
ochrany DDPP a rezervací. CS/EN, společné REST/MCP, bez migrace.
Zbývá párování rozdílných měn z non-CZK účtů, zahraniční IBAN a automatika
záloh/vratek. [Specifikace](specs/bank-currencies.md).

P16-13g: ruční párování různých měn z cizoměnové banky s běžnou vydanou
a přijatou fakturou, včetně připojení existující úhrady. Explicitní bankovní
kurz do CZK, uložené hodnoty obou stran a rozdíl; migrace 055, REST/MCP a CS/EN.
Zbývají rozdílné měny záloh/vratek, jejich automatika a zahraniční IBAN.
[Specifikace](specs/bank-foreign-matching.md#p16-13g--rozdílné-měny-z-cizoměnového-účtu).

### Dodávka 18. 10. 2026 — P16-13i, P16-08b, P16-14b

- [x] Zahraniční IBAN profilů a vystavených dokladů; historické PDF beze změny. QR zůstává CZ/CZK.
- [x] PDF/tisk souhrnného zaúčtování prodejek v CS/EN, snapshot vystavitele, cache po revizích, označení storna (migrace 056).
- [x] Samostatný opt-in pro automatické párování záloh a vratek ve stejné měně, existující úhrady bez duplicity (migrace 057).

Specifikace: [měny](specs/bank-currencies.md), [prodejky](specs/sales-receipts.md),
[automatika](specs/bank-auto-matching.md). Neuzavírá celé P16–P20. Otevřené
zůstávají rozdílné měny záloh/vratek, notifikace bank, příjem e-mailů a
vytěžování, S4/S5, účetní/daňové exporty a zbývající funkce P20.
