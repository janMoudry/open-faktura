# ACCOUNT-01 — Vlastní účet

Vlastní návrh na přímé zadání uživatele, 11. 10. 2026; nejde o tvrzení shody
s konkrétní obrazovkou iDokladu. Účet je globální a nezávisí na fakturačním profilu.

Menu avatara otevírá dialog se jménem, e-mailem a volitelnou změnou hesla.
Jméno může zůstat prázdné; avatar potom používá e-mail. Změna e-mailu nebo hesla
vyžaduje současné heslo. Jméno samo ověření heslem nepotřebuje. Nové heslo má
alespoň 12 znaků a nejvýše 1024 UTF-8 bajtů; e-mail se normalizuje stejně jako
při přihlášení. Formulář potvrzuje nové heslo, zachová vstupy při chybě a po
úspěchu vyčistí hesla. Zavření rozpracovaného dialogu vyžaduje potvrzení.
Aktualizované jméno a e-mail se ihned promítnou do menu. CS/EN používá sdílený
Select s českou a britskou vlajkou a názvem jazyka, volba přetrvá reload.

POST /api/auth/account přijímá name, email, current_password (volitelné pro
samotné jméno), new_password (null/neuvedené = beze změny). Vrací User, nyní
včetně name. Vyžaduje vlastní platnou relaci, povolený Origin a X-Faktura-Request.
Nepřijímá user_id, roli ani administrátorská oprávnění. 422 invalid_password,
422 invalid_input, 409 email_taken / conflict, 401 unauthorized, 429 rate_limited,
503 unavailable. Odpovědi nejsou cachované. Existující rate limiter omezuje
změny účtu na 20 pokusů za 15 minut na uživatele, hashování má omezený souběh.

Argon2 ověřuje heslo mimo transakci. Zámek uživatele a credential_version
zamezují zápisu podle zastaralých přihlašovacích údajů a vydání nové relace
po souběžné změně hesla. Relace se znovu kontroluje uvnitř transakce.
Změna přihlašovacích údajů atomicky zruší ostatní relace (včetně MCP); aktuální
relace si zachová původní absolutní expiraci. Audit ukládá akci a user_id,
nikoli hesla. Unikátní e-mail v DB chrání proti souběžným kolizím.

Migrace 202610110005 pouze přidává name s prázdným výchozím řetězcem a limitem
120 znaků. Existující účty a relace zůstávají zachované. Nasazovat backend před
novým frontendem, který očekává name v odpovědi User. Veřejný kontrakt je
contracts/auth.openapi.json; generovaný klient vzniká přes pnpm generate:api.

Limity: změna adresy zatím neověřuje vlastnictví schránky; reset zapomenutého
hesla e-mailem a MFA nejsou součástí tohoto balíku. Účetní údaje se nemění.

Akceptace: neautorizovaný/cizí origin nemůže zapisovat; jméno přežije reload;
špatné heslo, obsazený e-mail a nevalidní vstup neprovedou změnu; změněné údaje
fungují při novém přihlášení; staré heslo a ostatní relace přestanou fungovat;
aktuální relace, profily a oprávnění zůstávají. UI se ověřuje v obou jazycích
na mobilu a desktopu včetně přístupnosti dialogu a vlastního selectu.
