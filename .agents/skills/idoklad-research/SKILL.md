---
name: idoklad-research
description: Zkoumá referenční chování iDokladu, veřejné SDK, API a exporty pro Open Fakturu a převádí důkazy do specifikací. Použij při reverse-engineeringu a řešení neověřených pravidel.
---

# Průzkum iDokladu

Kořen repozitáře najdi přes `git rev-parse --show-toplevel`. Přečti `docs/research/EVIDENCE.md` a pouze relevantní část `FEATURES.md` a `docs/specs/`.

Vyber konkrétní otázku a scénář z `docs/research/SCENARIOS.md`. Nejprve využij existující důkazy, potom oficiální nápovědu, verzovaný SDK nebo dostupnou aplikaci/API. SDK anotace označ jako klientský kontrakt, ne jako prokázané serverové chování. Nikdy neodvozuj interní databázové tabulky z DTO.

Každému novému zjištění dej stabilní ID E-NNN, datum, přesný zdroj, prostředí/verzi a rozsah závěru. U experimentu zachyť anonymizovaný vstup, nastavení agendy, kroky a skutečný výsledek. Bez přístupu do testovací aplikace připrav reprodukovatelný scénář a nech výsledek neověřený.

Rozpory mezi SDK, nápovědou a aplikací ulož jako otevřenou otázku. Syntetické příklady nesmí mít status pozorování. Kopíruj jen nezbytné fragmenty kontraktu; neukládej celý cizí repozitář.

Výstupem je aktualizovaný důkaz a navázaný požadavek/scénář, nikoli jen seznam odkazů. Při změně pravidla označ dotčené specifikace a testovací případy.
