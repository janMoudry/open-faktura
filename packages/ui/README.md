# Open Faktura UI

Sdílené komponenty bez závislosti na fakturační doméně, API nebo routeru. Živý katalog: `/sandbox.html#/design-system` (pouze vývojový server). Barvy a typografie jsou v `packages/design-tokens`, styly komponent v `styles.css`.

## Použití

V hlavním CSS aplikace po Tailwind a design tokenech importovat `@open-faktura/ui/styles.css`. Tailwind musí skenovat `packages/ui/src` (web to již dělá).

```tsx
import { Button, Field, Icon } from "@open-faktura/ui";

<Field
  label="Zákazník"
  required
  hint="Firma nebo jméno podnikatele."
  error={errors.customer}
  value={customer}
  onChange={(event) => setCustomer(event.target.value)}
/>;
<Button type="submit" loading={saving}>
  <Icon name="check" />
  Uložit koncept
</Button>;
```

## Sada v0.1

| Komponenta                                             | Kontrakt                                                                                                                                                                                              |
| ------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Button                                                 | `variant`: primary / secondary / ghost / danger, `size`: md / sm; výchozí `type="button"`, submit vždy explicitně. `loading` blokuje opakovaný klik a nastaví aria-busy; popisek dodává volající.     |
| IconButton                                             | Stejné varianty, povinné `label` a `icon`; základní dotyková plocha 44 × 44 px.                                                                                                                       |
| Icon                                                   | Výhradně Lucide, dekorativní SVG; význam sděluje text nebo popisek nadřazeného ovládání.                                                                                                              |
| Input / Textarea                                       | Nativní HTML chování, společné styly; při samostatném použití povinný přístupný název od volajícího.                                                                                                  |
| Field / TextareaField                                  | Label, required, hint, error; automatická jedinečná ID a aria-describedby, při error také aria-invalid a oznámení chyby. Vlastní ID a popisy lze doplnit.                                             |
| Checkbox                                               | Nativní checkbox se společným labelem a dotykovou plochou; podporuje checked, defaultChecked a disabled.                                                                                              |
| Badge                                                  | Významové tóny neutral / success / warning / danger. Stav se vždy vyjadřuje textem, nikoli jen barvou.                                                                                                |
| Alert                                                  | Stejné tóny; `role="status"` pro novou neurgentní zprávu, `role="alert"` pro dynamickou chybu. Statické informace bez live role.                                                                      |
| Card                                                   | Sdílený povrch, okraj a odsazení, volitelný nadpis h2. Doménové sekce lze skládat jako children.                                                                                                      |
| FilterTabs                                             | Skupina filtrujících tlačítek s aria-pressed; není ARIA tablist, nepředstírá přepínání tabpanelů. Každá volba je dostupná Tabem.                                                                      |
| Table / TableHeader / TableRow / TableHead / TableCell | Nativní tabulka, jednotné řádky a sloupce se scope=col. Popisek tabulky, datový stav a mobilní alternativu dodává stránka.                                                                            |
| EmptyState                                             | Povinné vysvětlení a volitelná akce; nic automaticky nezakládá.                                                                                                                                       |
| Dialog                                                 | Řízený `open`, `onClose`, title, description, children. Radix zajišťuje modalitu, focus trap, scroll lock a Escape; adaptér vrací focus původnímu prvku. Klik na pozadí nezahazuje rozpracovaná data. |

## Rozměry a stavy

- Výchozí ovládání 44 px, kompaktní tlačítko 36 px; běžné ikonové akce používat ve výchozí velikosti.
- Inputy na mobilu 16 px, od sm 14 px. Rozměry neměnit lokálními px/py třídami; upravit společný token nebo přidat pojmenovanou variantu.
- Readonly a disabled jsou odlišné: readonly zůstává čitelné a fokusovatelné, disabled neaktivní. Disabled nesmí vypadat jako primární akce.
- Focus je viditelný, chybový vstup má kromě barvy propojený text. Validace samotná zůstává odpovědností formuláře.
- Pohyb je omezen na drobnou odezvu tlačítka a spinner; respektuje prefers-reduced-motion. Klávesnicový focus neposkakuje animací.
- `className` slouží hlavně pro umístění (šířka, vnější mezery), nikoli přepis barev a vnitřní geometrie komponenty.

## Ověření a rozšiřování

Nová komponenta nebo varianta musí mít použitelný příklad v katalogu a relevantní stavy. Komplexní interakce stavět na zavedených primitives, nepřidávat vlastní obecný focus trap nebo duplicitní sadu ikon. [Radix Dialog](https://www.radix-ui.com/primitives/docs/components/dialog) řeší dialog; React Aria zajišťuje výběry, našeptávání a kalendář s vlastním vzhledem.

`tests/e2e/design-system.spec.ts`: vazby chyb, disabled/loading, dialog a focus, axe WCAG 2 A/AA a 2.1 AA kontroly na katalogu, seznamu a editoru. `tests/e2e/invoices.spec.ts`: funkční regresní scénáře. 10. 10. 2026 prošel build, formátování, kontrola tokenů a E2E testy v Chromium desktop/mobilní emulaci. Vizuálně ověřeno při 1440 a 390 px. Automatické testy nejsou úplným auditem přístupnosti ani potvrzením chování na fyzickém telefonu nebo všech Tauri platformách.

Rozsah pokrývá současný seznam a editor. Napojení Autocomplete na skutečný adresář, výběr časového rozsahu, menu hromadných akcí a upload přidávat s konkrétní funkcí a testy.

## Aktuální brandový základ

Monochromatický základ, Manrope pro nadpisy i text, Lucide. Výjimkou jsou sémantické barvy: zelený success, jantarový warning a červený danger. Primary se nepoužívá jako náhrada success. Badge přidává vlastní stavovou ikonu podle tónu; volající ji znovu nevkládá. Tóny zachovávají sémantiku pro budoucí změnu brandu. Redesign řídí `.agents/skills/brand-redesign/SKILL.md`; vizuální identita se mění přes tokeny a sdílené komponenty, ne přes kopie stránek.

## Grafy

Používáme [Recharts](https://github.com/recharts/recharts), verzi 3.10.1 (MIT),
ověřeno 10. 10. 2026. Deklarativní SVG komponenty pro React pokryjí běžné
dashboardy; knihovna není určena pro neomezené množství bodů. Agregaci období a
omezení počtu bodů řeší datový adaptér/server.

Import: `import { DataChart } from "@open-faktura/ui/charts"`.
Grafy jsou samostatný export; neexportovat je z hlavního UI barrel souboru.
Katalog načítá ChartGallery přes React.lazy, takže seznam faktur nestahuje Recharts.

DataChart nabízí line/bar, jednu až dvě explicitně pojmenované řady, title,
description, data, formatValue a formatAxis. Bod má label, values podle klíčů řad
a volitelné formattedValues pro přesné popisky. Klíče řad musí být unikátní.
Neutrální barvy chart-primary/secondary jsou centrální tokeny, druhá čára je
přerušovaná. Stavové barvy neslouží k náhodnému rozlišení řad.
Animace jsou vypnuté, dostupná je klávesnice Recharts i rozbalovací datová tabulka.
Nula je hodnota; null, neplatné a chybějící hodnoty jsou mezery a „Bez dat“.
Bez jediného platného bodu se zobrazuje prázdný stav.

Číselné hodnoty slouží výhradně pro geometrii grafu. Peníze se nadále počítají
desetinnou aritmetikou v doméně; adaptér dodává přesné formattedValues do tooltipů
a tabulky. Ze souřadnic grafu nikdy neodvozovat účetní součty. Katalog používá
syntetické počty, nikoli živé účetní reporty.

## Atomy a výběrové komponenty

Katalog má explicitní oddíly Atomy, Molekuly a složené interakce. Atomy jsou
Button, IconButton, Input, Textarea, Checkbox, Radio, Switch, Badge, Icon,
Spinner a Separator. Radio používá nativní skupinu přes společný name a
fieldset/legend v kompozici. Switch je checkbox s role=switch, podporuje
checked/defaultChecked a onChange. Spinner má role=status a textový popisek;
dekorativní spinner uvnitř tlačítka zůstává ikonou. Separator je sémantické hr.

Select, DatePicker a Autocomplete jsou molekuly: skládají vstup, popisek,
popisy, validaci a vlastní nabídku. Jejich základ tvoří React Aria Components
1.22.1 (Apache-2.0), kalendář @internationalized/date 3.12.4 (Apache-2.0).
České prostředí, naše tokeny, Lucide ikony, portály s hlídáním okrajů viewportu.
Licence jsou přiložené ve webovém balíku.

- **Select / SelectField**: options s value, label, description a disabled;
  value/defaultValue jsou řetězce, onChange vrací hodnotu, nikoli DOM event.
  Jeden výběr, placeholder, label nebo aria-label, hint, error, required,
  disabled a name. SelectField je alias stejné komponenty.
- **Autocomplete**: stejný formát options; value/onChange představuje editovaný
  text, onSelect vrací klíč vybraného návrhu. Volný text je povolen; nejde o
  ověřenou vazbu na existující kontakt. Filtrování ignoruje diakritiku a prohledá
  label i description. Loading a prázdné výsledky mají vlastní obsah nabídky.
  Zdroj dat dodává aplikace; komponenta nevolá API. Pro vzdálené hledání řeší
  aplikace debounce, zrušení a pořadí odpovědí.
- **DatePicker**: value/defaultValue/min/max jsou platné kalendářní řetězce
  YYYY-MM-DD nebo prázdná hodnota; onChange vrací ISO datum nebo prázdný řetězec.
  Nepřevádět přes JS Date ani UTC. Podporuje ruční editaci segmentů, vlastní
  měsíční kalendář, required, disabled, readOnly, hint a error.
  Začátek týdne je pondělí. Nejde o výběr času nebo rozsahu.
- Nabídky podporují šipky, Enter, Escape a návrat focusu. Neskládat nové
  nativní select, input[type=date] nebo datalist v obrazovkách. Nativní skrytý
  select uvnitř React Aria slouží formulářům/autofillu, nikoli jako viditelné menu.

Inspirace chováním (ověřeno 10. 10. 2026):
[Ant Design Select](https://ant.design/components/select/),
[DatePicker](https://ant.design/components/date-picker/),
[AutoComplete](https://ant.design/components/auto-complete/).
Implementační dokumentace:
[React Aria Select](https://react-aria.adobe.com/Select),
[ComboBox](https://react-aria.adobe.com/ComboBox),
[DatePicker](https://react-aria.adobe.com/DatePicker).
Nejde o převzetí celé knihovny ani příslib všech variant Ant Designu.

Scénáře v tests/e2e/selection.spec.ts ověřují výběr klávesnicí, disabled volby,
návrat focusu, hledání bez diakritiky, volný text, loading, kalendář a omezení dat,
Radio/Switch a axe kontroly otevřených nabídek na desktopu i mobilní emulaci.

Omezení auditu: při otevřeném ComboBox má React Aria
[zdokumentované falešné hlášení aria-hidden-focus](https://github.com/adobe/react-spectrum/wiki/Known-accessibility-false-positives).
Pouze v tomto otevřeném stavu test vynechává uvedené pravidlo; ověřuje Tab,
zavření nabídky, další focus a poté celý audit bez výjimky. Nejde o úplný audit
se čtečkami obrazovky.

## Nadpisy, rozestupy a taby

Používat Heading s nezávislými props level (HTML H1–H6) a size
(page/section/card/label). Výchozí level=2, size=section. Page 32/24 px na
desktopu/mobilu, section 24 px, card 16 px, label 14 px. Vše Manrope 600.
Jeden H1 na stránce, vnořené sekce H2/H3; Card přijímá titleLevel, výchozí 2.
Dialog a kalendář zachovávají sémantické primitives své knihovny.

Rozestupy: inline=8, control=12, field=16, group=24, section=32, page=48 px.
Používat gap-inline/control/field/group/section/page a stejné role pro m/p/space.
Škála je v design-tokens; číselné utility ponechat pro mikrogeometrii prvků
(ikony, badge, padding tlačítek), nikoli pro nové rozložení obrazovek.
Živé vzorky a význam rolí jsou v katalogu Základy · nadpisy a rozestupy.

FilterTabs zachovává role=group a aria-pressed pro filtry. Aktivní linka se při
kliknutí přesouvá 200 ms; reaguje na změnu šířky, labelů a zalomení řádků.
První zobrazení a klávesnicové přepnutí jsou okamžité, reduced-motion vypíná
pohyb. Nové obrazovky používají tento společný prvek místo lokálních podtržení.

## Audit stavů

Výsledky a matice současné sady:
[design-system-audit.md](../../docs/design/design-system-audit.md).
Checkbox podporuje indeterminate (stav řídí volající); FilterTabs options mají
disabled. DataChart přijímá loading, error a onRetry. Loading má přednost před
error a oba stavy schovají starý graf i tabulku. IconButton loading nahrazuje
původní ikonu. Radius control/popover/panel a motion feedback/overlay/switch/tabs
patří do centrálních tokenů. Katalog ukazuje i readonly/error datum a zakázaný
zapnutý switch; nový stav musí mít ukázku a relevantní ověření.

Pole `Field` a `TextareaField` podporují `help`: informační ikonu vedle labelu
a přístupný tooltip. Zobrazuje se při hoveru a focusu ikony;
Escape ho zavře bez přesunu focusu. Nepoužívat pouze `title` nebo hover pro důležité
vysvětlení pole. Ukázka je u názvu zákazníka v katalogu.

## Lokalizace

Obalte UI v `LocaleProvider language="cs"` nebo `language="en"`. Výchozí jazyk
je čeština. Obecné popisky komponent a locale kalendářů se přepnou společně;
doménové labely přeloží aplikace. `useLocale()` poskytuje jazyk, locale a typované
`t`. Formátovací helpery a pravidla: [lokalizace](../../docs/LOCALIZATION.md).

## Tabulky

Produktové seznamy, náhled importu a položky dokladu používají společné Table
primitives. Jediný styl vlastní `ui-table-*`: neutrální hlavička, jemné vodorovné
oddělovače, 16 px vodorovně a 12 px svisle z tokenů field/control. Hover a
focus-within zvýrazní datový řádek; animace respektuje reduced motion.
Částky zarovnávat doprava s tabular-nums, data nezalamovat, identifikátor
zdůraznit odkazem a pomocné údaje tlumit. Celý řádek není falešné tlačítko.
Na mobilu posouvat pojmenovaný, klávesnicí dostupný region obalující tabulku.
Prázdný stav patří místo tabulky; v importu nezobrazovat sloupce bez obsahu.

Reference ověřené 11. 10. 2026: veřejná dokumentace
[iDoklad – faktury vydané](https://www.idoklad.cz/podpora/faktury-vydane)
a screenshot v návodu [FakturaOnline – hromadné stažení](https://www.fakturaonline.cz/navody/faktury/vystaveni-a-sprava-faktur/hromadne-stazeni-vystavenych-faktur).
Jde o vizuální podklady, nikoli pozorování přihlášených aplikací. Naše úprava
zachovává dostupné funkce; hromadné akce nepřebírá bez implementace.

RE-01 doplňuje přímé reference z testovacího iDokladu (E-021; screenshoty
04/05/06/11 v docs/research/screenshots/2026-10-11). Registr faktur používá
kompaktní tokeny control/inline a min-width invoice-register; badge se
nezalamuje uprostřed slova. Ostatní tabulky zachovávají výchozí hustotu.

`Tabs` jsou obsahové záložky nad React Aria: šipky, disabled položky, propojené
panely, kontrolovaný value/onChange a zachování stavu skrytých panelů.
Na rozdíl od FilterTabs mají role tablist/tab/tabpanel. `FilterTabs` nadále
mění filtr dat pomocí aria-pressed. Doménové texty a obsah dodává aplikace.
