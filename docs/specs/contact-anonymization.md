# Anonymizace adresářového kontaktu

## Doložené chování a záměrná odchylka

Veřejný článek iDokladu z 16. 5. 2018
[popisuje anonymizaci](https://www.idoklad.cz/blog/gdpr-v-idokladu-zadarmo-prehled-funkci-kterymi-vam-idoklad-usnadni-gdpr)
jako nevratnou operaci po výběru subjektu a přehledu jeho dokladů; zmiňuje
údaje v adresáři i na dokladech. Přečteno 18. 10. 2026, veřejná dokumentace,
nikoli aktuálně pozorované UI/API ani doložené serverové podmínky.

Open Faktura záměrně zachovává historické finanční snapshoty a původní PDF.
Navržený rozsah je proto anonymizace samostatného záznamu adresáře a jeho
adresářové historie. Není to globální výmaz všech kopií údajů a funkce
nepotvrzuje splnění právních povinností. Historické doklady, přílohy,
importní originály, již vytvořené exporty a zálohy zůstávají nedotčené.

## Navržený příkaz a invarianty

Pouze aktuální vlastník firmy smí po výslovném potvrzení anonymizovat její
kontakt. Příkaz obsahuje UUID a očekávanou revizi; nesmí obsahovat původní
jméno ani jiný původní osobní údaj. UI pro potvrzení vyžaduje zadání jména,
které porovnává pouze lokálně a neukládá do auditu.

Jedna transakce nahradí `contact.data`, `contact.initial_data` a všechna
`contact_audit.data` neutrálním kontaktem bez adresy, identifikátorů, banky,
poznámek, e-mailů a kontaktních osob. Kontakt zůstane pod původním ID,
archivuje se a přejde do konečného anonymizovaného stavu. Běžná editace ani
obnova nesmí obnovit dřívější údaje; nové obchodní vztahy potřebují nový
adresářový záznam. Stávající vazby dokladů nadále ukazují na stejné ID,
ale jejich vlastní historické kopie se nemění.

Audit zachová autora, okamžik, čísla revizí a druhy dřívějších operací,
obsahuje však jen anonymizovaná data. Nová auditní událost eviduje samotnou
anonymizaci bez původních údajů či jejich otisků. Replay vždy znovu ověří
aktuální členství a vlastnictví; shodné UUID se shodným příkazem nesmí
vytvořit druhou revizi. Starý create replay s původními údaji je konflikt.

## Stav

Jde o doložený návrh rozsahu připravený k implementaci po integračním
checkpointu. Tento dokument sám neznamená implementovanou funkci.
