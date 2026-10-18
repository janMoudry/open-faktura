# Vzhled nových dokladů (P20-05a/b)

18. 10. 2026. Podklad: FEATURES §2/14 a P20-05; konkrétní chování je návrhem
              Open Faktury, nikoli ověřenou shodou s iDokladem.

Vlastník aktivní firmy spravuje logo, podpis/razítko a zápatí samostatně v CS/EN.
Čtenář/editor mohou nastavení číst, změna vyžaduje owner a aktuální revizi.
Nastavení má UUID request_id, audit vstupu/výsledku a bezpečný replay, kterému
vždy předchází aktuální autorizace. Přístup instalačního správce bez členství
nestačí. Prázdná konfigurace má revizi 0.

Obrázky jsou pouze PNG/JPEG, nejvýše 100 000 dekódovaných vstupních bajtů a
1024 × 1024 pixelů. SVG, externí URL a další formáty nejsou podporované.
Rozměry se ověřují před dekódováním obrazu; PDF pracuje pouze s dekódovanými
pixely, bez externího načítání. Zápatí má nejvýše 500 znaků v každém jazyce.

`company_document_style.data` se kopíruje do `invoice_draft.document_style`
ve společné `issue_in_tx`, tedy také pro pravidelné faktury a všechny REST/MCP
cesty. Vlastník nemůže změnou konfigurace přepsat starý snapshot. Existující
DB ochrana vystavených faktur zahrnuje i nový sloupec. Logo se vykreslí před
hlavičkou, podpis a zápatí za obsahem. Zápatí vybírá uložený jazyk dokladu,
nikoli aktuální jazyk UI. Obrázky zachovávají poměr stran; logo do 40 mm,
podpis do 35 mm. Původní PDF cache se nikdy nepřepisuje. Staré vystavené
faktury mají NULL snapshot a zůstávají bez dodatečného odhadnutí brandu.

UI nastavení firmy v CS/EN nabízí omezený upload, náhled a odebrání obrázku,
obě zápatí, chyby, uložení a ochranu rozpracovaných změn při navigaci.
`document-style.openapi.json` a generovaný klient/MCP sdílejí StyleStore.
Test `document_style.rs` ověřuje role, revize/replay, odmítnutí SVG, historický
snapshot a cache při pozdější změně stylu a DB neměnnost.

Migrace 202610180082 je aditivní, před upgradem zastavit starý server (SELECT *).
Migrace 202610180084 přidává uložený vzhled nabídkám/zálohám a pokladním
dokladům. Sales create zachytí styl; obsahová editace nabídky vytvoří novou
revizi s aktuálním stylem, pouhé archivování nebo platba vzhled nemění. Záloha
vykreslí PDF ze stylu dané revize. Společný cash insert zachytí styl také pro
automatické doklady. Historické sales/cash řádky mají NULL; stávající PDF
v odeslaných zásilkách a pokladní cache se nepřepisují. Sdílený test ověřuje
uložený styl nabídky/zálohy i první pozdější render pokladny po změně nastavení.
Migrace 202610180085 zachytí vzhled také při vystavení DDPP a dobropisu.
Jejich uložené PDF vzniká ve stejné transakci ze zachyceného vzhledu. Staré
řádky mají NULL a původní PDF se nepřekresluje. Integrační test ověřuje nově
vystavené značkové doklady i byteovou neměnnost dřívějších originálů po změně
nastavení. Širší tiskové předvolby mimo uložený jazyk a tento vzhled nejsou
doložené; nevydáváme je za uzavření všech položek P20-05.

Transparentní PNG se bezpečně skládá na bílou barvu papíru před předáním
rendereru, který alfa kanál nepodporuje; SVG se nevykresluje. PDF integrace
používá Poppler pdftotext (testovací závislost), protože lopdf 0.26 neumí
správně interpretovat Unicode mapy vloženého Noto fontu.

Výchozí přítomnost cen na dodacím listu (`delivery_note_prices`, false) je
společnou tiskovou předvolbou pro nově vyžádané pomocné výstupy. Nemění již
uložené finanční PDF; jednotlivé stažení může volbu přepsat. Pravidla a
veřejný podklad jsou v [tiskových výstupech](document-outputs.md).
Výchozí false se při serializaci vynechává kvůli kompatibilitě starých
příkazů a jejich UUID replay; klient doplní chybějící pole na false.

## P20 — Text před položkami (18. 10. 2026)

E-028 dokládá samostatný text před/za položkami v nápovědě nabídek
(https://www.idoklad.cz/podpora/karta-cenove-nabidky, aktualizace 3. 9. 2026,
veřejná dokumentace přečtená 11. 10. 2026). Firemní výchozí text a limity níže
jsou vlastní návrh; individuální editace textu každého dokladu tím nevzniká.

Owner nastaví `before_items_cs`/`before_items_en`, prostý víceřádkový text
nejvýše 1000 Unicode znaků na jazyk. Řídicí znaky mimo CR/LF/tab jsou odmítnuté.
Prázdná pole mají zpětně kompatibilní default a při serializaci se vynechají,
aby starý příkazový replay neměnil svůj JSON. Nastavení používá stejnou revizi,
UUID, firemní oprávnění a existující snapshot jako ostatní vzhled.

Renderer faktur, nabídek, záloh, DDPP a dobropisů vloží uložený text v jazyce
dokladu před první položku, za nadpis a informaci o cenách/kurzu. Není to interní
poznámka, HTML ani nový daňový údaj. Cash renderer bez seznamu položek jej
nepoužívá. Pomocné dodací listy/RTF/potvrzení plateb tuto předvolbu nepřebírají.
Úprava nastavení nemění staré snapshoty, uložená PDF ani odeslané přílohy;
bez migrace, žádné doplňování historických hodnot.

Datum tisku nebylo v uložených podkladech doloženo. Tato dodávka jej nezavádí:
aktuální čas při opakovaném renderu by měnil historický obsah originálu.
Nejde o uzavření všech tiskových předvoleb P20.

DB test `document_style` ověřuje limit/řídicí znaky, výběr anglického textu,
pořadí před první položkou a neměnnost PDF po změně nastavení. UI regrese
`product-administration.spec.ts` doplňuje uložení/reload obou jazykových textů.

Ověření dodávky: `cargo test -p faktura-server --test document_style --
--include-ignored --test-threads=1` s izolovanou DB prošel 4/4 (31,24 s),
TypeScript a formátování prošly. Root ověřil product-admin UI 12/12 včetně
uložení/reload textů CS/EN na desktopu a mobilu. PDF bylo ověřeno textovou
extrakcí a byteovou neměnností; samostatná vizuální kontrola víceřádkového
úvodu v PDF nebyla provedena. Fyzický tisk není ověřen.
