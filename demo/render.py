"""Deterministic 18-second product showcase. Real app stills; no simulated clicks."""
from pathlib import Path
import argparse, hashlib, math, subprocess, wave, struct, json
from functools import lru_cache
from PIL import Image, ImageDraw, ImageFont, ImageOps
import imageio_ffmpeg
ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'docs/media'
WORK = ROOT / 'demo/render-work'
WORK.mkdir(exist_ok=True)
FF = imageio_ffmpeg.get_ffmpeg_exe()
FPS, SECONDS = 30, 18
REPO_URL = 'github.com/flyndagr/atlas-washi'
SEAL = json.loads((ROOT / 'assets/seal.json').read_text())
INK, PAPER, RED = '#36352b', '#f9f4e3', tuple(SEAL['color'])
TIMELINE = [(0,5,'A quiet place for your thoughts.','focus.png'),
            (5,10,'Fountain-pen feeling. Plain Markdown.','notebook.png'),
            (10,15,'Connect your ideas on a canvas.','canvas.png'),
            (15,18,'Built in Rust. Open source.','focus.png')]
def font(name,size): return ImageFont.truetype(str(ROOT/'assets'/name),size)
def draw_seal(im, center, width):
    # Fixed aspect ratio and supersampling keep the stamp identical in every format.
    k = width / SEAL['width'] * 4
    size = (round(SEAL['width'] * k), round(SEAL['height'] * k))
    mark = Image.new('RGBA', size)
    pen = ImageDraw.Draw(mark)
    inset = SEAL['inset'] * k
    pen.rounded_rectangle((inset, inset, size[0]-inset, size[1]-inset),
        radius=SEAL['radius']*k, outline=RED, width=round(SEAL['stroke']*k))
    face = font(SEAL['font'], round(SEAL['font_size']*k))
    bounds = pen.textbbox((0,0), SEAL['glyph'], font=face)
    pen.text(((size[0]-bounds[2]-bounds[0])/2, (size[1]-bounds[3]-bounds[1])/2),
        SEAL['glyph'], font=face, fill=RED)
    mark = mark.resize((round(size[0]/4),round(size[1]/4)), Image.Resampling.LANCZOS)
    im.paste(mark, (round(center[0]-mark.width/2),round(center[1]-mark.height/2)), mark)

@lru_cache(maxsize=1)
def washi_tile():
    """Atlas's src/washi.rs kozo pulp and fibers, at its default 0.7 grain."""
    n, strength, seed = 1024, .7, 712367
    def rand():
        nonlocal seed
        seed ^= (seed << 13) & 0xffffffff
        seed ^= seed >> 17
        seed ^= (seed << 5) & 0xffffffff
        return seed / 0xffffffff
    def pulp(x, y, cells, salt):
        def noise(x, y):
            v = ((x % cells) * 374761393 & 0xffffffff) ^ ((y % cells) * 668265263 & 0xffffffff) ^ salt
            v = ((v ^ (v >> 13)) * 1274126177) & 0xffffffff
            return (v ^ (v >> 16)) / 0xffffffff
        x, y = x*cells, y*cells
        ix, iy = int(x), int(y)
        u, v = x-ix, y-iy
        u, v = u*u*(3-2*u), v*v*(3-2*v)
        return (noise(ix,iy)*(1-u)+noise(ix+1,iy)*u)*(1-v) + (noise(ix,iy+1)*(1-u)+noise(ix+1,iy+1)*u)*v
    pixels = []
    for y in range(n):
        for x in range(n):
            cloud = sum(pulp(x/n,y/n,c,s)*weight for c,s,weight in [(8,93,.55),(31,721,.30),(113,8123,.15)])
            density = max(0,(cloud-.33)*35+rand()*8)
            pixels.append((113,95,63,int(density*strength)))
    for _ in range(4200):
        x,y,angle = rand()*n,rand()*n,rand()*math.tau
        length,bend,alpha = 3+rand()**2*38,(rand()-.5)*5,int((8+rand()*22)*strength)
        for step in range(int(length)):
            curve = math.sin(step/length*math.pi)*bend
            xx = int((x+math.cos(angle)*step-math.sin(angle)*curve)%n)
            yy = int((y+math.sin(angle)*step+math.cos(angle)*curve)%n)
            pixels[yy*n+xx] = (126,108,77,alpha)
            pixels[((yy+1)%n)*n+xx] = (255,255,255,alpha//2)
    texture = Image.new('RGBA',(n,n))
    texture.putdata(pixels)
    return Image.alpha_composite(Image.new('RGBA',(n,n),PAPER),texture).convert('RGB')

@lru_cache(maxsize=3)
def washi_background(w,h):
    # Fixed physical scale per format; identical stationary paper across every beat.
    side = round(768*w/1280)
    tile = washi_tile().resize((side,side),Image.Resampling.LANCZOS)
    paper = Image.new('RGB',(w,h),PAPER)
    for y in range(0,h,side):
        for x in range(0,w,side): paper.paste(tile,(x,y))
    return paper

def scene(index,w,h):
    im=washi_background(w,h).copy(); d=ImageDraw.Draw(im)
    scale=w/1280
    small=font('Inter-Regular.ttf',round(21*scale))
    title=font('CormorantGaramond.ttf',round(48*scale))
    d.text((w*.055,h*.045),'atlas  /  THE WASHI EDITION',font=small,fill=INK)
    d.line((w*.055,h*.11,w*.945,h*.11),fill='#d4c8ad',width=1)
    caption=TIMELINE[index][2]
    if h>w:
        lines=[['A quiet place','for your thoughts.'],['Fountain-pen feeling.','Plain Markdown.'],['Connect your ideas','on a canvas.'],['Built in Rust.','Open source.']][index]
        for j,line in enumerate(lines):d.text((w*.055,h*.15+j*round(62*scale)),line,font=title,fill=INK)
        box=(int(w*.055),int(h*.33),int(w*.945),int(h*.81))
    else:
        d.text((w*.055,h*.15),caption,font=title,fill=INK)
        box=(int(w*.055),int(h*.27),int(w*.945),int(h*.87))
    shot=Image.open(OUT/TIMELINE[index][3]).convert('RGB')
    if index==0:
        shot=shot.crop((550,140,2280,1150))
    elif index==2:
        shot=shot.crop((530,210,2290,1260))
    if index==3:
        # Final frame names the project and points viewers to the public source.
        d.text((w*.10,h*.36),'atlas',font=font('CormorantGaramond.ttf',round(150*scale)),fill=INK)
        d.text((w*.10,h*.59),'A notebook with room to think.',font=font('Inter-Regular.ttf',round(29*scale)),fill=INK)
        d.text((w*.10,h*.69),REPO_URL,font=font('Inter-Regular.ttf',round(25*scale)),fill=INK)
        draw_seal(im, (w*.86,h*.44), w*.08)
    else:
        bw,bh=box[2]-box[0],box[3]-box[1]
        shot=ImageOps.contain(shot,(bw,bh),Image.Resampling.LANCZOS)
        x=box[0]+(bw-shot.width)//2;y=box[1]+(bh-shot.height)//2
        d.rectangle((x-1,y-1,x+shot.width+1,y+shot.height+1),outline='#c9bea6',width=1)
        im.paste(shot,(x,y))
    d.text((w*.055,h*.925),'LOCAL NOTES  ·  LINKS  ·  CANVAS',font=small,fill=INK)
    for n in range(4):d.rectangle((w*.84+n*w*.027,h*.937,w*.859+n*w*.027,h*.941),fill=RED if n==index else '#c9bea6')
    return im

def frame(t,scenes):
    idx=next(i for i,(a,b,_,_) in enumerate(TIMELINE) if a<=min(t,17.999)<b)
    elapsed=t-TIMELINE[idx][0]
    if idx and elapsed<.45:
        a=elapsed/.45;a=a*a*(3-2*a)
        return Image.blend(scenes[idx-1],scenes[idx],a)
    return scenes[idx]

def soundtrack():
    rate=48000; data=bytearray()
    notes=[(0,293.66),(1.5,440),(3,587.33),(6,329.63),(7.5,440),(9,659.25),(12,293.66),(13.5,440),(15,587.33)]
    for n in range(rate*SECONDS):
        t=n/rate;v=0.
        for start,f in notes:
            x=t-start
            if 0<=x<5:
                env=(1-math.exp(-x*35))*math.exp(-x*1.1)
                v+=.15*env*(math.sin(2*math.pi*f*x)+.15*math.sin(2*math.pi*2*f*x))
        v*=min(t/1,1,(SECONDS-t)/1.5)
        data.extend(struct.pack('<h',int(max(-1,min(1,v))*32767)))
    path=WORK/'sound.wav'
    with wave.open(str(path),'wb') as f:f.setparams((1,2,rate,0,'NONE','not compressed'));f.writeframes(data)
    return path

def render(w,h,name,audio):
    scenes=[scene(i,w,h) for i in range(4)]
    scenes[3].save(OUT/(name+'-share.png'))
    assert hashlib.sha256(frame(7.25,scenes).tobytes()).digest()==hashlib.sha256(frame(7.25,scenes).tobytes()).digest()
    contact=Image.new('RGB',(780,round(h/w*390)*2),PAPER)
    for i,s in enumerate(scenes):
        thumb=s.resize((390,round(h/w*390)),Image.Resampling.LANCZOS)
        contact.paste(thumb,((i%2)*390,(i//2)*thumb.height))
    contact.save(OUT/(name+'-contact.jpg'),quality=92)
    cmd=[FF,'-y','-loglevel','error','-f','rawvideo','-pix_fmt','rgb24','-s',f'{w}x{h}','-r',str(FPS),'-i','-','-i',str(audio),'-af','loudnorm=I=-16:TP=-1.5:LRA=7','-c:v','libx264','-preset','fast','-crf','18','-pix_fmt','yuv420p','-c:a','aac','-b:a','192k','-t',str(SECONDS),'-movflags','+faststart',str(OUT/(name+'.mp4'))]
    proc=subprocess.Popen(cmd,stdin=subprocess.PIPE)
    for n in range(FPS*SECONDS):proc.stdin.write(frame(n/FPS,scenes).tobytes())
    proc.stdin.close()
    if proc.wait():raise RuntimeError('Encoding failed')
    print(name,'rendered',flush=True)
if __name__=='__main__':
    a=soundtrack()
    for w,h,name in [(1280,720,'atlas-demo'),(1080,1080,'atlas-demo-square'),(1080,1920,'atlas-demo-vertical')]:render(w,h,name,a)
