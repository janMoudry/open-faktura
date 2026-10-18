# Různé kurzy záloh, DDPP a vyúčtování: audit a invarianty

Historický audit 18. 10. 2026 byl pořízen před změnou peněžní logiky. Níže odděluje tehdejší stav od následně implementovaného omezeného rozsahu. Není tvrzením numerické parity iDokladu. Rozlišuje uživatelskou nápovědu, statický SDK kontrakt, právní zdroj a vlastní návrh.

## Aktuálně doložené reference

- [iDoklad zálohové faktury](https://www.idoklad.cz/podpora/zalohove-faktury), stránka uvádí aktualizaci22.4.2024, znovu přečteno18.10.2026: společně vyúčtované zálohy mají stejného odběratele a měnu, jsou plně uhrazené a nevyúčtované. DDPP vstupují do vyúčtování. Nedokládá požadavek stejného kurzu ani konkrétní CZK numeriku. Historická výjimka sazeb2023/2024 není obecný FX algoritmus.
- [iDoklad karta faktury](https://www.idoklad.cz/podpora/karta-faktury), aktualizace18.12.2025, přečteno18.10.2026: zahraniční měna má automaticky navržený a upravitelný kurz. Nedokládá kurs DDPP ke konkrétní úhradě. Uvádí, že cizoměnové doklady nezaokrouhluje; toto není specifikace konverze daňových částek do CZK.
- [iDoklad DDPP](https://www.idoklad.cz/podpora/danove-doklady-k-platbe), přečteno18.10.2026: v textu nebylo nalezeno přesné kurzové pravidlo.
- [Oficiální SDK](https://github.com/Solitea/IdokladSdk/tree/f76c7e16bc45f4fa601bf2889dc81b8d45a9dafc), commitf76c7e16bc45f4fa601bf2889dc81b8d45a9dafc, staticky přečteno18.10.2026, nikoli spuštěno. `Models/IssuedTaxDocument/List/IssuedTaxDocumentListGetModel.cs` má vlastní ExchangeRate a ExchangeRateAmount. `Models/Common/TaxDocumentItemPrices.cs` odděluje native TotalVat/TotalWithoutVat/TotalWithVat od jejich Hc protějšků; jednotlivé DDPP položky používají tento typ. `IssuedTaxDocumentPostModel` přijímá PaymentId, DateOfIssue a Items, nikoli libovolný vlastní kurz. `AccountProformaInvoicesPutModel` přijímá pouze seznam ProformaIds. Tyto DTO dokládají oddělené částky a vazby, nikoli jejich výpočet, volbu data nebo databázový model.
- [Aktuální informativní e-Sbírka ZDPH od1.1.2026](https://e-sbirka.gov.cz/sb/2004/235?odkazId=38092765), §4 odst.8, ověřeno18.10.2026 přes indexovaný text oficiálního zdroje: konverze váže kurz na vznik povinnosti přiznat daň/plnění. To není doklad, že datem je vždy vystavení proforma faktury. Úplné dynamické zobrazení zákona neposkytlo čitelný text; historické výsledky §37a označené2024/2016 nelze vydávat za kompletní aktuální ověření.
- [GFŘ2404/24/7100-30116-050822](https://financnisprava.gov.cz/assets/cs/prilohy/d-seznam-dani/INFORMACE-GFR_k_par37a_ZDPH-zaklad_dane_pri_zmene_sazeb.pdf),2024, rozlišuje vypořádání a změny sazeb. Není ověřeným příkladem více FX kurzů a jejich zaokrouhlení.

Nebyl proveden přihlášený experiment iDokladu ani API Recount; syntetické číselné příklady níže jsou výhradně demonstrační matematika.

## Historický audit kódu před implementací

`advance_settlement::configure` požaduje stejnou měnu i přesnou strukturální rovnost `{rate,units}` všech záloh. Snapshot Settlement ukládá pouze jeden kurz první zálohy; jednotlivé Source jej neobsahují. Doménová validate a infra kontrola při issue vyžadují stejný kurz finálního návrhu. Ani ekvivalentní zápisy25/1 a2500/100 nyní neprojdou.

`tax_receipt::build` kopíruje sales.invoice, datum zdanění nastaví na paid_on, ale kurz nechá ze zálohové faktury. Výsledný DDPP snapshot tedy obsahuje kurz zálohy, nikoli nezávisle potvrzený kurz daňového dne. TaxDeduction v následném vyúčtování z DDPP zachová id/number/native Totals, ale ztratí jeho vlastní kurz a rozhodná data.

`advance_settlement::apply` odečítá přesné native DDPP skupiny a počítá native payable = původnípayable + rounding_adjustment − credited. Současný tax_report převede native VAT konečné faktury po odpočtech jejím jedním kurzem a DDPP jejich vlastním uloženým kurzem. Při stejných kurzech je to konzistentní s vlastním původním modelem. Po odstranění guardu by vznikl neověřený algoritmus bez explicitních CZK daňových snapshotů.

Dobropis vychází z původních uložených native řádků a původního kurzu; podporu různých daňových kurzů nelze dokončit pouze změnou finálního vyúčtování. Bankovní kurzové vypořádání je oddělený aktuální modul: jeho hodnoty ani kurz se nesmějí automaticky stát DDPP daňovým kurzem. Starší text P19-01i o zákazu některých bankovních vazeb je historický stav; aktuální bankovní rozšíření tímto auditem nejsou rušena.

## Invarianty pro budoucí plátcovský model

1. Native pohledávka a úhrada mají měnu dokladu. Přepočet do CZK nesmí měnit částku, kterou klient skutečně platí, ani vytvářet druhou úhradu.
2. Každá záloha, konkrétní platba/DDPP a konečné plnění musí mít rozlišitelné vlastní kurzové údaje a rozhodné datum. Dnes stejné hodnoty nejsou důvodem sloučit jejich význam.
3. DDPP zachová nezměněné native Totals a samostatný historický daňový CZK snapshot po sazbách. Pozdější kurz ani oprava kontaktu je nepřepočítají. Chybějící snapshot není nula.
4. Finální doklad potřebuje oddělené CZK daňové plnění a odpočty přesných CZK DDPP, vedle native finančního Totals. Pouhé přidání druhého exchange_rate ke stávajícím Totals nestačí.
5. Konverzní/zaokrouhlovací algoritmus je verzovaný a doložený včetně součtové alokace haléřů. Každý odpočet nese původní DDPP id, datum, kurz, sazbu a konkrétní odečtené CZK částky.
6. Kurzový rozdíl banky, obchodní ocenění pohledávky/výnosu a daňový rozdíl nejsou automaticky jedna veličina. Samostatné rozdíly nesmějí přepisovat daň DDPP ani native credited.
7. Zálohu/DDPP lze použít pouze jednou. Reservace, revize, firma, členství, nejnižší datum, audit, atomický issue a UUID replay zůstávají společné REST/MCP/UI.
8. Oprava/dobropis musí spotřebovávat přesnou původní CZK daňovou kapacitu podle doloženého pravidla. Oprava nesmí historický odpočet nebo haléř použít podruhé.
9. Staré vystavené doklady/PDF a JSON snapshoty se nepřepisují ani nebackfillují dnešním kurzem. Legacy výpočet zůstává rozpoznatelnou verzí; nové schéma není důkazem správnosti historického daňového kurzu.
10. Dokud chybí doložené pravidlo, různé skutečné daňové kurzy plátce musí být odmítnuté při preview i zápisu/issue, ne pouze skryté v UI.

## Proč nestačí odebrat rovnost kurzu

Čistě syntetický příklad: DDPP100+21EUR při25CZK/EUR; konečné native plnění200+42EUR při26. Native doplatek121EUR je jasný. Dvě různé hypotézy pro CZK daň však dávají rozdíl:

| Hypotéza (nikoli právní expected výsledek)                                    | Zbývající základ / VAT CZK    |
| ----------------------------------------------------------------------------- | ----------------------------- |
| Převést celé konečné plnění novým kurzem a odečíst historicky převedený DDPP  | 5200−2500=2700 / 1092−525=567 |
| Zachovat historický DDPP a novým kurzem převést pouze native zbývající plnění | 100×26=2600 / 21×26=546       |

Rozdíl100/21CZK není chyba desetinné aritmetiky. Je to rozhodnutí o právní konstrukci základu, které se nesmí domyslet z DTO. U více částečných plateb, sazeb a záporného vypořádání navíc chybí doložené pořadí/alokace a pravidla oprav. Nepoužít ani jednu hypotézu jako implementační default bez důkazu.

## Implementovaný omezený rozsah (18. 10. 2026)

Vlastní návrhová rozhodnutí Open Faktury, nikoli numerická parita reference:

- Neplátce bez DDPP: stejná měna všech záloh; rozdílné kurzy jsou zachované samostatnými obchodními snapshoty `Source.valuation`. Native credited/payable a konečné plnění se nezmění. Period report plnění převádí kurzem konečného dokladu; úhrady a bankovní rozdíly zůstávají ve svých modulech. Žádná nová DPH numerika se tím nezavádí. UI zobrazuje původní kurzy zdrojů, zamyká měnu a dovoluje upravit kurz konečné neplátcovské faktury. Původní zdrojové doklady se nemění.
- Přesně ekvivalentní kurzové zápisy se porovnávají v Decimal pomocí křížových součinů rate1×units2=rate2×units1 po stávající validaci. Původní řetězce se nekanonizují a nevzniká podpora různých ekonomických kurzů. Toto odstraňuje technickou strukturální nerovnost, ne daňové omezení.
- Plátce: nové zdroje zachovávají kurzovou provenienci, ale skutečně rozdílné kurzy nadále odmítá společná doména při configure, preview i issue. Samostatné CZK daňové snapshoty a jejich nový algoritmus implementované nejsou. Ekvivalentní poměry tento guard neobcházejí.

Implementace nemění native odpočty ani historické snapshoty. DB regrese vyúčtování ověřila osm scénářů včetně neplátcovských zdrojů 25/24 a konečného kurzu 26, rezervací, replay a odmítnutí skutečně rozdílného plátcovského kurzu; doménové testy ověřily přesnou ekvivalenci poměrů. Úplná podpora různých kurzů záloh/DDPP/plátcovského vyúčtování zůstává otevřená kvůli výše uvedeným nedoloženým pravidlům. Browserová regrese `tests/e2e/advance-fx.spec.ts` následně prošla 4/4 (CS/EN, desktop/mobile; 24,0 s). Ověřila reálnou izolovanou DB, úpravu konečného kurzu na 26, uložení/reload/vystavení, native doplatek 100 EUR a nezměněné zdrojové kurzy 25/24. Vizuálně byly zkontrolovány CS mobile a EN desktop screenshoty; nejde o ověření plátcovského algoritmu.
