# P19-02b — Ověření DIČ EU ve VIES

Důkaz E-082 v docs/research/EVIDENCE.md.

Primární zdroje Evropské komise, přečteno 18. 10. 2026:
https://ec.europa.eu/taxation_customs/vies/?lang=en (UI v7.3.0-HF1),
https://ec.europa.eu/taxation_customs/vies/checkVatService.wsdl
(SHA-256 a5de093d23585cb3ff4d6c2a4367ba6003de48b428e30bb99548f2d184829279).
Staticky přečtený SOAP checkVat kontrakt; žádný živý dotaz na obchodníka.

Read-only GET `/api/companies/{company}/vies?vat_number=…`, MCP vies_lookup
a ruční akce v detailu kontaktu slouží k ověření pro přípravu faktury.
Členství před sítí, i viewer smí číst. Žádné hromadné vytěžování databáze,
změny kontaktu, automatická volba daňového režimu ani rozhodování o odpočtu.
CZ používá v UI český Registr DPH, ostatní podporované předpony VIES.

VIES podporuje EU včetně EL a Severního Irska XI; GB se neposílá, neboť
Komise ukončila tuto službu pro GB od 1. 1. 2021. Není dovozována vazba
mezi VAT předponou, měnou a zemí bankovního účtu. Identifikátor se trimuje
a převede na uppercase, nepřepisuje se uložený kontakt. Vstupní povolené
znaky čísla jsou ASCII alfanumerické a +/* kvůli národním tvarům; žádný
nezjištěný národní checksum se nevydává za ověření registrace.

Pevný HTTPS endpoint
`https://ec.europa.eu/taxation_customs/vies/services/checkVatService`,
SOAP checkVat, žádná URL od klienta či přesměrování. Timeout 10 s, odpověď
nejvýše 256 KiB, nejvýše dvě současná volání a rozestup 500 ms v instanci.
Síť neprobíhá v databázové transakci. Parser omezuje hloubku/uzly, odmítá
DTD/entity, kontroluje přesné namespaces a jedinou odpověď, shodnou zemi
a číslo, boolean valid a datum. Fault, timeout a nedostupný členský registr
jsou nedostupnost, nikdy náhradní valid=false.

Výsledek obsahuje platnost podle VIES, datum dotazu podle registru,
čas přijetí a případně název/adresu, pokud je členský stát poskytne.
Nejde o český údaj spolehlivosti ani potvrzení správného režimu DPH.
Údaje se zobrazují jako text, neinterpretují se jako HTML či pokyny.
Testy `vies.rs` ověřují synteticky valid/invalid, technické selhání,
namespace/identitu, XI/GB a autorizaci před injektovaným providerem.
