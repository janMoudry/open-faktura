from pathlib import Path
from html import escape
import shutil
from fontTools.ttLib import TTFont
from fontTools.varLib.instancer import instantiateVariableFont
from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.pens.transformPen import TransformPen
from fontTools.pens.boundsPen import BoundsPen

ROOT = Path('/Users/janmoudry/Documents/open-faktura')
OUT = ROOT / 'docs/design/brand/open-faktura-2026-10-17'
FONT = ROOT / 'apps/web/node_modules/@fontsource-variable/manrope/files/manrope-latin-wght-normal.woff2'
font = instantiateVariableFont(TTFont(FONT), {'wght': 800}, inplace=True)
glyphs = font.getGlyphSet()
cmap = font.getBestCmap()
INK, RED, WHITE = '#17191C', '#D84B30', '#FFFFFF'
MARK = 'M0 0H128V64H104V24H24V104H64V128H0Z'

def word_paths(word, x, top):
    advance = 0
    bounds = []
    for char in word:
        name = cmap[ord(char)]
        pen = BoundsPen(glyphs)
        glyphs[name].draw(pen)
        bounds.append((advance + pen.bounds[0], pen.bounds[3]))
        advance += glyphs[name].width - 25
    left = min(b[0] for b in bounds)
    high = max(b[1] for b in bounds)
    scale = .045
    pen = SVGPathPen(glyphs)
    advance = 0
    for char in word:
        name = cmap[ord(char)]
        transform = (scale, 0, 0, -scale, x + (advance-left)*scale, top + high*scale)
        glyphs[name].draw(TransformPen(pen, transform))
        advance += glyphs[name].width - 25
    return pen.getCommands()

WORDS = word_paths('open', 180, 24) + ' ' + word_paths('faktura', 180, 96)

def svg(name, symbol, text=None, mark_only=False):
    box = '0 0 160 160' if mark_only else '0 0 512 192'
    size = 'width="160" height="160"' if mark_only else 'width="512" height="192"'
    transform = 'translate(16 16)' if mark_only else 'translate(24 28)'
    parts = [f'<svg xmlns="http://www.w3.org/2000/svg" {size} viewBox="{box}" role="img" aria-labelledby="title">',
             f'<title id="title">{escape(name)}</title>',
             '<desc>Open Faktura — otevřený rám a dvouřádkový slovní znak; vektorové křivky bez externích závislostí.</desc>' if not mark_only else '<desc>Samostatný otevřený rám Open Faktury.</desc>',
             f'<path fill="{symbol}" transform="{transform}" d="{MARK}"/>']
    if not mark_only:
        parts.append(f'<path fill="{text}" d="{WORDS}"/>')
    parts.append('</svg>')
    return '\n'.join(parts) + '\n'

variants = {
 'open-faktura': svg('Open Faktura', RED, INK),
 'open-faktura-inverse': svg('Open Faktura — pro tmavé pozadí', RED, WHITE),
 'open-faktura-mono': svg('Open Faktura — jednobarevné tmavé logo', INK, INK),
 'open-faktura-white': svg('Open Faktura — jednobarevné bílé logo', WHITE, WHITE),
 'open-faktura-symbol': svg('Open Faktura — symbol', RED, mark_only=True),
 'open-faktura-symbol-mono': svg('Open Faktura — tmavý symbol', INK, mark_only=True),
 'open-faktura-symbol-white': svg('Open Faktura — bílý symbol', WHITE, mark_only=True),
}
for name, content in variants.items():
    (OUT / f'{name}.svg').write_text(content)
shutil.copyfile(ROOT / 'apps/web/public/licenses/manrope.txt', OUT / 'Manrope-OFL.txt')

preview = ['<svg xmlns="http://www.w3.org/2000/svg" width="1600" height="1040" viewBox="0 0 1600 1040">',
           '<rect width="1600" height="1040" fill="#F5F5F3"/>',
           '<rect x="40" y="40" width="980" height="560" rx="12" fill="#FFFFFF"/>',
           '<rect x="1040" y="40" width="520" height="560" rx="12" fill="#202329"/>',
           '<rect x="40" y="620" width="740" height="380" rx="12" fill="#FFFFFF"/>',
           '<rect x="800" y="620" width="760" height="380" rx="12" fill="#E9E9E6"/>']

def place(key, x, y, scale):
    import re
    content = re.sub(r'^<svg[^>]*>|</svg>\s*$', '', variants[key], flags=re.M)
    return f'<g transform="translate({x} {y}) scale({scale})">{content}</g>'

preview += [place('open-faktura', 90, 160, 1.7),
            place('open-faktura-inverse', 1060, 230, .94),
            place('open-faktura-mono', 115, 710, 1.15),
            place('open-faktura-symbol', 880, 710, 1.15),
            place('open-faktura-symbol-mono', 1120, 710, .75),
            place('open-faktura-symbol-white', 1335, 710, .75), '</svg>']
(OUT / 'preview.svg').write_text('\n'.join(preview))
print(f'Created {len(variants)} SVG logo assets and preview in {OUT}')
