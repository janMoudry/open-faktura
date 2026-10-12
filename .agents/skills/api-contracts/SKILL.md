---
name: api-contracts
description: Udržuje veřejné REST, OpenAPI a MCP kontrakty Open Faktury a generovaný klient. Použij při změně endpointů, DTO, nástrojů a integračních událostí.
---

# api-contracts

Přečti `docs/architecture/DEVELOPMENT_CONVENTIONS.md` a specifikaci operace. Zjisti autoritativní generátor v repu; negeneruj druhé nezávislé schéma.

Odděl transportní DTO od DB modelu. Částky přenášej bez ztráty desetinné přesnosti. Definuj chyby, null význam, stránkování a idempotenci zápisu. Typy TypeScriptu nenahrazují validaci příchozích dat.

REST a MCP mají stejné aplikační příkazy a práva. Tool annotation není autorizace. Návrh, vystavení a odeslání ponech jako explicitní operace; dlouhé úlohy vracejí identifikátor a dohledatelný stav.

Při změně regeneruj kontrakt/klienta skutečnými příkazy projektu a zkontroluj kompatibilitu existujících klientů. U MCP ověř podporovanou verzi SDK a přihlašovací tok na reálném klientovi, pokud se mění transport či autorizace. Neslibuj kompatibilitu pouze podle kompilace.
