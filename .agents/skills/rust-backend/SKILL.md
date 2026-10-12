---
name: rust-backend
description: Implementuje Rust backend Open Faktury s oddělenou doménou, aplikačními příkazy a adaptéry. Použij při změně serveru, workerů a backendové logiky.
---

# rust-backend

Přečti `docs/architecture/DEVELOPMENT_CONVENTIONS.md`, dotčenou specifikaci a podle potřeby `docs/DATA_MODEL.md`. Rust je přijaté rozhodnutí; konkrétní framework ověř v Cargo manifestech.

Používej doménové typy pro identifikátory a peníze, rozliš validaci vstupu, doménovou chybu a chybu infrastruktury. HTTP i MCP volají stejnou aplikační operaci s aktérem a firmou. Nedůvěřuj company_id od klienta bez autorizace.

Doklad, číslování, audit a outbox zapisuj v odpovídající společné transakci. Externí I/O neprováděj pod dlouho drženým DB zámkem. Opakované příkazy řeš klíčem, hashem vstupu a uloženým výsledkem; chybné znovupoužití klíče je konflikt.

Při implementaci async práce omez souběh a dobu běhu; blokující PDF/CPU práci odděl od obsluhy requestu. Logy obsahují korelační ID, nikoli tokeny a celé citlivé doklady. Spusť dostupné rustfmt/clippy a relevantní testy; příkazy udržuj v AGENTS.md až po jejich zavedení.
