# Anonymizace adresářového kontaktu

## Doložené chování a záměrná odchylka

Veřejný článek iDokladu z 16. 5. 2018
[popisuje anonymizaci](https://www.idoklad.cz/blog/gdpr-v-idokladu-zadarmo-prehled-funkci-kterymi-vam-idoklad-usnadni-gdpr)
jako nevratnou operaci po výběru subjektu a přehledu jeho dokladů; zmiňuje
údaje v adresáři i na dokladech. Přečteno 18. 10. 2026, veřejná dokumentace,
nikoli aktuálně pozorované UI/API ani doložené serverové podmínky.

Open Faktura záměrně zachovává historické finanční snapshoty a původní PDF.
Rozsah je proto anonymizace samostatného záznamu adresáře a jeho
adresářové historie. Není to globální výmaz všech kopií údajů a funkce
nepotvrzuje splnění právních povinností. Historické doklady, přílohy,
importní originály, již vytvořené exporty a zálohy zůstávají nedotčené.

## Příkaz a invarianty

Pouze aktuální vlastník firmy smí po výslovném potvrzení anonymizovat její
kontakt. Příkaz obsahuje UUID a očekávanou revizi; nesmí obsahovat původní
jméno ani jiný původní osobní údaj. UI pro potvrzení vyžaduje zadání jména,
které porovnává pouze lokálně a neukládá do auditu.

Jedna transakce nahradí `contact.data`, `contact.initial_data` a všechna
`contact_audit.data` neutrálním kontaktem bez adresy, identifikátorů, banky,
poznámek, e-mailů a kontaktních osob. Kontakt zůstane pod původním ID,
archivuje se a přejde do konečného anonymizovaného stavu. Běžná editace ani
obnova nesmí obnovit dřívější údaje; nové adresářové vazby potřebují nový
nebo jiný neanonymizovaný záznam. Stávající vazby dokladů nadále ukazují na stejné ID,
ale jejich vlastní historické kopie se nemění.

Audit zachová autora, okamžik, čísla revizí a druhy dřívějších operací,
obsahuje však jen anonymizovaná data. Nová auditní událost eviduje samotnou
anonymizaci bez původních údajů či jejich otisků. Replay vždy znovu ověří
aktuální členství a vlastnictví; shodné UUID se shodným příkazem nesmí
vytvořit druhou revizi. Starý create replay s původními údaji je konflikt.

## Implementace a ověření

Migrace `202610180100_contact_anonymization.sql` přidává konečný stav,
příkazový replay a DB ochranu proti obnovení obsahu. Při upgradu zastavit
starý server; nový proces provede migrace. Žádné kontakty se při migraci
neanonymizují. Auditní metadata původních revizí zůstávají; anonymizace je
výslovně jedinou operací, která nahrazuje jejich adresářové datové payloady.

REST `POST /api/companies/{company}/contacts/{contact}/anonymize` a MCP
`contact_anonymize` sdílejí `AnonymizationStore`. Vstup `Command` má
`request_id`, `revision`, `confirm=true`; nadbytečné klíče jsou odmítnuté.
Neutrální uložený název `— #ID` neobsahuje původní název ani jazyk UI.
Příznak `Contact.anonymized` určuje lokalizovaný popisek v seznamu a detailu,
nikoli porovnání názvu. Export zachová tento neutrální technický zástupný text.

`pnpm test:contact-anonymization` používá pouze izolované syntetické DB.
Pět integračních testů ověřuje owner/editor/viewer/instalačního správce,
chybné potvrzení, revizi a UUID, opakování po změně role, více souběžných
příkazů a souběh s editací bez deadlocku, odstranění starých adresářových
payloadů a neměnnost vystaveného JSON/PDF. REST/MCP mají stejný replay;
REST navíc ověřuje relaci a Origin. Regrese `pnpm test:contacts` prošla.
CS/EN scénář `tests/e2e/contacts.spec.ts` doplňuje neplatné zadání jména,
zrušení potvrzení, explicitní operaci, reload a chybějící akce pro viewer
nebo konečný anonymizovaný záznam.

Části mimo adresář (historické finanční snapshoty, přílohy, importní
originály, staré exporty a zálohy) tato operace nemaže ani neprohledává.
Rozhodnutí, zda je takový rozsah vhodný pro konkrétní požadavek, není
právním závěrem aplikace.


## Nové vazby a zachování již uložených kopií

Nový návrh faktury, nabídka, šablona, záloha či pravidlo opakování nesmějí
přiřadit anonymizované ID kontaktu. Stejně se odmítne změna existujícího
dokumentu z jiného nebo prázdného ID na anonymizované ID. Běžně archivovaný
kontakt zachovává dosavadní pravidla; archivace není anonymizace.

Při úpravě návrhu, prodejního dokumentu, pravidla nebo konfiguraci existujícího
vyúčtovacího návrhu lze ponechat přesně tutéž již uloženou vazbu. Povolení
se odvozuje z identity uloženého zamčeného zdroje, nikoli z tvrzení klienta.
Jeho vlastní snapshot se nenahrazuje vyprázdněnými údaji adresáře a historické
PDF se nemění. Odpojení vazby je možné; pozdější znovupřiřazení již bude novou
vazbou a je odmítnuté. Ověřený replay původního create UUID vrací existující
kopii a nevytváří novou vazbu.

Převod šablony/nabídky vytváří nový návrh, proto přebírané anonymizované ID
nepovoluje. Také další nově generovaný návrh z pravidelné fakturace vyžaduje
neanonymizovanou adresářovou vazbu; chyba nic neodešle a worker ji eviduje
standardním stavem chyby. Uživatel může vybrat jiný kontakt nebo explicitně
vazbu odstranit a pokračovat s nezávislým obsahem dokumentu. Nejde o zákaz
ručně zadaných odběratelů bez vazby na adresář.

Regrese `anonymized_contact_rejects_new_links_but_preserves_existing_owned_snapshots`
ověřuje nové vazby a změnu vazeb napříč draft/sales/recurring, historický
create replay, povolené úpravy stejné uložené identity, odpojení/znovupřiřazení
a zachování dosavadní podpory běžně archivovaných kontaktů.
