"""Create a public-safe original art-direction demonstration through the real agent API."""
import argparse,json,os,shutil,time
from pathlib import Path
from mcp_smoke import Client
from media_smoke import launch,wait
from mockup_smoke import build_cover
ROOT=Path(__file__).resolve().parents[1]
def run(binary):
 qa=ROOT/'qa/mockup-demo';qa.mkdir(parents=True,exist_ok=True);work=qa/'workspace';work.mkdir(exist_ok=True)
 os.environ['CANVAS_STUDIO_HYPERFRAMES']=str(ROOT/'tools/hyperframes/node_modules/hyperframes/bin/hyperframes.mjs')
 server,log,connection,_=launch(binary,qa,'data-'+str(time.time_ns()));c=Client(work,connection)
 try:
  def edit(cmds):return c.call('studio_edit',{**c.guard(),'commands':cmds})
  scenes=build_cover(c);edit([{'action':'add_page'},{'action':'add_page'}])
  for page,title,bg,ink,accent,heading,strap in [(2,'Slow Sunday · product box','#e9b99d','#34372c','#b6543c','SLOW\nSUNDAY','THE SMALL PLEASURES COLLECTION'),(3,'Good Things · stationery','#e9e8d7','#244b3d','#bed7b2','GOOD\nTHINGS\nAHEAD.','A NOTE FROM THE STUDIO')]:
   pid=c.call('studio_document')['pages'][page]['id']
   edit([{'action':'page_title','id':pid,'value':title},{'action':'background','page':page,'color':bg},
   {'action':'add_text','page':page,'content':strap,'size':13,'tracking':2,'color':ink,'x':65,'y':70,'width':590,'height':40},
   {'action':'add_text','page':page,'content':heading,'size':83,'family':'DisplaySerif','color':ink,'leading':1.02,'x':60,'y':170,'width':600,'height':400},
   {'action':'add_shape','page':page,'shape':'Circle','color':accent,'x':145,'y':645,'width':430,'height':240},
   {'action':'add_text','page':page,'content':'CONSIDERED DESIGN.\nMADE TO BE KEPT.','size':16,'tracking':1,'color':ink,'x':68,'y':945,'width':600,'height':85}])
  for scene,page,bg,side in [(scenes[0],0,'#e8e4dc','#163e35'),(scenes[1],2,'#e5d6c7','#b85a40'),(scenes[2],3,'#ccd8cf','#e8e7d7')]:
   edit([{'action':'mockup_update','id':scene['id'],'page':page,'width':1.5,'height':2.25,'background':bg,'cover_color':side,'output_width':1200,'output_height':1200}])
  files=[]
  for scene,name in zip(scenes,['Field Notes book','Slow Sunday box','Good Things stationery']):
   j=c.call('studio_mockup_render',{**c.guard(),'id':scene['id']});wait(c,j,timeout=600)
   c.call('studio_media_job',{**c.guard(),'id':j['id'],'operation':'save','path':name+'.png','overwrite':True});files.append(name+'.png')
   if scene==scenes[0]:edit([{'action':'mockup_place','id':scene['id'],'job':j['id'],'page':1}])
  j=c.call('studio_mockup_render',{**c.guard(),'id':scenes[0]['id'],'movie':True});wait(c,j,timeout=600)
  c.call('studio_media_job',{**c.guard(),'id':j['id'],'operation':'save','path':'Field Notes reveal.mp4','overwrite':True});files.append('Field Notes reveal.mp4')
  c.call('studio_media_job',{**c.guard(),'id':j['id'],'operation':'source','path':'Book reveal source.zip','overwrite':True});files.append('Book reveal source.zip')
  edit([{'action':'film_setup','width':1200,'height':1200,'fps':30,'duration':180},{'action':'mockup_film','id':scenes[0]['id'],'job':j['id'],'start':0}])
  c.call('studio_export',{**c.guard(),'path':'Product studio demo.canvas','format':'canvas','overwrite':True});files.append('Product studio demo.canvas')
  c.call('studio_export',{**c.guard(),'path':'Poster print proof.pdf','format':'pdf','overwrite':True});files.append('Poster print proof.pdf')
  durable=ROOT/'StudioProjects/Product Studio 1.15 Demo';durable.mkdir(parents=True,exist_ok=True)
  for file in files:shutil.copy2(work/file,durable/file)
  (durable/'README.txt').write_text('Canvas Studio 1.15 — original fictional product artwork. No real brand or endorsement.\nOpen Product studio demo.canvas. Browser: Mockups. Native: Tools / Product studio 3D.\nThree editable cover pages, three saved scenes, an independent poster page, and a 6-second book reveal in Film.\nPNG posters and the print proof are snapshots; update artwork then re-render to refresh.\nThe source ZIP contains a separately editable code scene and native master with pinned renderer and Three.js MIT license.\nNo paid API or asset credits used.\n')
  print('Saved demonstration:',durable)
 finally:c.close();server.terminate();server.wait(timeout=10);log.close()
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('--binary',default='./canvas-studio');a=p.parse_args();run(Path(a.binary).resolve())
