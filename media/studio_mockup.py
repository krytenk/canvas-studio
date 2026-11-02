"""Typed product geometry rendered locally through the same Three.js source as live preview."""
import json,shutil,zipfile
from pathlib import Path
from studio_motion import call,sha,write,VERSION

def run(w):
 s=w.spec;root=Path(s['directory']);m=s['scene'];width=m['output_width'];height=m['output_height'];movie=s['movie']
 # Rust installs these resources from the application binary; no CDN/network imports.
 runtime=Path(__file__).parent
 (root/'vendor').mkdir(exist_ok=True)
 for name in ['mockup-scene.js','vendor/three.module.js','vendor/three.core.js','vendor/THREE-LICENSE.txt']:shutil.copyfile(runtime/name,root/name)
 build="""import {createMockup} from './mockup-scene.js';
 const canvas=document.querySelector('canvas');
 window.__hf=window.__hf||{};window.__hf.buildReady=window.__hf.buildReady||{};
 window.__hf.buildReady.mockup=(async()=>{
  const spec=await(await fetch('./scene.json')).json();
  try{const scene=await createMockup(canvas,spec,'./assets/front.png',spec.back_page?'./assets/back.png':null);
   window.addEventListener('hf-seek',e=>scene.renderAt(e.detail.time));scene.renderAt(0);document.body.dataset.ready='true';
  }catch(e){document.body.dataset.error=e.message;throw e;}
 })();"""
 (root/'boot.js').write_text(build)
 (root/'index.html').write_text(f'''<!doctype html><html><head><meta charset="utf-8"><title>Canvas Studio · product scene</title><style>html,body{{margin:0;width:{width}px;height:{height}px;overflow:hidden}}canvas{{display:block;width:100%;height:100%}}</style></head><body><div id="root" data-composition-id="main" data-width="{width}" data-height="{height}" data-duration="6" data-fps="30" data-no-timeline><canvas data-layout-ignore></canvas></div><script type="module" src="boot.js"></script></body></html>''')
 write(root/'hyperframes.json',{'media':{'autoProxy':False}})
 cli=['node',s['cli']]
 # Enable software WebGL via HyperFrames; no physical GPU or subscription is needed.
 call(w,[*cli,'lint',str(root),'--json'])
 shots=w.folder/'poster';call(w,[*cli,'snapshot',str(root),'--at','0','--no-end','--output',str(shots),'--no-browser-gpu','--json'])
 images=list(shots.glob('*.png'))
 if len(images)!=1:raise ValueError('Expected one product poster')
 shutil.copyfile(images[0],w.folder/'frame.png')
 if movie:
  try:
   call(w,[*cli,'render',str(root),'--fps','30','--workers','1','--quality','delivery','--crf','20','--output',str(w.folder/'mockup.partial.mp4'),'--frames-cache-dir','off','--no-browser-gpu'])
   (w.folder/'mockup.partial.mp4').replace(w.folder/'movie.mp4')
  finally:(w.folder/'mockup.partial.mp4').unlink(missing_ok=True)
 lock=Path(s['cli']).parents[3]/'package-lock.json'
 if not lock.is_file():raise ValueError('Missing pinned motion renderer lock file')
 shutil.copyfile(lock,root/'package-lock.json')
 write(root/'package.json',{'private':True,'dependencies':{'hyperframes':VERSION},'scripts':{'preview':f'npx hyperframes@{VERSION} preview','render':f'npx hyperframes@{VERSION} render --fps 30 --workers 1 --quality delivery --crf 20 --no-browser-gpu --frames-cache-dir off --output movie.mp4'}})
 write(root/'manifest.json',{'schema':1,'engine':'three.js','three_version':'0.187.0','hyperframes_version':VERSION,'scene':m,'document_id':s['document_id'],'source_revision':s['source_revision'],'fingerprint':s['fingerprint'],'width':width,'height':height,'fps':30,'frames':180 if movie else 1,'seek':'Pure renderAt(t), six-second eased product reveal; no random or RAF clock','files':{p.relative_to(root).as_posix():sha(p) for p in root.rglob('*') if p.is_file() and '.hyperframes' not in p.parts}})
 with zipfile.ZipFile(w.folder/'motion-source.zip','w',zipfile.ZIP_DEFLATED) as z:
  for p in sorted(root.rglob('*')):
   if p.is_file() and '.hyperframes' not in p.parts and 'node_modules' not in p.parts:z.write(p,p.relative_to(root).as_posix())
 return {'engine':'three.js','three_version':'0.187.0','width':width,'height':height,'fps':30,'frames':180 if movie else 1,'duration':6 if movie else 0,'source_available':True,'scene_id':m['id'],'fingerprint':s['fingerprint']}
