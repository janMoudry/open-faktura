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
