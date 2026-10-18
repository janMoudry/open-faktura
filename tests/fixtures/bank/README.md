# Syntetické kontraktové příklady

Všechny soubory vytvořil projekt pro testy parserů podle veřejných formátů
(E-042). Nejde o skutečné exporty bank ani zákaznická data. IBAN ve fixture je
běžný demonstrační IBAN; nepoužívat k odesílání peněz.

camt053.xml testuje sdílenou strukturu; není důkazem individuálních variant
Air Bank, ČS, ČSOB, KB nebo UniCredit. MONETA příklad záměrně uvádí explicitní
směr, který konzervativní adaptér vyžaduje. RB test obsahuje ediční číslo účtu
s umělým IBAN checksumem v unit testu (uživatelský účet vždy kontroluje doména).
