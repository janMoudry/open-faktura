---
name: repo-structure
description: Udržuje adresáře a hranice závislostí monorepa Open Faktury. Použij při zakládání, přesunu nebo dělení modulů, komponent a balíčků.
---

# repo-structure

Přečti `docs/architecture/DEVELOPMENT_CONVENTIONS.md` od kořene repa z `git rev-parse --show-toplevel`. Je autoritou nad starším layoutem v průzkumu.

Zařaď soubor podle odpovědnosti: pages pro obrazovky, components podle atomic designu, hooks pro React orchestrace, services pro API a utils pouze pro čisté pomocné funkce. Top-level features nezaváděj. Sdílené UI patří do packages/ui a nesmí importovat doménu aplikace. Přesunem zachovej jediný zdroj komponenty.

Desktop sdílí webové UI. Rust vrstvy dodržují domain → žádná infrastruktura, application → domain, infrastructure → porty application; server/worker je sestavují. Nedělej crate pro každou tabulku.

Při přesunu aktualizuj importy, veřejné exporty, build konfiguraci a testy; ověř cykly a porušení hranic dostupnými nástroji projektu. Adresáře vytvářej s reálným obsahem. Změnu konvence promítni do dokumentu, ne do paralelního seznamu pravidel.
