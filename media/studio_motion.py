"""Canvas Studio → HyperFrames adapter. No remote media, arbitrary code or shell input."""
import copy,hashlib,html,json,math,os,shutil,signal,subprocess,time,zipfile
from pathlib import Path
VERSION='0.8.206'
# The same pure pose sampler drives every exported composition and random-time seek.
POSE_JS='''function pose(m,frame){if(!m)return{x:0,y:0,scale:1,rotation:0,opacity:1};const keys=m.keys,f=frame-m.origin;if(f<=keys[0].frame)return keys[0];for(let i=0;i<keys.length-1;i++){const a=keys[i],b=keys[i+1];if(f<b.frame){let u=(f-a.frame)/(b.frame-a.frame);if(a.ease==='hold')u=0;else if(a.ease==='ease-in')u=u*u*u;else if(a.ease==='ease-out')u=1-Math.pow(1-u,3);else if(a.ease==='ease-in-out')u=u<.5?4*u*u*u:1-Math.pow(-2*u+2,3)/2;const out={};for(const k of ['x','y','scale','rotation','opacity'])out[k]=a[k]+(b[k]-a[k])*u;return out;}}return keys[keys.length-1];}'''
def sha(path):
 h=hashlib.sha256()
 with path.open('rb') as f:
  for data in iter(lambda:f.read(1048576),b''):h.update(data)
 return h.hexdigest()
def write(path,value):path.write_text(json.dumps(value,indent=2,ensure_ascii=False,allow_nan=False))
def call(w,args,cwd=None):
 from studio_media import Cancelled,atomic
 log=w.folder/'hyperframes.log';env=os.environ.copy();env.update(HYPERFRAMES_NO_TELEMETRY='1',HYPERFRAMES_RENDER_DETACHED='1',HYPERFRAMES_SKIP_SKILLS='1',HYPERFRAMES_NO_UPDATE_CHECK='1')
 with log.open('ab') as output:
  log_start=log.stat().st_size
  p=subprocess.Popen(args,cwd=cwd,env=env,stdin=subprocess.DEVNULL,stdout=output,stderr=subprocess.STDOUT,start_new_session=True);began=time.monotonic()
  try:
   while p.poll() is None:
    if w.cancel.exists():raise Cancelled()
    if time.monotonic()-began>900:raise ValueError('Motion job exceeded 15 minutes')
    state=dict(state='running',phase='hyperframes',progress=0,elapsed_seconds=round(time.monotonic()-began),worker_pid=os.getpid())
    with log.open('rb') as progress_log:
     progress_log.seek(max(log_start,log.stat().st_size-8192))
     for line in progress_log.read().decode(errors='replace').splitlines():
      if line.startswith('@hf-progress '):
       try:
        event=json.loads(line[13:]);state['phase']=event.get('code','hyperframes');state['progress']=min(.99,max(0,float(event.get('pct',0))/100));state['measured_progress']=True
       except (ValueError,TypeError):pass
    atomic(w.progress,state)
    time.sleep(.2)
   if p.returncode:raise ValueError('HyperFrames: '+log.read_text(errors='replace')[-3500:])
  finally:
   if p.poll() is None:
    os.killpg(p.pid,signal.SIGTERM)
    try:p.wait(timeout=4)
    except subprocess.TimeoutExpired:os.killpg(p.pid,signal.SIGKILL);p.wait()
def build(w):
 from studio_media import Worker,FORMATS,atomic
 s=w.spec;t=s['timeline'];root=Path(s['directory']);assets=root/'assets';comps=root/'compositions';comps.mkdir(exist_ok=True);assets.mkdir(exist_ok=True)
 mode=s['mode'];preview=mode in ('preview','frame');factor=min(1,640/max(t['width'],t['height'])) if preview else 1
 width=max(2,round(t['width']*factor/2)*2);height=max(2,round(t['height']*factor/2)*2);fps=t['fps'];seconds=t['duration']/fps
 cli=Path(s['cli']);version=subprocess.check_output(['node',str(cli),'--version'],env={**os.environ,'HYPERFRAMES_NO_TELEMETRY':'1'},timeout=15,text=True).strip()
 if version!=VERSION:raise ValueError(f'Motion needs pinned HyperFrames {VERSION}; installed {version}')
 clips=[c for tr in t['tracks'] if tr['enabled'] for c in t['clips'] if c['track']==tr['id'] and (mode!='frame' or c['start']<=s['frame']<c['start']+c['duration'])]
 prepared={}
 for c in clips:
  if w.cancel.exists():
   from studio_media import Cancelled
   raise Cancelled()
  a=t['assets'].get(c.get('asset'));source=Path(s['sources'][c['id']]);track=next(tr for tr in t['tracks'] if tr['id']==c['track'])
  if track['kind']=='audio':continue
  if a and a['kind']=='video':
   dest=assets/f"video-{c['id']}.mp4";offset=c.get('source_offset_frames',0);anchor=c.get('source_anchor_us',c['source_in_us'])/1e6
   length=c['duration']
   if mode=='frame':offset+=s['frame']-c['start'];length=1;dest=dest.with_suffix('.png')
   filters=f"setpts=PTS-STARTPTS,fps={fps},trim=start_frame={offset}:end_frame={offset+length},tpad=stop_mode=clone:stop={length},trim=end_frame={length},setpts=N/({fps}*TB),scale={max(width,height)}:{max(width,height)}:force_original_aspect_ratio=decrease:force_divisible_by=2"
   w.ffmpeg(['-ss',f'{anchor:.9f}','-protocol_whitelist','file,pipe','-f',FORMATS['.'+a['extension']],'-i',str(source),'-an','-vf',filters,'-frames:v',str(length),*(['-update','1'] if mode=='frame' else ['-c:v','libx264','-preset','veryfast','-crf','18','-pix_fmt','yuv420p','-movflags','+faststart']),str(dest)],c['duration']/fps)
  elif a:
   dest=assets/f"image-{c['id']}.{a['extension']}";shutil.copyfile(source,dest)
  else:
   svg=source.with_suffix('.svg');dest=svg if svg.exists() else source
  prepared[c['id']]=dest.relative_to(root).as_posix()
 # Reuse the established, sample-accurate native audio graph; only its picture is suppressed.
 audio_t=copy.deepcopy(t);audio_t.update(width=64,height=64)
 for tr in audio_t['tracks']:tr['kind']='audio'
 audio_t['clips']=[c for c in audio_t['clips'] if c.get('asset') and t['assets'][c['asset']]['has_audio'] and c['audio']['enabled']]
 audio_path=assets/'audio-mix.wav';audio_job=w.folder/'audio';audio_job.mkdir(exist_ok=True)
 audio_spec={'op':'render','timeline':audio_t,'sources':s['sources'],'mode':'export','frame':None,'output':str(audio_path),'audio_only':True}
 if mode!='frame':
  mix=Worker(audio_spec,audio_job/'result.json',w.cancel);mix.render()
 hosts=[]
 for order,c in enumerate(clips):
  if c['id'] not in prepared:continue
  ident='clip-'+c['id'];duration=c['duration']/fps;start=c['start']/fps;motion=c.get('motion');x,y,cw,ch=c['rect']
  tag='video' if prepared[c['id']].endswith('.mp4') else 'img'
  attrs=' muted playsinline' if tag=='video' else f' alt="{html.escape(c["name"],quote=True)}"'
  media=f'<{tag} id="{ident}-media" class="clip" data-start="0" data-duration="{duration:.9f}" src="{prepared[c["id"]]}"{attrs} style="width:100%;height:100%;object-fit:{c["fit"]}"'+('></video>' if tag=='video' else '>')
  model=json.dumps(motion,separators=(',',':')).replace('<','\\u003c');opacity=c['opacity']
  scene=f'''<template><style>#root{{position:absolute;inset:0}}#{ident}-visual{{position:absolute;left:{x*factor}px;top:{y*factor}px;width:{cw*factor}px;height:{ch*factor}px;transform-origin:50% 50%}}</style>
<div id="root" data-composition-id="{ident}" data-width="{width}" data-height="{height}" data-no-timeline><div id="{ident}-visual" data-layout-allow-overflow>{media}</div></div>
<script>(()=>{{{POSE_JS} const layer=document.getElementById('{ident}-visual');const motion={model};function renderAt(time){{const p=pose(motion,time*{fps});layer.style.transform=`translate(${{p.x*{factor}}}px,${{p.y*{factor}}}px) rotate(${{p.rotation}}deg) scale(${{p.scale}})`;layer.style.opacity=String(p.opacity*{opacity});}}window.addEventListener('hf-seek',e=>renderAt(e.detail.time));renderAt(0);}})();</script></template>'''
  (comps/f'{ident}.html').write_text(scene)
  hosts.append(f'<div id="{ident}" class="clip" data-composition-id="{ident}" data-composition-src="compositions/{ident}.html" data-start="{start:.9f}" data-duration="{duration:.9f}" data-width="{width}" data-height="{height}" data-no-timeline data-track-index="{order+1}" data-track-kind="graphics" style="z-index:{order+1}"></div>')
 bg='#'+''.join(f'{n:02x}' for n in t['background'][:3])
 audio_markup='' if mode=='frame' else f'<audio id="master-audio" class="clip" data-start="0" data-duration="{seconds:.9f}" data-volume="1" src="assets/audio-mix.wav"></audio>'
 (root/'index.html').write_text(f'''<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width={width},height={height}"><title>Canvas Studio motion</title><style>*{{box-sizing:border-box}}html,body{{margin:0;width:{width}px;height:{height}px;overflow:hidden}}#root{{position:relative;width:100%;height:100%}}#background{{position:absolute;inset:0;background:{bg}}}</style></head><body><div id="root" data-composition-id="main" data-width="{width}" data-height="{height}" data-fps="{fps}" data-duration="{seconds:.9f}" data-start="0" data-no-timeline><div id="background" data-layout-ignore></div>{''.join(hosts)}{audio_markup}</div></body></html>''')
 write(root/'hyperframes.json',{'paths':{'blocks':'compositions','components':'compositions/components','assets':'assets'},'media':{'autoProxy':False}})
 write(root/'package.json',{'private':True,'scripts':{'preview':f'npx hyperframes@{VERSION} preview','check':f'npx hyperframes@{VERSION} check --json','render':f'npx hyperframes@{VERSION} render --workers 1 --fps {fps} --quality {'draft' if preview else 'delivery'} --crf 20 --no-browser-gpu --frames-cache-dir off --output movie.mp4'},'dependencies':{'hyperframes':VERSION}})
 lock=cli.parents[3]/'package-lock.json'
 if not lock.is_file():raise ValueError('Pinned motion lockfile is missing. Reinstall with scripts/setup_motion.py')
 shutil.copyfile(lock,root/'package-lock.json')
 (root/'README.txt').write_text('Canvas Studio motion source. Open editable-master.canvas in Canvas Studio to edit text, artwork and keyframes; re-export to regenerate these scenes. Scene transforms and SVG text are editable source. External HTML edits do not import back into Canvas Studio. Run npm ci, then npm run preview/check/render. The supplied video clips are frame-normalized; the WAV uses the native audio mix. All assets are local. See manifest.json for source timing and versions. No hosted service is required.\n')
 manifest={'schema':1,'engine':'hyperframes','engine_version':VERSION,'studio_version':s.get('studio_version'),'document_id':s['document_id'],'source_revision':s['source_revision'],'width':width,'height':height,'fps':fps,'frames':t['duration'],'timeline':t,'assets':{p.relative_to(root).as_posix():sha(p) for p in assets.iterdir() if p.is_file()},'audio':'assets/audio-mix.wav','preview':preview,'node':subprocess.check_output(['node','--version'],text=True).strip(),'ffmpeg':subprocess.check_output(['ffmpeg','-version'],text=True).splitlines()[0]}
 license_file=Path(__file__).with_name('Noto-OFL.txt')
 if license_file.exists():shutil.copyfile(license_file,root/'Noto-OFL.txt')
 manifest['scope']='frame' if mode=='frame' else 'movie'
 manifest['render_settings']={'crf':20,'workers':1,'software_gpu':True,'quality':'draft' if preview else 'delivery'}
 try:
  browser=subprocess.check_output(['node',str(cli),'browser','path'],env={**os.environ,'HYPERFRAMES_NO_TELEMETRY':'1','HYPERFRAMES_NO_UPDATE_CHECK':'1'},text=True,timeout=15).strip()
  manifest['chromium']=subprocess.check_output([browser,'--version'],text=True,timeout=10).strip()
 except Exception:manifest['chromium']='unavailable; run the motion setup helper'
 manifest['files']={p.relative_to(root).as_posix():sha(p) for p in root.rglob('*') if p.is_file() and p.name!='manifest.json' and '.hyperframes' not in p.parts and 'node_modules' not in p.parts}
 write(root/'manifest.json',manifest)
 return root,width,height

def _run(w):
 s=w.spec;root,width,height=build(w);cli=['node',s['cli']];mode=s['mode'];fps=s['timeline']['fps']
 # Lint every generated source; a faulty adapter must not emit a plausible bad movie.
 call(w,[*cli,'lint',str(root),'--json'])
 if mode=='frame':
  shots=w.folder/'frame-review';call(w,[*cli,'snapshot',str(root),'--at',f"{s['frame']/fps:.9f}",'--no-end','--output',str(shots),'--no-browser-gpu','--json'])
  images=list(shots.glob('*.png'))
  if len(images)!=1:raise ValueError('Expected one exact-time HyperFrames frame')
  shutil.copyfile(images[0],w.folder/'frame.png')
 elif mode!='source':
  call(w,[*cli,'check',str(root),'--json','--samples','3','--timeout','45000'])
  partial=w.folder/'motion.partial.mp4';call(w,[*cli,'render',str(root),'--fps',str(fps),'--workers','1','--quality','draft' if mode=='preview' else 'delivery','--crf','20','--output',str(partial),'--frames-cache-dir','off','--no-browser-gpu'])
  if partial.stat().st_size>128*1024*1024:raise ValueError('Motion output exceeds 128 MiB')
  os.replace(partial,w.folder/'movie.mp4')
 if mode=='frame':return {'engine':'hyperframes','engine_version':VERSION,'width':width,'height':height,'fps':fps,'frames':1,'duration':1/fps,'source_available':False}
 # Reproducible source snapshot: no caches, local paths or runtime credentials.
 archive=w.folder/'motion-source.zip'
 with zipfile.ZipFile(archive,'w',zipfile.ZIP_DEFLATED) as z:
  for p in sorted(root.rglob('*')):
   if p.is_file() and '.hyperframes' not in p.parts and 'node_modules' not in p.parts:z.write(p,p.relative_to(root).as_posix())
 if archive.stat().st_size>128*1024*1024:raise ValueError('Motion source exceeds 128 MiB')
 return {'engine':'hyperframes','engine_version':VERSION,'width':width,'height':height,'fps':fps,'frames':1 if mode=='frame' else s['timeline']['duration'],'duration':1/fps if mode=='frame' else s['timeline']['duration']/fps,'source_available':True}

def run(w):
 try:return _run(w)
 finally:(w.folder/'motion.partial.mp4').unlink(missing_ok=True)
