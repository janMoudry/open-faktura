# P19-02 — Ověření DIČ v českém Registru DPH

Důkaz E-081 v docs/research/EVIDENCE.md.

Primární zdroje, přečteno 18. 10. 2026:
- https://financnisprava.gov.cz/cs/dane/dane-elektronicky/danovy-portal/registr-dph
- https://podpora.mojedane.gov.cz/cs/seznam-okruhu/rozhrani-pro-treti-strany/informace-o-webove-sluzbe-pro-zjistovani-4381
- WSDL https://adisrws.mfcr.cz/adistc/axis2/services/rozhraniCRPDPH.rozhraniCRPDPHSOAP?wsdl
  SHA-256 7e878ee0721507fc91f94e457ecb5ddca5c0ccc750d06035848cc185c35b3385.

Jde o oficiální veřejný a staticky přečtený kontrakt. Dokumentace popisuje
rozšíření o identifikované osoby od 20. 6. 2023 a denní aktualizaci registru.
Živý dotaz na konkrétní firmu ani shoda s iDokladem nebyly provedené.

GET `/api/companies/{company}/vat-registry?vat_number=CZ…`, MCP
`vat_registry_lookup` a ruční akce v detailu kontaktu sdílejí příkaz.
Členství se ověřuje před sítí; i viewer smí číst. Vstup trim/uppercase,
CZ a 1–10 ASCII číslic dle WSDL DICType. Neprovádí se nezdokumentovaný
checksum DIČ. Zahraniční DIČ není posíláno do českého registru.

Operace getStatusNespolehlivySubjektRozsirenyV2, SOAPAction dle WSDL,
pevný HTTPS endpoint `https://mojedane.gov.cz/dpr/axis2/services/rozhraniCRPDPH.rozhraniCRPDPHSOAP`.
Žádné přesměrování, klientská URL ani I/O v SQL transakci. Timeout 8 s,
limit odpovědi 1 MiB, 4 současné požadavky a rozestup 250 ms v instanci.
Parser omezuje XML hloubku/uzly, odmítá DTD/entity, kontroluje namespace,
jednu odpověď, statusCode=0, shodné DIČ a známé enum hodnoty.

Výstup rozlišuje plátce/skupinu/identifikovanou osobu/nespolehlivou osobu/
nenalezeno a odděleně příznak nespolehlivosti ANO/NE/NENALEZEN. NE se v UI
popisuje jako absence příznaku, nikoli důkaz plátcovství. Chybná nebo
nedostupná odpověď je 503, nikdy náhradní „nenalezeno“ či „spolehlivý“.
Vrací datum vytvoření dat registru a čas skutečného dotazu. Zveřejněné účty
mají datum zveřejnění a případného ukončení; ukončený účet není označený
jako aktuálně zveřejněný. Čísla účtů se neodhadují z DIČ.

Výsledek nemění kontakt, plátcovství profilu, historické doklady ani daňový
režim. Nejde o automatické rozhodnutí o ručení, odpočtu či daňovém přiznání.
ARES návrh identifikačních údajů zůstává samostatný existující tok.
Akceptace: syntetické parser testy plus `tests/vat_registry.rs` ověřují
oprávnění a validaci před providerem, REST, nedostupnost a absenci zápisu.
