"""HyperFrames acceptance on offline generated sources; public MCP only.
The optional CI browser uses an isolated profile, never an existing user tab.
"""
import argparse,gzip,hashlib,io,json,os,subprocess,time,zipfile
from pathlib import Path
from PIL import Image,ImageDraw
from mcp_smoke import Client
from media_smoke import launch,wait,probe,pcm,run_cmd
ROOT=Path(__file__).resolve().parents[1]
def raw(path):return run_cmd(['ffmpeg','-v','error','-i',str(path),'-map','0:v:0','-f','rawvideo','-pix_fmt','rgb24','-'])
def run(binary,chrome=None):
    qa=ROOT/'qa/motion';qa.mkdir(parents=True,exist_ok=True);work=qa/'workspace';work.mkdir(exist_ok=True)
    os.environ['CANVAS_STUDIO_HYPERFRAMES']=str(ROOT/'tools/hyperframes/node_modules/hyperframes/bin/hyperframes.mjs')
    im=Image.new('RGBA',(320,180),(0,0,0,0));ImageDraw.Draw(im).rectangle((10,70,30,90),fill=(255,0,0,255));im.save(work/'marker.png')
    run_cmd(['ffmpeg','-v','error','-y','-f','lavfi','-i',r'aevalsrc=if(between(t\,2\,2.08)\,0.6*sin(2*PI*1000*t)\,0):s=48000:d=4','-c:a','pcm_s16le',str(work/'cue.wav')])
    folder='data-'+str(time.time_ns());server,log,connection,url=launch(binary,qa,folder);c=Client(work,connection);report={}
    try:
        def edit(commands,**kw):return c.call('studio_edit',{**c.guard(),'commands':commands,**kw})
        def render(mode='export',**kw):return c.call('studio_media_render',{**c.guard(),'mode':mode,**kw})
        def save(job,name,operation='save'):wait(c,job,timeout=240);return c.call('studio_media_job',{**c.guard(),'id':job['id'],'operation':operation,'path':name,'overwrite':True})
        c.call('studio_create',{**c.guard(),'width':320,'height':180});edit([{'action':'film_setup','width':320,'height':180,'fps':30,'duration':120,'background':'#000000'}])
        a=c.call('studio_import',{**c.guard(),'kind':'media','path':'marker.png'})['asset'];b=c.call('studio_import',{**c.guard(),'kind':'media','path':'cue.wav'})['asset']
        edit([{'action':'film_add','asset':a['id'],'start':30,'duration':90}]);clip=c.call('studio_document')['timeline']['clips'][0]
        keys=[dict(frame=0,x=0,y=0,scale=1,rotation=0,opacity=1,ease='linear'),dict(frame=30,x=100,y=0,scale=1,rotation=0,opacity=1,ease='linear'),dict(frame=89,x=0,y=0,scale=1,rotation=0,opacity=0.6,ease='linear')]
        edit([{'action':'film_motion','id':clip['id'],'keys':keys},{'action':'film_add','asset':b['id'],'start':0,'duration':120}])
        g=c.guard();bad=[dict(k) for k in keys];bad[1]['frame']=0;c.call('studio_edit',{**g,'commands':[{'action':'film_motion','id':clip['id'],'keys':bad}]},error=True);assert c.guard()==g
        frame=render('frame',frame=45);save(frame,'frame.png');image=Image.open(work/'frame.png').convert('RGB');pixels=image.load();red=[x for y in range(image.height) for x in range(image.width) if pixels[x,y][0]>200 and pixels[x,y][1]<50];assert (min(red),max(red))==(60,80),(min(red),max(red))
        first=render();save(first,'first.mp4');p=probe(work/'first.mp4');v=next(s for s in p['streams'] if s['codec_type']=='video');assert int(v['nb_frames'])==120 and abs(float(v['duration'])-4)<.002,v
        samples=pcm(work/'first.mp4');peaks=[i for i,s in enumerate(samples) if abs(s)>.2];assert peaks and abs(peaks[0]/48000-2)<.025,peaks[:1]
        second=render();save(second,'second.mp4');assert raw(work/'first.mp4')==raw(work/'second.mp4'),'Repeated HyperFrames render changed decoded pixels'
        edit([{'action':'film_split','id':clip['id'],'at':71}]);split=render();save(split,'split.mp4');assert raw(work/'first.mp4')==raw(work/'split.mp4'),'Splitting changed the animated frame sequence'
        source=render('source');save(source,'motion-source.zip');
        with zipfile.ZipFile(work/'motion-source.zip') as z:
            assert z.testzip() is None;assert '--no-browser-gpu' in json.loads(z.read('package.json'))['scripts']['render'];manifest=json.loads(z.read('manifest.json'));assert manifest['engine_version']=='0.8.206' and manifest['frames']==120
            for name,digest in manifest['files'].items():assert hashlib.sha256(z.read(name)).hexdigest()==digest,name
            native=z.read('editable-master.canvas');env=json.loads(gzip.decompress(native[len(b'CANVAS-STUDIO-PACKAGE\n1\n'): ]));assert env['version']==4 and 'motion-keyframes-v1' in env['required_features']
            (work/'motion.canvas').write_bytes(native)
        c.call('studio_media_job',{**c.guard(),'id':first['id'],'operation':'source','path':'from-render.zip','overwrite':True})
        cancel=render();c.call('studio_media_job',{**c.guard(),'id':cancel['id'],'operation':'cancel'});wait(c,cancel,('cancelled',));assert not (qa/folder/f"canvas-studio/media/jobs/{cancel['id']}/movie.mp4").exists()
        before=c.call('studio_document')['timeline'];c.call('studio_open',{**c.guard(),'path':'motion.canvas'});assert c.call('studio_document')['timeline']==before
        if chrome:
            from playwright.sync_api import sync_playwright
            with sync_playwright() as pw:
                browser=pw.chromium.launch(channel=chrome,headless=True);page=browser.new_page(viewport={'width':1440,'height':1000});errors=[];page.on('pageerror',lambda e:errors.append(str(e)));page.goto(url);page.get_by_role('button',name='Film studio',exact=True).click();page.get_by_role('button',name='Clip marker.png',exact=True).first.click();page.get_by_text('Motion',exact=True).wait_for();page.get_by_role('button',name='Rise',exact=True).click();page.wait_for_function("()=>state.timeline.clips[0].motion.keys[0].y>0");page.get_by_text('2 editable keyframes',exact=True).click();page.get_by_role('button',name='Apply keyframes',exact=True).wait_for();page.screenshot(path=str(qa/'motion-ui.png'));assert not errors,errors;browser.close()
        report={'exact_frame':'PASS: x=60 at frame 45','repeat_render_decoded_pixels':'PASS','split_animation_clock':'PASS','audio_sync_error_seconds':abs(peaks[0]/48000-2),'frames':120,'portable_motion_v4':'PASS','source_manifest_hashes':'PASS','cancel_preserves_edit':'PASS','atomic_invalid_keys':'PASS','browser':'PASS' if chrome else 'CI only'}
        (qa/'report.json').write_text(json.dumps(report,indent=2));print(json.dumps(report,indent=2))
    finally:c.close();server.terminate();server.wait(timeout=10);log.close()
if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--binary',default='./canvas-studio');p.add_argument('--chrome');a=p.parse_args();run(Path(a.binary).resolve(),a.chrome)
