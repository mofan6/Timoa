"""Recreate the code-native Timoa icon; requires Pillow only during asset creation."""
from pathlib import Path
from PIL import Image, ImageDraw
import math

assets = Path(__file__).resolve().parent.parent / 'assets'
svg = '''<svg xmlns="http://www.w3.org/2000/svg" width="256" height="256" viewBox="0 0 256 256">
<defs><linearGradient id="base" x1="0" y1="0" x2="1" y2="1"><stop stop-color="#302B20"/><stop offset="1" stop-color="#11110F"/></linearGradient>
<linearGradient id="gold" x1="0" y1="0" x2="1" y2="1"><stop stop-color="#F3D17D"/><stop offset="1" stop-color="#D6A335"/></linearGradient></defs>
<rect x="4" y="4" width="248" height="248" rx="60" fill="url(#base)"/>
<path d="M 172 78 A 72 72 0 1 0 198 128" fill="none" stroke="url(#gold)" stroke-width="16" stroke-linecap="round"/>
<rect x="108" y="37" width="40" height="11" rx="5.5" fill="#E3B341"/>
<path d="M128 95 V139 L157 157" fill="none" stroke="#F5E8C7" stroke-width="12" stroke-linecap="round" stroke-linejoin="round"/>
<circle cx="128" cy="139" r="8" fill="#E3B341"/>
<circle cx="193" cy="91" r="9" fill="#E3B341"/>
</svg>'''
assets.joinpath('logo.svg').write_text(svg, encoding='utf-8')

def frame(size):
    scale=4; n=size*scale; k=n/256
    im=Image.new('RGBA',(n,n)); d=ImageDraw.Draw(im)
    def box(v): return tuple(round(x*k) for x in v)
    d.rounded_rectangle(box((4,4,252,252)), radius=60*k, fill='#1E1C17')
    # Ring starts just below the upper-right opening and sweeps around the clock.
    points=[]
    for step in range(281):
        a=math.radians(-52-step*1.0)
        points.append((round((128+72*math.cos(a))*k),round((135+72*math.sin(a))*k)))
    d.line(points,fill='#E3B341',width=max(1,round(16*k)),joint='curve')
    for x,y in [points[0],points[-1]]:
        r=8*k; d.ellipse((x-r,y-r,x+r,y+r),fill='#E3B341')
    d.rounded_rectangle(box((108,37,148,48)),radius=5.5*k,fill='#E3B341')
    d.line([box((128,95)),box((128,139)),box((157,157))],fill='#F5E8C7',width=max(1,round(12*k)),joint='curve')
    for x,y in [(128,95),(157,157)]: d.ellipse(box((x-6,y-6,x+6,y+6)),fill='#F5E8C7')
    d.ellipse(box((120,131,136,147)),fill='#E3B341')
    d.ellipse(box((184,82,202,100)),fill='#E3B341')
    return im.resize((size,size),Image.Resampling.LANCZOS)

sizes=[16,20,24,32,40,48,64,128,256]
frames=[frame(size) for size in sizes]
frames[-1].save(assets/'timoa.ico',format='ICO',sizes=[(s,s) for s in sizes],append_images=frames[:-1])
frames[-1].save(assets/'icon-preview.png')
