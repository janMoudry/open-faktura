# P20 — koš návrhů faktur

Rozhodnutí Open Faktury, 18. 10. 2026; nejde o doloženou shodu mazání iDokladu.
Koš je vratný a nevystavuje ani nestornuje účetní doklady. Vystavené faktury
zůstávají chráněné původními pravidly a opravují se dobropisem.

Editor/vlastník profilu může přesunout nevystavený návrh do koše nebo jej
obnovit. Zápis vyžaduje revizi a UUID požadavku, opakování nemění stav podruhé.
Koš má audit a zachovává obsah, přílohy i vazby; trvalé odstranění není nabízené.
Návrh s rezervovanou zálohou nebo vazbou na nabídku/pravidelný výskyt se do koše
nepřesune, dokud není návaznost vyřešená. Žádné rezervace se skrytě neuvolňují.

Návrhy v koši nejsou v běžném seznamu ani exportu návrhů a nelze je změnit,
vystavit nebo zkopírovat přes standardní detail. Seznam koše je profilový,
stránkovaný a přístupný i čtenáři; obnovovat smí editor/vlastník. Obnova vrací
původní obsah a další revizi, konečné číslo se nepřiděluje.

Současné přesunutí a vystavení zamyká stejný řádek: uspěje pouze jedna cesta.
Databázová ochrana nepovolí vystavit skrytý návrh ani obejít již vystavený doklad.
Testy pokrývají role/firmy, replay, konflikt revize, návaznosti, běžný seznam a
obnovu, souběh a REST/MCP. UI používá CS/EN a potvrzovací dialog.
