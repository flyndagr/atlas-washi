"""Export the canonical Atlas mark and macOS icon from assets/brand.json.
Requires Pillow. Run from any directory. Transparent mark padding matches the app.
"""
from pathlib import Path
import json, math, random
from PIL import Image, ImageDraw
ROOT = Path(__file__).resolve().parents[1]
brand = json.loads((ROOT / 'assets/brand.json').read_text())

def mark(size, radius=None):
    scale = 4
    r = (radius or size * 14 / 38) * scale
    center = size * scale / 2
    im = Image.new('RGBA', (size * scale, size * scale))
    draw = ImageDraw.Draw(im)
    point = lambda x,y: (center+x*r, center+y*r)
    c = brand['circle']
    for i in range(c['segments']):
        t = i/c['segments']
        a,b = c['start']+t*c['sweep'],c['start']+(i+1)/c['segments']*c['sweep']
        rr = 1+c['wobble']*math.sin(a*5)
        width = r*(c['base_width']+c['width_variation']*abs(math.sin(math.pi*t)))
        p,q = point(math.cos(a)*rr,math.sin(a)*rr),point(math.cos(b),math.sin(b))
        draw.line([p,q],fill=tuple(brand['ink']),width=round(width))
        for x,y in (p,q):draw.ellipse((x-width/2,y-width/2,x+width/2,y+width/2),fill=tuple(brand['ink']))
    n=brand['nib']
    def bounds(y):
        outer=n['half_width']*((y-n['top'])/(n['shoulder']-n['top']) if y<=n['shoulder'] else (n['bottom']-y)/(n['bottom']-n['shoulder']))
        hole=math.sqrt(max(0,n['hole_radius']**2-(y-n['shoulder'])**2))
        inner=min(max(hole,n['slit_half_width'] if y<n['shoulder'] else 0),max(outer,0))
        return max(outer,0),inner
    for i in range(n['segments']):
        y0=n['top']+i*(n['bottom']-n['top'])/n['segments']; y1=n['top']+(i+1)*(n['bottom']-n['top'])/n['segments']
        o0,h0=bounds(y0);o1,h1=bounds(y1)
        for side in (-1,1):draw.polygon([point(x*side,y) for x,y in [(h0,y0),(o0,y0),(o1,y1),(h1,y1)]],fill=tuple(brand['accent']))
    return im.resize((size,size),Image.Resampling.LANCZOS)

mark(512).save(ROOT/'assets/atlas-mark.png')
mark(512).save(ROOT/'website/assets/mark.png')
mark(64).save(ROOT/'website/assets/favicon.png')
# The Dock's rounded paper tile is the only background treatment of the mark.
size=1024
im=Image.new('RGBA',(size,size));mask=Image.new('L',(size,size))
ImageDraw.Draw(mask).rounded_rectangle((64,64,960,960),radius=160,fill=255)
rng=random.Random(432)
paper=Image.new('RGBA',(size,size));paper.putdata([(244+g,237+g,216+g,255) for g in (rng.randrange(-3,4) for _ in range(size*size))])
im.paste(paper,(0,0),mask);im.alpha_composite(mark(size,281));im.save(ROOT/'assets/Atlas.png')
im.save(ROOT/'assets/Atlas.icns',format='ICNS')
im.resize((192,192),Image.Resampling.LANCZOS).save(ROOT/'website/assets/app-icon.png')
