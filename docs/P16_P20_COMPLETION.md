# Dokončení P16–P20

Aktivní rozsah potvrzený uživatelem 15. 10. 2026: všechny zbývající části
P16 až P20, včetně backendu, PostgreSQL, REST/MCP, češtiny a angličtiny.
Tento seznam je sledování práce, nikoli prohlášení o dokončení. Autoritativní
rozsah zůstává v IMPLEMENTATION_PLAN.md a FEATURES.md; nic se nevypouští
přesunem mezi etapami.

## P16 — Finance

Podrobná akceptace a stav: [P16_COMPLETION.md](P16_COMPLETION.md).
Dokončeno P16-12: společné platební operace záloh a dobropisů v bance/pokladně.
P16-09a má konfiguraci terminálů i jejich snapshoty na prodejkách.
CZK prodejky a dávkové zaúčtování mají backend/DB/REST/MCP/CS/EN a regrese;
zbývají návazné daňové/skladové funkce a samostatná interní tisková sestava.
Dále zbývají měny, notifikace,
tiskové nastavení a členění DPH. Skutečné bankovní exporty a živé ověření
adaptérů vyžadují doložené vstupy; syntetické testy je nenahrazují.

## P17 — Příjem dokladů

- [x] P17-01: upload, originál, deduplikace, archiv/obnova, auditní historie,
  připojení k existujícímu dokladu a atomický převod do přijaté faktury
  včetně přílohy. PostgreSQL regrese a CS/EN desktop/mobile scénáře.
- [ ] P17-02 Omezený ISDOC 6.0.1/6.0.2 je implementovaný s kontrolou součtů
  a potvrzením do závazků. Zbývají QR, PDF s vloženým ISDOC a další režimy.
- [ ] P17-03 Příjem e-mailů, přílohy, autentizace vstupu, deduplikace a diagnostika.
- [ ] P17-04 Volitelné vytěžení, vlastní nastavení poskytovatele, kontrola návrhu;
  základní tok bez AI, žádné automatické zaúčtování nedůvěryhodného výstupu.

## P18 — Sklad

- [x] P18-01 Skladové karty, počáteční stavy a ruční pohyby s auditní stopou;
  přesnost, role, replay, CS/EN a REST/MCP ověřené.
- [x] P18-02 Explicitní vazby řádků na sklad: vystavení faktury, potvrzený příjem
  nákupu, opravy/storna nákupů a prodejek, množstevní vratky dobropisů.
  Atomické pohyby, replay, limity vratek a profilová izolace; staré doklady
  se nezapojují zpětně odhadem. Záporná zásoba je povolená.
- [ ] P18-03 Inventura, dohledatelné rozdíly a historie jsou implementované.
  P18-03a doplňuje statistiku vybrané karty za období (počáteční/konečné saldo,
  ruční příjmy/výdeje a oddělené úpravy), nezávislou na stránkování.
  P18-03b přidává souhrnný stav k datu, hledání, stránkování a explicitní
  časově platné ruční ocenění CZK s historií a oddělenými neznámými hodnotami.
  P18-03c přidává explicitní pořizovací ceny nákupních příjmů a konzervativní
  průběžný průměr běžných příjmů/výdejů. Rozúčtování oprav/vratek, ceny
  ručních příjmů a samostatné vedlejší náklady zůstávají otevřené. Neznámé
  hodnoty se neodhadují; shoda metody s iDokladem není doložená (E-056).

## P19 — Přehledy, integrace a daně

- [ ] P19-01 Měsíční CZK přehled hodnot dokladů s dohledáním zdrojů je
  implementovaný a ověřený (PostgreSQL, REST/MCP, CS/EN desktop/mobile). Podporuje ceny s/bez DPH, kumulaci od začátku období, meziroční srovnání
  a přechod na loňské zdroje. Zbývají další přehledy z FEATURES.
- [ ] P19-02 ARES/DIČ: dokončení podle FEATURES včetně chybových stavů integrací.
- [ ] P19-03 Účetní exporty po cílových systémech, verzované kontrakty a validace.
- [ ] P19-04 Daňové režimy a formuláře po verzích podle oficiálních podkladů;
  účetní a daňová pravidla nejdřív doložit, neodvozovat z vzhledu reference.

## P20 — Úplnost produktu

- [ ] P20-01 Štítky, koš, obnova a pravidla mazání historických dokladů.
- [ ] P20-02 Zámky po exportu, revize a audit exportů.
- [ ] P20-03 Hromadný tisk, RTF, dodací listy a potvrzení úhrady.
- [ ] P20-04 QR Platba v nových PDF faktur/záloh je implementovaná; ověřeno
  dekódování CS/EN PDF. Zbývají QR Faktura, Platba+F a tiskové preference.
- [ ] P20-05 Brand dokladů, podpis/razítko, jazyk a tiskové předvolby.
- [ ] P20-06 Světlý/tmavý/systémový režim s uložením na účtu je implementovaný
  a ověřený v CS/EN desktop/mobile. Úplný audit přístupnosti a preference
  oznámení zbývají.
- [ ] P20-07 Převod vlastnictví, odchod, odstranění profilu a anonymizace historie.
- [ ] P20-08 Úplný export dat a pokročilé migrační adaptéry.
- [ ] P20-09 Nápověda, provozní dokumentace a aktualizace klientů s vazbami na P11.

Každá položka se uzavírá až po specifikaci, implementaci a ověření. Označení
„odpovídá iDokladu“ vyžaduje konkrétní referenční důkaz; technická regrese sama
shodu nepotvrzuje. Externě neověřené části mají zůstat viditelně otevřené.
