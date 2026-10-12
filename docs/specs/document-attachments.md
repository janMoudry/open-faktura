# ATTACHMENTS-01 — vlastní přílohy (P10-03)

12. 10. 2026, implementovaný rozsah: faktury včetně uložených návrhů a historických
importů, cenové nabídky, výběr souborů do e-mailu a historické stažení. E-030 je
veřejná nápověda, nikoli živě ověřený upload v iDokladu. Číselné limity v bytech,
quota a způsob uložení níže jsou naše návrhová rozhodnutí.

## Uživatelský postup

Záložka Přílohy v detailu faktury/nabídky; v editoru nabídky společný panel.
Nový neuložený doklad nejdříve uložit. Nahrání z výběru nebo přetažením, seznam
názvů/velikostí, stažení a potvrzené odstranění. Role owner/editor mění; viewer
pouze čte. Archivovanou nabídku nelze měnit. Šablony nejsou v tomto rozsahu.

Maximálně 5 aktivních souborů/doklad, 1–2 097 152 bajtů na soubor (UI uvádí MiB),
256 MiB uložených bytů na profil včetně odeslaných historických příloh.
Podporované přípony: PDF, PNG, JPG/JPEG, ZIP, RAR, XLSX, DOCX, ODT, ODS, TXT.
Backend kontroluje příponu a základní signaturu/strukturu, TXT je UTF-8 bez NUL.
Žádné rozbalování archivů, inline HTML ani spouštění souborů. Nejde o antivirovou
kontrolu. Názvy nesmí obsahovat cestu, řídicí znaky či přepínače směru textu.

Při odesílání se vlastní soubory explicitně zaškrtnou; výchozí výběr je prázdný.
PDF dokladu je samostatná pevná příloha jako dosud. E-mail s odkazem na nabídku
může mít také vybrané souborové přílohy. Veřejná stránka nabídky je nezpřístupňuje.
Kopie/převod dokladu automaticky nekopíruje vlastní přílohy; originál je ponechá.

## Konzistence a ukládání

Migrace 017: document_attachment s bytovým obsahem v PostgreSQL a složenými
firemními FK, metadaty, SHA-256 a upload request_id. Stejný obsah/název/doklad/aktér
se stejným klíčem vrátí stejný soubor, jiné použití je konflikt. Smazaný upload se
stejným klíčem neobnoví. Přílohy nemění revizi, součty ani PDF vydaného dokladu.

Při vytvoření e-mailu se pod společným profilovým zámkem ověří vybraná ID a zapíší
metadata i vazby delivery_attachment v transakci s outboxem. Neexistující,
smazaná nebo cizí příloha nesmí vést k částečnému e-mailu. SMTP čte uložené vazby;
obsah vybraného souboru je neměnný. Limity uploadu, smazání a odeslání používají
stejný zámek proti souběhu. Prázdný výběr se neserializuje do command, aby původní
idempotentní požadavky zůstaly kompatibilní.

Smazání odstraní přílohu z aktivního seznamu. Pokud již patří do outboxu, zachová
byty pro odeslání a historii. Jinak uvolní obsah, metadata/audit ponechá jako
idempotentní tombstone. Historická cesta ověřuje firmu, doklad, odeslání a přílohu.
Stažení je attachment/octet-stream, nosniff, no-store; název RFC 5987 UTF-8.

Byty jsou součástí běžné PostgreSQL zálohy, bez samostatného adresáře souborů.
Starší backend nemá podporu MIME příloh; nasadit novou verzi všech workerů před
použitím funkce. Žádné přepisování minulých migrací, stará historie má prázdný seznam.

REST a MCP sdílejí AttachmentStore a aplikační upload; kontrakt attachment.openapi.json.
JSON upload/download v MCP používá base64, REST download je binární. SMTP MIME
přikládá původní byty a detekovaný MIME typ. Nastavení SMTP i vystavení jsou oddělené.

## Ověření

DB/SMTP scénáře attachment.rs: souběžný upload/replay, role a cizí profil/doklad,
velikost/typ/počet, odstranění/uvolnění, zachování historie, přesné byty, MIME výběr,
REST hlavičky a MCP. Stávající invoice_delivery/delivery/share ověřují regresi
idempotence a upgradu. UI invoices.spec.ts a sales.spec.ts: CS/EN desktop/mobil,
upload, stažení, výběr, SMTP capture, odstranění a stažení z historie. Pouze
syntetická data a loopback SMTP, nikoli skuteční zákazníci.
