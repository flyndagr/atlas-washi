"""Original Atlas washi icon. Standard-library rasterization of an enso and pen nib."""
from pathlib import Path
import math, struct, zlib, random
size = 1024
rng = random.Random(432)
rows = []
for y in range(size):
    row = bytearray()
    for x in range(size):
        dx=max(224-x,0,x-800); dy=max(224-y,0,y-800)
        c=(0,0,0,0)
        if dx*dx+dy*dy < 160*160:
            grain=rng.randrange(-3,4)
            c=(244+grain,237+grain,216+grain,255)
            px=x-502; py=y-500
            a=math.atan2(py,px)%(math.pi*2)
            rr=math.hypot(px,py)
            radius=281+math.sin(a*5)*3+math.sin(a*19)*1.2
            thickness=13+20*abs(math.sin(a/2))
            if 0.25 < a < 6.13 and abs(rr-radius)<thickness:
                c=(58+grain,57+grain,47+grain,255)
            # Original pointed nib, split and breather hole.
            t=(y-327)/350
            width=102*t if t<0.67 else 68*(1-t)/0.33
            if 0<t<1 and abs(x-502)<width: c=(156,64,46,255)
            if 334<y<565 and abs(x-502)<3: c=(245,237,217,255)
            if (x-502)**2+(y-565)**2<11**2: c=(245,237,217,255)
        row.extend(c)
    rows.append(b'\0'+row)
def chunk(tag,data):
    return struct.pack('>I',len(data))+tag+data+struct.pack('>I',zlib.crc32(tag+data)&0xffffffff)
Path('assets/Atlas.png').write_bytes(b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',size,size,8,6,0,0,0))+chunk(b'IDAT',zlib.compress(b''.join(rows)))+chunk(b'IEND',b''))
