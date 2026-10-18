# P19-01g — Kurz dokladu a přehledy v CZK

Podklad E-063: veřejná nápověda iDokladu uvádí přepočet cizích měn kurzem
uloženým na dokladu. Směr, přesnost a fáze zaokrouhlení níže jsou vlastní
pravidla Open Faktury; numerická shoda s iDokladem není ověřena.

## Uložení a výpočty

Calculation.exchange_rate a received Data.exchange_rate jsou volitelné
snapshoty `{rate, units}`. CZK je bez snapshotu, jiná podporovaná měna jej
vyžaduje. rate = CZK za units jednotek cizí měny. rate kladné nejvýše milion,
max. 6 desetinných míst; units kladné celé číslo nejvýše milion. Podporované
měny: CZK/EUR/USD/GBP/CHF/PLN/HUF/JPY/SEK/NOK/DKK/CAD/AUD. Částky dokladů
mají v této verzi dvě desetinná místa i pro JPY; nejde o podporu hotovostních
nominálů všech měn. P19-01j doplňuje explicitní načtení návrhu kurzu ČNB; viz exchange-rates.md.
Automatické změny kurzů nejsou součástí této verze.

Přepočet `částka × rate / units`, Decimal/NUMERIC, na 2 desetinná místa
od poloviny od nuly. Zaokrouhluje se jednotlivý doklad (či jeho aktuální
zůstatek), poté se sčítá. Výpočty položek zůstávají v měně dokladu, WholeCzk
je povoleno jen pro CZK. Peněžní tok v přehledu úhrad používá kurz faktury,
ne kurz bankovní transakce; realizované kurzové rozdíly se zatím neúčtují.

Migrace 051 přidává SQL funkci report_czk. Stávající CZK data se nepřepisují.
Kurz je uložen ve stávajících JSONB datech a auditu, po vystavení vydané
faktury se spolu s ostatním obsahem nemění. Bez platného kurzu přehled
selže, místo aby cizí měnu považoval za CZK nebo ji tiše zahodil.

## Rozsah

Vydané běžné faktury: vytvoření, vystavení, PDF, ruční bankovní úhrady,
import historických dokladů s kurzem. Přijaté běžné faktury: vytvoření a
ruční bankovní úhrady; přehled hodnot, bilance, prodlení, odběratelé/dlužníci,
DPH, orientační obrat a přehled nákupních úhrad používají kurz dokladu.
Domácí měna profilu zůstává CZK. Export/dokumentové UI zachovává cizí měnu,
přehledy vracejí CZK. Měna a kurz se nikdy neberou z dnešní kurzovní tabulky.

Prodejky, účtenky, skladové vazby a
CZK bankovní/pokladní párování zatím zůstávají v CZK. Backend odmítá jejich
nepodporované cizoměnové kombinace. Cizoměnové položky nelze přebírat
z CZK ceníku bez konverze, proto se zadávají přímo. U bankovní ruční úhrady
částka znamená měnu dokladu; bez implicitního párování na CZK pohyb.
Tyto mezery nejsou dokončením celého měnového účetnictví ani P19.

Akceptace: 100 EUR při 25 CZK/EUR → 2500 CZK; úhrada 40 EUR → zůstatek
1500 CZK; kurz za 100 jednotek; záporné půlhaléře; chybějící/nulový kurz;
změna dnešního nastavení nemění historické výsledky; profilová izolace;
CZK regrese, CS/EN a zachování měny v PDF i úhradách.

## Ověření 17. 10. 2026

- `pnpm check` a `pnpm build:web` prošly.
- Izolované PostgreSQL testy invoice, payment, received, import a period_report:
  43 úspěšných testů; samostatný nový test přijaté JPY faktury také prošel.
- Doménový test exchange ověřil kurz za 100 jednotek a hranice zaokrouhlení.
- `pnpm test:e2e tests/e2e/dashboard.spec.ts`: 4/4 scénáře CS/EN,
  desktop/mobile. Vytvoření EUR dokladu, uložení a opětovné načtení kurzu,
  vystavení, bankovní úhrada a zůstatek v přehledu. Screenshoty fx-payment-*
  v docs/design používají výhradně syntetickou testovací agendu.
- Lokální vývojová DB byla zálohována před upgradem; migrace 051 prošla
  a nový server vrací ready. Build upozorňuje na velikost některých JS chunků.

Testy ověřují vlastní výpočetní pravidla. Nejsou potvrzením numerické
shody s iDokladem ani plné podpory cizoměnových účetních operací.

P19-01h doplňuje cizoměnové dobropisy se stejným kurzem jako původní faktura,
zápočty a ruční bankovní vratky. Viz [dobropisy](credit-notes.md).

P19-01i podporuje zálohy/DDPP/vyúčtování se společným uloženým kurzem; rozdílné kurzy jsou odmítnuty. Viz advance-settlement.md.

P16-13a/P19-01k doplňuje explicitní ruční CZK bankovní párování běžných cizoměnových faktur. Přehledy stále oceňují úhrady kurzem dokladu; uložený rozdíl se zobrazuje u bankovní vazby, nikoli v souhrnném výkazu. Viz bank-foreign-matching.md.
