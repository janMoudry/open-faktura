---
name: faktura-implementation
description: Implementuje specifikovaný modul Open Faktury a ověřuje funkční shodu napříč webem, API a MCP. Použij při vývoji funkcí tohoto repozitáře.
---

# Implementace podle specifikace

Od kořene repozitáře načti `docs/IMPLEMENTATION_PLAN.md`, dotčenou specifikaci a část `docs/DATA_MODEL.md`. Při chybějících pravidlech doplň nejprve průzkum podle `idoklad-research`; při změně dat použij `faktura-domain`. Tyto skills jsou sourozenecké adresáře pod `.agents/skills/`.

Vyber požadavek se stabilním ID a explicitními akceptačními scénáři. Pokud chybí rozhodnutí ovlivňující výpočty či životní cyklus, odděl tuto blokovanou část a pokračuj nezávislou prací. Neoznačuj domněnku jako dosaženou shodu.

Implementuj doménovou operaci jednou; web, API a MCP jsou adaptéry. Zápisy potřebují firemní oprávnění, transakční konzistenci a odpovídající ochranu proti opakování. Externí odesílání řeš odděleně od vystavení dokladu.

Ověř relevantní referenční scénáře, izolaci firem a podle změny souběh či opakování. Test očekávaných částek nesmí pouze znovu použít tentýž kalkulátor. Funkční shodu označ za ověřenou až s referenčním důkazem a úspěšným testem; technické testy samy nestačí.

Aktualizuj stav požadavku, odkazy na testy a známé odchylky. Závěrem uveď dokončené chování, provedené kontroly a zbývající mezery. Bez testovacího prostředí neslibuj ověřenou integraci.
