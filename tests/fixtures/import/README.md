# Importní fixtures

money-s3-synthetic.xml je syntetický minimální doklad podle struktury veřejného
XSD Money S3 (E-058), nikoli skutečný zákaznický export. Ověřuje základní cenu,
ignorování vedlejší hladiny a nepřenášení skladového množství do ceníku.

Přesná publikovaná ukázka Skladova_karta.xml se testuje volitelně mimo repo
pomocí MONEY_VENDOR_FIXTURE a testu published_vendor_fixture.

catalog-synthetic.csv je vlastní tabulková fixture s cenou 1250,50 CZK,
jednotkou h a sazbou 21. Není exportem Money. Ověřuje ruční mapování,
výslovný desetinný oddělovač a zachování textového barcode 000123.
