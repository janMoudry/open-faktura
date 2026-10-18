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

## Rozšíření P17, 18. 10. 2026 — návrhový kontrakt adaptérů

PDF s vloženým ISDOC: lokální extrakce jednoho souboru `.isdoc` ze slovníku
Filespec/EF, typ EmbeddedFile, nezkomprimovaný nebo FlateDecode. Payload má
stejný limit 2 MiB a prochází stávajícím ISDOC parserem a kontrolou součtů.
Více vložených ISDOC je nejednoznačnost a chyba. Originální PDF zůstává
neměnné. Jde o konzervativní podmnožinu klasického PDF, nikoli validátor
PDF/A: inkrementální revize, šifrování, object/xref streams, escapované názvy,
více než 64 kontejnerových značek a nepřímá délka streamu jsou odmítnuty
ještě před načtením parserem. Počet značek se konzervativně počítá i v datech
streamů; odmítnutý dokument lze vždy zpracovat ručně. Dekomprese vložené
přílohy má tvrdý výstupní limit. CPU parsing je mimo async executor.

Podklad formátu: ISDOC 6.0.2, https://isdoc.cz/6.0.2/doc/isdoc.html
(sekce ISDOC.PDF), veřejná dokumentace přečtena 18. 10. 2026; podpůrný
oficiální projekt https://github.com/isdoc/isdoc.pdf. Vlastní limity výše
nejsou pravidly iDokladu a nedokládají shodu všech PDF exportů.

### Autentizovaná e-mailová brána

Migrace 060, `inbox/settings` a `inbox/mail`. Příjem je výchozí vypnutý a
firma jej explicitně zapíná. Externí vlastní brána musí nejdříve přijmout
e-mail a parsovat MIME, pak předat JSON `MailIntake` přihlášenou relací
editora/ownera přes REST nebo ekvivalentní MCP. Není to vestavěný IMAP ani
SMTP server a nepředstírá se kompatibilita s konkrétní poštovní službou.
SMTP odesílací nastavení se pro tento účel nepoužívá. Metadata sender/subject
jsou nedůvěryhodná; API netvrdí ověření původu odesílatele.

Kontrakt obsahuje request_id UUID, Message-ID, sender, subject a 1–10 příloh
v existujícím Upload formátu. Limit celkem 10 MiB, každá příloha 2 MiB;
kontrola skutečného obsahu je shodná s ručním uploadem. Všechny přílohy,
originály, audit a diagnostika přijaté zprávy se zapisují v jedné transakci.
Message-ID je unikátní v rámci firmy; stejná metadata a hashe příloh vrací
původní výsledek i při jiném request_id. Odlišný obsah stejného Message-ID
nebo opětovné použití request_id pro jinou zprávu je konflikt. Chyba celé
zprávy neuloží žádnou přílohu a vrací standardní REST/MCP chybu. Diagnostika
zobrazuje posledních 100 úspěšných zpráv, metadata a počet příloh; neukládá
těla e-mailů, tajemství ani odmítnutá těla. Zpráva nikdy nevytvoří závazek.
Opětovný pokus vypnuté brány je odmítnut i u dřívějšího Message-ID.

### Volitelný poskytovatel vytěžení

Instalační správce nastaví důvěryhodný JSON `INBOX_EXTRACTION_PROVIDERS_JSON`:
objekt názvů poskytovatelů s `url` (HTTPS, bez credentials/fragmentu) a volitelným
`token`. Konfigurace patří do privátního prostředí instalace, nikdy do repozitáře,
logu, DTO ani firemního nastavení. Příklad pouze se syntetickými hodnotami:
`{"own-extractor":{"url":"https://extractor.example.test/extract","token":"replace-in-private-environment"}}`.
Firma zvolí existující název, prázdný název vytěžení vypíná. Žádný URL ani token
nepřichází z klienta. Jde o vlastní integrační kontrakt Open Faktury, nikoli
předem ověřený adaptér konkrétního AI produktu.

Editor explicitním POST `/{id}/extract-provider` odešle `{filename,media_type,base64}`
na daný endpoint. Volitelné tajemství je pouze Authorization Bearer. Provider
musí vrátit přímo `ReceivedData` JSON. Timeout 30 s, redirecty vypnuté, maximální
odpověď 2 MiB. Chybové tělo se nepropaguje do logu ani klienta. Model nesmí
vybrat soukromý kontakt, skladovou kartu ani pořizovací vedlejší náklady.
Data procházejí společnou validací; oprávnění a zapnutí poskytovatele se
kontrolují znovu po síti. Síť neprobíhá pod SQL zámkem. Originál se nemění,
nikdy se automaticky nevytvoří účetní doklad; výsledek je pouze editovatelný
návrh. UI před tlačítkem výslovně uvádí odeslání originálu poskytovateli.

### QR Faktura — lokální návrh z PNG/JPEG

Zdroj: veřejný standard KDP ČR SID 1.0,
https://www.kdpcr.cz/informace/qr-faktura/popis-formatu a
https://www.kdpcr.cz/informace/qr-faktura/integrace-s-qr-platbou,
přečteno 18. 10. 2026. Nejde o pozorování iDokladu. QR Platba SPD sama
neobsahuje účetní rozpis a není návrhem faktury; přijímá se SID nebo SPD
obsahující X-INV s vloženým SID. Společná pole AM/CC/DT/ACC/X-VS se spojí
podle integračního pravidla; konfliktní duplicity se odmítají. Procentové
escapování je dekódováno bez jakéhokoliv načítání URL. Duplicitní klíče,
nepodporované verze, špatné CRC32 (pokud uvedeno), převrácené/režimové typy,
zálohy a neodpovídající součet se odmítají.

Implementovaný rozsah je běžný tuzemský CZK doklad od roku 2024, s explicitními
TB0/T0 (21 %), TB1/T1 (12 %) a NTB (0 %); původní základy a daň se zachovají.
Zdroj aktuálních sazeb: informace GFŘ ke změnám od 1. 1. 2024,
https://financnisprava.gov.cz/assets/cs/prilohy/d-seznam-dani/Informace_GFR_ke_zmenam_sazeb_DPH_od_1_1_2024.pdf,
přečteno 18. 10. 2026. Tento parser neurčuje správnost sazby konkrétního
plnění. Starší sazby, TB2, cizí měny a zúčtování vyžadují ruční doplnění.
AM minus součet rozpisu se přijme jako zaokrouhlení pouze do ±1 CZK.
Chybějící rozpis se nesmí zaměnit za osvobozené plnění.

SID neobsahuje jméno a adresu dodavatele ani jednotlivé položky faktury.
Návrh je proto zachová prázdné, členění tvoří souhrnné řádky z daňových
skupin; MSG je případný popis. Uživatel musí doplnit dodavatele a chybějící
popisy před uložením. Validace skutečného přijatého dokladu se neoslabuje.
Lokální dekodér PNG/JPEG před dekompresí kontroluje rozměry (nejvýše
4096×4096 a 16 milionů pixelů), při více účetních QR odmítne nejednoznačnost.
QR ve vyrastrovaných stránkách PDF se zatím nedekóduje; PDF používá vložený
ISDOC nebo volitelného externího poskytovatele.

### ISDOC cizí měny a shoda vlastního exportu

Běžná faktura může mít CZK jako LocalCurrencyCode a podporovanou měnu
ForeignCurrencyCode s CurrRate/RefCurrRate. Přijatý doklad uloží původní
cizoměnové částky `*Curr` a přesný kurz/jednotky; lokální CZK pole kontroluje
v desetinné aritmetice proti tomuto kurzu bez zaokrouhlení na haléře.
Řádková zahraniční DPH je rozdíl uloženého původního gross a net (ISDOC
řádek nemá LineExtensionTaxAmountCurr). Celkové TaxAmountCurr, PayableAmountCurr
a původní net/gross se musí shodovat s řádky. Zálohy a již uplatněné částky
musí být nulové v obou měnách. Neplatný kurz, nepodporovaná měna, chybějící
cizoměnové částky nebo nesoulad lokálního ekvivalentu jsou chyba.

Syntetický roundtrip test používá skutečný společný ISDOC 6.0.2 renderer
Open Faktury (CZK a JPY, kurz 15.25/100). Ověřuje kompatibilitu našeho
exportu/importu a zachování původních částek; není důkazem numerické shody
s iDokladem. Zálohy/vyúčtování, opravné doklady, RPDP a zahraniční dodavatel
mimo dosavadní rozsah vyžadují další explicitní doménové mapování.

E-mailová brána používá časově omezenou běžnou relaci. Na 401 pozastaví
příjem, obnoví standardní login (pro MCP `mcp-login` a privátní SESSION_FILE)
a zopakuje stejný request_id/Message-ID; cookie nebo token nikdy nevypisuje.
Na 403 ověří členství a explicitní zapnutí. Úspěšné intake jsou v historii,
odmítnutí musí brána provozně evidovat podle chybových kódů bez těl dokladů.

## Akceptační audit proti FEATURES_REFERENCE §7, 18. 10. 2026

Aktuální nativní IMAP včetně samostatně povoleného workeru je specifikován
v [inbox-mailbox.md](inbox-mailbox.md), migrace 062/063. Nativní LOGIN přes
TLS je odlišný od vlastní JSON brány; poskytovatelé vyžadující XOAUTH2
potřebují další autentizační adaptér. Podklady aktuální dodávky E-083–E-085.

| Referenční požadavek | Skutečný stav |
| --- | --- |
| Příchozí adresa agendy, e-mailové přílohy | Firma vybere vlastní instalačně nakonfigurovanou IMAP schránku nebo vlastní gateway. Aplikace sama nezřizuje nové veřejné e-mailové adresy/doménu. |
| Ruční a hromadné nahrání | Výběr více souborů; jednotlivé originály/hash/limity a deduplikace. |
| Přetažení souboru | Dropzone sdílí stejný upload, limity a deduplikaci s výběrem více souborů. |
| ISDOC/QR | Běžné ISDOC 6.0.1/6.0.2 v popsaném subsetu, omezené vložené PDF XML, SID/SPD X-INV v PNG/JPEG. Žádné obecné QR čtení v PDF ani podpisový validátor. |
| AI faktur/účtenek | Volitelný vlastní HTTPS provider kontrakt, vyžaduje konfiguraci adaptéru. Výsledek je validovaný návrh a nic se automaticky nezaúčtuje. |
| Originál vedle editoru | Autentizovaný omezený náhled vedle formuláře: PDF pouze canvas (nejvýše 50 stran), PNG/JPEG canvas po kontrole rozměrů, XML jako escapovaný text. Žádný HTML/aktivní obsah. |
| Ruční kontrola a oprava | Editor přijatých faktur/účtenek; stejná společná validace při uložení. |
| Založení dodavatele z dokumentu | Explicitní potvrzení přes společný kontaktní formulář a createContact; upozornění na shodné IČO. Výsledek připojí kontakt k návrhu. |
| Rozdíly proti kontaktu | Porovnání údajů dodavatele/adresy/účtu s kontaktem; uživatel ponechá snapshot nebo výslovně převezme kontaktní údaje. |
| Faktura/účtenka/příloha existujícího dokladu | Hotové společné transakční příkazy a kontrola rolí. |
| Stav zpracování | Pending/processed, archiv/obnova a historie. |
| Připojení/odpojení dokumentu | Připojení je hotové; odpojení uložené inbox vazby zatím chybí. |
| Upozornění na nové dokumenty | Historie příjmu je hotová; aktivní uživatelské oznámení zatím chybí. |
| Předání originálu účetnímu | Stažení originálu je hotové; zvláštní přeposlání e-mailem účetnímu zatím chybí. |

Tento audit neoznačuje celý P17 ani univerzální podporu všech formátů jako
hotové. Výše chybějící body jsou skutečný rozsah, nikoli přejmenované
předpoklady nebo nahrazení jinou funkcí.
