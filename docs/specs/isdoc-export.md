# P19-03a — Export vystavené faktury ISDOC 6.0.2

Důkaz E-080 v docs/research/EVIDENCE.md.

Primární podklad: https://isdoc.cz/6.0.2/ a
https://isdoc.cz/6.0.2/doc/isdoc.html, verze 6.0.2 z 23. 3. 2022,
přečteno 18. 10. 2026. Oficiální XSD je uložené s původní licencí v
`tests/fixtures/isdoc/isdoc-invoice-6.0.2.xsd` (SHA-256
fb5de36fe7b5517acb8aa0c7950d91f8e28910e63b8cdcaaf7db0b0f4f0420bb).
Jde o staticky přečtený kontrakt, nikoli ověření importu do konkrétní verze
Money/Pohoda ani pozorování iDokladu.

Read-only GET `/api/companies/{company}/invoices/{id}/isdoc` a MCP
`invoice_isdoc` používají společný export nad uloženou vystavenou fakturou.
Stejná profilová oprávnění jako detail, včetně role viewer. Návrh vrací
konflikt; chybějící členství nebo doklad nenalezeno. Odpověď je attachment
`application/xml`, `no-store`, `nosniff`; MCP vrací base64, media_type a filename.
Stažení nic nevystavuje, nepřepočítává, neodesílá ani nemění PDF cache.

Mapování: DocumentType=1, číslo a data dokladu, vlastní kopie dodavatele a
odběratele, uložené řádky a daňové souhrny. Efektivní jednotkové ceny po slevách
jsou doplňující prezentace na 8 desetinných míst; řádkové a celkové částky se
berou přesně z neměnného snapshotu, nikoli z nové kalkulace. Množství a jednotky
se zachovávají, XML text je escapovaný. Platební metoda, splatnost, účet, IBAN
a variabilní symbol pocházejí z kopie platebních údajů. PaidAmount=0 znamená,
že výměnný doklad neobsahuje pozdější historii úhrad; jeho původní PayableAmount
se při následné úhradě nemění.

CZK: CurrRate=RefCurrRate=1, žádné finanční elementy Curr. Cizí měna:
ForeignCurrencyCode a uložené rate/units v CurrRate/RefCurrRate, původní
částky ve finančních elementech Curr a přesný desetinný přepočet do CZK v
lokálních elementech. XSD AmountType neomezuje počet desetinných míst;
export lokální přepočet nezaokrouhluje na haléře a nemění účetní přehledy.
Žádný dotaz na dnešní kurz. Standard nemá LineExtensionTaxAmountCurr;
daň na řádku je v lokální měně.

Omezení jsou explicitní: faktura s odpočtem záloh je odmítnutá do doplnění
TaxedDeposits/referenčních vazeb; žádný podpis, ISDOCX/ISDOC.PDF ani potvrzení
importu cílovým účetním programem. Dobropisy, zálohové a DDPP mají jiné typy
dokumentů a tento endpoint je nepřeznačuje na obyčejnou fakturu.

Akceptace a regrese: `crates/server/tests/isdoc.rs`, oficiální XSD přes
`xmllint --nonet`, původní částky a escapování, ČJ/UTF-8, foreign rate/units,
neplátce, snapshot nezávislý na dnešní úhradě, autorizace REST/DB a odmítnutí
návrhu. Syntetické scénáře nejsou skutečný účetní export.
