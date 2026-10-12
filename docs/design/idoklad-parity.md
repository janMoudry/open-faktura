# Shoda s iDokladem — pracovní matice

Aktualizováno 11. 10. 2026. Reference E-021 až E-023, živá syntetická agenda
v Chrome a uložené screenshoty. Monochromatický brand Open Faktury byl znovu
potvrzen uživatelem. Manrope a Lucide zůstávají; logo, obchodní obsah a tarify
nepřebíráme. Technický test není důkaz úplné funkční ani pixelové shody.

| Oblast | Implementováno v Open Faktuře | Zbývající rozdíl |
| --- | --- | --- |
| Rám aplikace | Tmavý vnější rám, světlý obsah, pevná navigace, rozbalené aktivní skupiny, oválné akce, kompaktní desktop | Horní přepínač profilů a avatar zůstávají podle dřívějšího zadání; chybí schování menu |
| Seznamy faktur a kontaktů | Hledání, filtry, URL stav, sloupce, řádkové akce, stránkování | Chybí uživatelské sady sloupců, hromadné akce a výběr řádků |
| Faktury | Detail, editor návrhu, vystavení, PDF, ruční úhrady | Návrh je samostatný stav; neověřeny a nepokryty všechny účetní varianty reference |
| Kontakty | Detail/editor se stejnými sekcemi a labely, dodací adresy, ARES, archivace | Role dodavatele a obecná záložka souvisejících dokladů chybí; dostupný odkaz na faktury odběratele |
| Nastavení profilu | Stejné záložky a rám čtení/úpravy, horizontální pole, vlastní API a oprávnění | Nastavení banky/číslování zůstává v profilu; chybí logo a podpis a správa členství přes UI |
| Účet | Funkční jméno/e-mail/heslo, CS/EN | Stále dialog; reference má vlastní stránku se dvěma záložkami; chybí 2FA |
| Přehled | Skutečné součty úhrad, pohledávek a počty dokladů | Chybí časové grafy, nákupy, stáří závazků a limit DPH; žádná demonstrační čísla nejsou doplněna |
| Import | Existující P09 průvodce a databázový zápis | Předvolby služeb nejsou potvrzené adaptéry skutečných exportů |
| Další moduly | Viditelně nedostupné položky navigace | Náklady, sklad, zálohy, dobropisy, bankovní párování a další moduly nejsou implementované |

## Ověření

Výsledky tohoto průchodu doplněny do `reference-audit-2026-10-11.md`.
Screenshoty Open Faktury používají výhradně syntetické E2E údaje; uživatelská
fakturační agenda nebyla během redesignu měněna. Mobilní layout zůstává vlastní
responzivní adaptací, nikoli potvrzenou kopií mobilního iDokladu.

## Rozšíření rozsahu a ceník (11. 10. 2026)

Uživatel potvrdil i všechny dosud chybějící moduly; jde o aktivní rozsah P10–P20,
nikoli pouze vizuální úpravy. P12-01 přidává první novou schopnost: ceník s
výběrem do faktury. E-024/snímky 20–22 dokládají referenční pole a členění.

Ověřené UI Open Faktury: kompaktní seznam, hledání název/kód/EAN, detail/úprava,
archiv/obnova, ceny a výchozí množství, výběr do faktury. Rozdíly: archiv a revize
jsou náš návrh; chybí sledování skladu/počáteční pohyb (P18), jiné měny (P12-04),
referenční uložení/validace a plátcovské scénáře zatím nejsou doložené. Desktop
stále není pixelová kopie; uchováváme náš appbar, font, ikony a monochromatické barvy.


## Šablony a nabídky (P12-02/P12-03)

E-025/E-026 doplněny skutečným uložením referenční šablony a nabídky. Nové
monochromatické registry sdílejí tabulky a stránkování. Detail ukazuje čitelné
údaje, úprava zachovává stejné pořadí sekcí, společný editor položek a součty.
Šablona předvyplní fakturu/nabídku. Převod nabídky otevře navázaný návrh; jeho
vystavení používá existující P07. Archiv, uložený návrh a deduplikace jsou naše
pravidla. QUOTE-DELIVERY-01 přidává vlastní monochromatické PDF, tiskový náhled,
e-mail s přílohou a historii u nabídky podle E-027. Veřejné odkazy/tracking,
záloha, vlastní přílohy, štítky, sady sloupců ani hromadné akce nejsou implementované.
Mobil používá karty historie; jde o vlastní responzivní adaptaci.
Snímky `sales-quote-{cs,en}-{desktop,mobile}.png` a `sales-list-…` pocházejí
z izolovaných E2E databází; skutečná uživatelská agenda se neměnila.
