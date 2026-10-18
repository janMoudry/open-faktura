# Koš nabídek a šablon (P20-02, 18. 10. 2026)

## Rozsah a vlastní pravidla

FEATURES §2 uvádí koš a obnovu. Veřejná nápověda
[Faktury vydané](https://www.idoklad.cz/podpora/faktury-vydane) je podkladem
existence koše; přesný níže uvedený životní cyklus nabídek a šablon je vlastní
bezpečně omezený návrh Open Faktury, nikoli doložené chování iDokladu.

Do koše lze přesunout pouze nabídku nebo šablonu bez chráněné historie.
Zálohové faktury jsou odmítnuté vždy. Nabídka/šablona nesmí mít `invoice_id`
ani záznam `sales_conversion` (také opakovaně použitá šablona je chráněná).
Nabídka nesmí mít žádný záznam e-mailové zásilky nebo veřejného sdílení,
včetně neúspěšných zásilek a již odvolaných/vypršených odkazů. Exportní
zámek také blokuje přesun. Dřívější export bez zámku a lokální přílohy
zůstávají zachované; nevytváříme předstíranou historii stažení PDF.

Koš není archivace. Archivovaný dokument lze přesunout do koše a obnovení
zachová jeho původní příznak archivu. Číslo nabídky zůstává obsazené; koš nesnižuje číselnou řadu ani znovu
nepřiděluje původní číslo. Žádný řádek dokladu, ceníku, přílohy,
konverze nebo auditní historie se fyzicky nemaže. Nezávislé kopie vytvořené
z obsahu šablony se nemění; chráněné jsou skutečně evidované vztahy, nikoli
odhad vazby podle shodného názvu.

## Příkazy, oprávnění a souběh

Všichni aktuální členové firmy mohou číst její koš, vlastník/editor mohou
přesouvat a obnovovat. Instalační administrátor bez členství nemá výjimku.
Oba zápisy vyžadují UUID a očekávanou revizi; zvyšují revizi o jednu a
přidávají událost `trash`/`restore` do stávajícího `sales_audit`. Obsahová
kopie auditu zůstává stejná. Příkazový audit obsahuje vstup a výsledek;
DB jej chrání před přepisem a mazáním.

Opakování se shodným UUID vrací původní výsledek bez další změny; aktuální
oprávnění se ověřuje vždy před replay. Znovupoužití UUID pro jiný příkaz
je konflikt. Replay starého přesunu po pozdější obnově dokument znovu
neschová: vrací historický výsledek původního příkazu.

Příkaz zamyká zdrojový řádek a teprve potom čte vztahy a stav koše. Převod,
nové sdílení, e-mail a export znovu kontrolují koš po získání zámku zdroje.
Souběžný převod/export a přesun tedy nemohou současně uspět. DB triggery
navíc odmítají chráněný zdroj a obsahové změny uvnitř koše.

## Rozhraní a klient

Migrace `202610180103_sales_trash.sql` přidává `sales_trash` a
`sales_trash_command`; při upgradu zastavit staré API. Migrace sama nic
neskrývá. Běžný seznam, detail, editace, převod a create replay skrytý
zdroj nevracejí. Přílohy, aktuální PDF nabídky, sdílení a nový export používají
stejný zdrojový guard. Dříve vytvořené exportní soubory zůstávají dostupné
přes jejich autorizované ID.

REST `GET/POST /api/companies/{company}/sales-trash` a MCP
`sales_trash_list`/`sales_trash_execute` sdílejí aplikační `TrashStore`.
Seznam má 25 položek, počet a stránku z jednoho snapshotu. Zápis obsahuje
`request_id`, `id`, `revision`, `action=trash|restore`. Neznámé klíče jsou
odmítnuté. Kontrakt: `contracts/sales-trash.openapi.json`, generované tools
ze stejného kontraktu a runtime validovaný TypeScript klient.

CS/EN detail nabízí potvrzovací dialog s výslovnými omezeními. Seznamy
nabídek a šablon odkazují na společný koš; seznam ukazuje typ, jméno,
odběratele a okamžik přesunu ve firemní časové zóně. Viewer nemá akce zápisu.

## Ověření

`pnpm test:sales-trash` používá pouze izolované syntetické databáze.
Ověřuje role, profilovou izolaci, revize/replay, původní data a audit,
odmítnutí záloh, konverzí, historie sdílení/e-mailu a exportních zámků,
souběh retry i převodu/exportu, REST relaci/Origin a MCP paritu.
Regrese zahrnuje sales, delivery, share, attachment a document_exports.
`tests/e2e/sales.spec.ts` doplňuje reálnou izolovanou DB pro CS/EN přesun,
zrušení potvrzení, koš po reloadu, obnovu obsahu a chráněný původní doklad.
