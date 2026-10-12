# iDoklad — referenční screenshoty 11. 10. 2026

Chrome, viewport 1680 × 906, plné stránky; JPEG. Samostatný testovací účet,
syntetický neplátce DPH bez IČO, CZK. Žádné skutečné doklady uživatele.
Doklad 20260001: 3 hod × 100 Kč = 300 Kč, částečná úhrada 100 Kč a doplatek 200 Kč.
Podklady pro rekonstrukci chování a layoutu; cizí logo/ilustrace nepatří do
produktových assets Open Faktury. Viz E-021 a průchodový audit.

| Soubor | Stav |
| --- | --- |
| [01](idoklad-01-editor-customer.jpg) | Editor po založení a výběru odběratele |
| [02](idoklad-02-editor-filled.jpg) | Vyplněný doklad před uložením |
| [03](idoklad-03-after-save.jpg) | Varování po Uložit: chybí banka, akce Přesto uložit |
| [04](idoklad-04-invoice-list.jpg) | Vydaná faktura v seznamu, výchozí sloupce a akce |
| [05](idoklad-05-filters.jpg) | Rozbalené filtry seznamu |
| [06](idoklad-06-invoice-detail.jpg) | Detail vydané faktury, záložka Doklad |
| [07](idoklad-07-payments-empty.jpg) | Úhrady před první platbou; načítající se prázdný seznam, definitivní empty state viz pozadí 08 |
| [08](idoklad-08-payment-dialog.jpg) | Dialog zadání první částečné platby 100 Kč |
| [09](idoklad-09-partial-payment.jpg) | Částečně uhrazeno, jeden řádek 100 Kč |
| [10](idoklad-10-payment-remainder.jpg) | Dialog doplatku: 100/300 Kč, předvyplněno 200 Kč |
| [11](idoklad-11-paid.jpg) | Uhrazeno, dva platební řádky |

![Seznam faktur](idoklad-04-invoice-list.jpg)
![Doplatek](idoklad-10-payment-remainder.jpg)

## Adresář (E-022)

| Soubor | Stav |
| --- | --- |
| [12](idoklad-12-contacts.jpg) | Seznam kontaktů, sloupce, akce a stránkování |
| [13](idoklad-13-contact-detail.jpg) | Čitelný detail, základní údaje a kontaktní informace |
| [14](idoklad-14-contact-editor.jpg) | Editor stejného kontaktu, dvě záložky, rozbalovací podrobnosti |

Čtení existujícího syntetického kontaktu; editor opuštěn bez uložení změn.

## Rám, přehled a nastavení (E-023)

| Soubor | Stav |
| --- | --- |
| [15](idoklad-15-overview.jpg) | Úvodní přehled, skutečné widgety reference |
| [16](idoklad-16-company.jpg) | Základní nastavení firmy a rozbalená navigace |
| [17](idoklad-17-company-accounting.jpg) | Účetní údaje firmy |
| [18](idoklad-18-user-account.jpg) | Přihlašovací údaje uživatelského účtu |
| [19](idoklad-19-user-personal.jpg) | Osobní údaje uživatelského účtu |

Pouze čtení syntetické agendy. Změny účtu ani nastavení nebyly odeslány.

## Ceník a sklad (E-024)

| Soubor | Stav |
| --- | --- |
| [20](idoklad-20-catalog-empty.jpg) | Prázdný seznam ceníku a sloupce |
| [21](idoklad-21-catalog-editor.jpg) | Nová položka, cena a měna |
| [22](idoklad-22-catalog-stock-editor.jpg) | Zapnutá skladová evidence a pole počátečního pohybu |

Neuložený editor opuštěn přes Zrušit a potvrzení. Žádné nové referenční položky nevznikly.

## Šablony a nabídky — vstupní průzkum (E-025)

| Soubor | Stav |
| --- | --- |
| [23](idoklad-23-templates-empty.jpg) | Prázdný registr šablon |
| [24](idoklad-24-template-editor.jpg) | Nová šablona, bez změn |
| [25](idoklad-25-quotes-empty.jpg) | Prázdný registr nabídek |
| [26](idoklad-26-quote-editor.jpg) | Nová nabídka, bez změn |
| [27](idoklad-27-quote-states.jpg) | Rozbalené stavy nabídky |

Žádná šablona ani nabídka nebyla uložena. Převod do faktury dosud neověřen.


## Uložení a převod (E-026)

| Soubor | Stav |
| --- | --- |
| [28](idoklad-28-quote-detail.jpg) | Uložená syntetická nabídka CN20260001, 2 × 1 200 Kč |
| [29](idoklad-29-quote-conversion-editor.jpg) | Předvyplněný editor faktury z nabídky; následně zrušen |
| [30](idoklad-30-template-saved.jpg) | Uložená šablona bez odběratele |

- [31 — nabídka: e-mail s PDF přílohou](31-quote-email-attachment.jpg), E-027; pouze dialog, neodesláno.
- [32 — prázdná historie e-mailů](32-email-history-empty.jpg), E-027.
- [33 — faktura: e-mail s PDF přílohou](idoklad-33-invoice-email.jpg), E-029; pouze dialog, neodesláno.
