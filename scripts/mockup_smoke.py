"""Real local Three.js rendering, document guards, portable posters and Film handoff.
--chrome exercises isolated CI UI and deterministic WebGL seeks; never a personal profile.
"""
import argparse,gzip,hashlib,json,os,shutil,time,zipfile
from pathlib import Path
from PIL import Image,ImageChops,ImageStat
from mcp_smoke import Client
from media_smoke import launch,wait,probe,run_cmd
ROOT=Path(__file__).resolve().parents[1]
def build_cover(c):
 def edit(cmds):return c.call('studio_edit',{**c.guard(),'commands':cmds})
 c.call('studio_create',{**c.guard(),'width':720,'height':1080})
 edit([{'action':'title','value':'Product studio · Fieldwork collection'},
 {'action':'background','page':0,'color':'#173e35'},
 {'action':'add_text','page':0,'name':'Imprint','content':'STUDIO EDITIONS     /     VOL. 01','size':13,'tracking':2,'color':'#c5e3ba','x':65,'y':70,'width':590,'height':40},
 {'action':'add_text','page':0,'name':'Book title','content':'FIELD\nNOTES','size':92,'family':'DisplaySerif','color':'#f6edda','leading':.97,'x':60,'y':175,'width':600,'height':280},
 {'action':'add_shape','page':0,'shape':'Circle','color':'#c6dca2','x':72,'y':580,'width':570,'height':380},
 {'action':'add_shape','page':0,'shape':'Circle','color':'#173e35','x':130,'y':625,'width':455,'height':285},
 {'action':'add_text','page':0,'content':'A PLACE FOR\nYOUR NEXT CHAPTER.','size':17,'leading':1.35,'color':'#e6dfce','x':68,'y':473,'width':600,'height':90},
 {'action':'add_text','page':0,'content':'OBSERVE. COLLECT. CREATE.','size':12,'tracking':2,'color':'#d9e7c1','x':68,'y':1000,'width':600,'height':40},
 {'action':'add_page'}, {'action':'background','page':1,'color':'#f0ece3'}])
 edit([{'action':'mockup_create','kind':kind,'page':0} for kind in ('book','box','cards')])
 return c.call('studio_document')['mockups']
def run(binary,chrome=None):
 qa=ROOT/'qa/mockups';qa.mkdir(parents=True,exist_ok=True);work=qa/'workspace';work.mkdir(exist_ok=True)
 os.environ['CANVAS_STUDIO_HYPERFRAMES']=str(ROOT/'tools/hyperframes/node_modules/hyperframes/bin/hyperframes.mjs')
 folder='data-'+str(time.time_ns());server,log,connection,url=launch(binary,qa,folder);c=Client(work,connection)
 try:
  def edit(cmds,error=False):return c.call('studio_edit',{**c.guard(),'commands':cmds},error=error)
  scenes=build_cover(c);book=scenes[0]['id'];box=scenes[1]['id'];cards=scenes[2]['id']
  edit([{'action':'mockup_update','id':s['id'],'output_width':320,'output_height':320} for s in scenes])
  before=c.guard();edit([{'action':'title','value':'Invalid batch'},{'action':'mockup_update','id':book,'depth':-1}],True);assert c.guard()==before
  def render(id,movie=False):return c.call('studio_mockup_render',{**c.guard(),'id':id,'movie':movie})
  def save(j,name):wait(c,j,timeout=600);return c.call('studio_media_job',{**c.guard(),'id':j['id'],'operation':'save','path':name,'overwrite':True})
  renders={}
  for key,id in [('book',book),('box',box),('cards',cards)]:
   j=render(id);save(j,key+'.png');renders[key]=j
   image=Image.open(work/(key+'.png'));assert image.size==(320,320);assert max(ImageStat.Stat(image.convert('RGB')).stddev)>25,'Blank product render'
  again=render(book);save(again,'book-repeat.png');assert ImageChops.difference(Image.open(work/'book.png'),Image.open(work/'book-repeat.png')).getbbox() is None,'Repeated product poster changed'
  g=c.guard();edit([{'action':'mockup_place','id':book,'job':renders['book']['id'],'page':0}],True);assert c.guard()==g
  # An unrelated document edit is allowed; an edited cover/camera is refused.
  edit([{'action':'title','value':'Product studio · Fieldwork collection'}]);edit([{'action':'mockup_place','id':book,'job':renders['book']['id'],'page':1}])
  c.call('studio_export',{**c.guard(),'path':'poster-proof.pdf','format':'pdf','overwrite':True})
  edit([{'action':'mockup_update','id':book,'yaw':-35}]);edit([{'action':'mockup_place','id':book,'job':renders['book']['id'],'page':1}],True)
  edit([{'action':'mockup_update','id':book,'yaw':-28}]);edit([{'action':'background','page':0,'color':'#244739'}]);edit([{'action':'mockup_place','id':book,'job':renders['book']['id'],'page':1}],True)
  edit([{'action':'background','page':0,'color':'#173e35'}]);movie=render(book,True);save(movie,'book-reveal.mp4')
  meta=probe(work/'book-reveal.mp4');video=next(s for s in meta['streams'] if s['codec_type']=='video');assert int(video['nb_frames'])==180 and video['r_frame_rate']=='30/1'
  # Motion is visible and first-frame poster uses the same art direction (lossy MP4 tolerance).
  run_cmd(['ffmpeg','-v','error','-y','-i',str(work/'book-reveal.mp4'),'-frames:v','1',str(work/'movie-first.png')])
  diff=ImageStat.Stat(ImageChops.difference(Image.open(work/'movie-first.png').convert('RGB'),Image.open(work/'book.png').convert('RGB')));assert max(diff.mean)<5,diff.mean
  raw=run_cmd(['ffmpeg','-v','error','-i',str(work/'book-reveal.mp4'),'-vf','select=eq(n\\,90)','-frames:v','1','-f','rawvideo','-pix_fmt','rgb24','-']);assert raw!=Image.open(work/'movie-first.png').convert('RGB').tobytes()
  edit([{'action':'film_setup','width':320,'height':320,'fps':30,'duration':180},{'action':'mockup_film','id':book,'job':movie['id'],'start':0}]);assert len(c.call('studio_document')['timeline']['clips'])==1
  c.call('studio_media_job',{**c.guard(),'id':movie['id'],'operation':'source','path':'product-source.zip','overwrite':True})
  with zipfile.ZipFile(work/'product-source.zip') as z:
   manifest=json.loads(z.read('manifest.json'));assert manifest['three_version']=='0.187.0'
   for name,digest in manifest['files'].items():assert hashlib.sha256(z.read(name)).hexdigest()==digest,name
   assert 'MIT License' in z.read('vendor/THREE-LICENSE.txt').decode()
  cancel=render(book,True);c.call('studio_media_job',{**c.guard(),'id':cancel['id'],'operation':'cancel'});wait(c,cancel,('cancelled',))
  c.call('studio_export',{**c.guard(),'path':'product.canvas','format':'canvas','overwrite':True})
  data=(work/'product.canvas').read_bytes();env=json.loads(gzip.decompress(data[len(b'CANVAS-STUDIO-PACKAGE\n1\n'): ]));assert env['version']==5 and 'product-mockups-v1' in env['required_features']
  before=c.call('studio_document')['mockups'];c.call('studio_open',{**c.guard(),'path':'product.canvas'});assert c.call('studio_document')['mockups']==before
  # A fresh process/storage directory must recover both the portable image and movie assets.
  restored=work/'cold';restored.mkdir(exist_ok=True);shutil.copyfile(work/'product.canvas',restored/'product.canvas')
  p2,l2,con2,_=launch(binary,qa,'cold-'+str(time.time_ns()),restored/'product.canvas');c2=Client(restored,con2)
  try:
   assert c2.call('studio_document')['mockups']==before
   c2.call('studio_export',{**c2.guard(),'page':1,'path':'poster.png','format':'png','dpi':96,'overwrite':True})
   assert Image.open(restored/'poster.png').size==(720,1080)
  finally:c2.close();p2.terminate();p2.wait(timeout=10);l2.close()
  if chrome:
   from playwright.sync_api import sync_playwright
   with sync_playwright() as pw:
    browser=pw.chromium.launch(channel=chrome,headless=True,args=['--enable-unsafe-swiftshader']);page=browser.new_page(viewport={'width':1512,'height':1050});errors=[];page.on('pageerror',lambda e:errors.append(str(e)));page.goto(url);page.get_by_role('button',name='Product mockups',exact=True).click();page.get_by_text('Live Three.js ·',exact=False).wait_for();page.locator('.mockup-job').first.wait_for();assert not page.locator('#message.error').is_visible()
    canvas=page.locator('#mockup-canvas');count=canvas.get_attribute('data-render-count');page.wait_for_timeout(400);assert count==canvas.get_attribute('data-render-count'),'Idle WebGL render loop'
    page.locator('#mockup-time').fill('1');one=canvas.screenshot();page.locator('#mockup-time').fill('4');four=canvas.screenshot();page.locator('#mockup-time').fill('1');assert canvas.screenshot()==one and one!=four,'Out-of-order Three.js seeks are not deterministic'
    page.get_by_label('Turn °',exact=True).fill('-36');page.get_by_role('button',name='Apply scene',exact=True).click();page.wait_for_function("()=>state.mockups[0].yaw===-36");page.get_by_text('Live Three.js ·',exact=False).wait_for();page.screenshot(path=str(qa/'mockup-ui.png'))
    page.get_by_role('button',name='Back to design',exact=True).click();assert page.get_by_role('button',name='Product mockups',exact=True).is_visible();assert not errors,errors;browser.close()
    fallback=pw.chromium.launch(channel=chrome,headless=True,args=['--disable-webgl']);fp=fallback.new_page();fp.goto(url);fp.get_by_role('button',name='Product mockups',exact=True).click();fp.get_by_text('Preview unavailable',exact=True).wait_for();assert fp.get_by_role('button',name='Render listing image',exact=True).is_enabled();fallback.close()
  report={'book_box_cards':'PASS: actual locally rendered Three.js PNGs','repeat_poster_pixels':'PASS','poster_movie_frame_mean_error':diff.mean,'movie_frames':180,'movie_film_handoff':'PASS','portable_v5_cold_restore':'PASS','source_hashes_and_license':'PASS','same_cover_recursion_and_stale_artwork_camera':'PASS','cancel':'PASS','browser_seek_idle_fallback':'PASS' if chrome else 'CI only'}
  (qa/'report.json').write_text(json.dumps(report,indent=2));print(json.dumps(report,indent=2))
 finally:c.close();server.terminate();server.wait(timeout=10);log.close()
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('--binary',default='./canvas-studio');p.add_argument('--chrome');a=p.parse_args();run(Path(a.binary).resolve(),a.chrome)
