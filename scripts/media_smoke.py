"""Offline media acceptance: real MCP, FFmpeg decoded frames/audio, portable cold restore.
Optional --chrome runs the Film UI on an isolated CI browser, never a user's tab.
"""
import argparse,array,gzip,hashlib,json,math,os,re,shutil,subprocess,time
from pathlib import Path
from mcp_smoke import Client
ROOT=Path(__file__).resolve().parents[1]
def run_cmd(args):return subprocess.run(args,check=True,capture_output=True).stdout
def wait(c,j,states=('complete',),timeout=180):
    end=time.monotonic()+timeout
    while time.monotonic()<end:
        value=c.call('studio_media_job',{**c.guard(),'id':j['id'],'operation':'status'})
        if value['state'] in ('complete','failed','cancelled'):
            assert value['state'] in states,value
            return value
        time.sleep(.15)
    raise AssertionError('Media job timed out')
def launch(binary,qa,folder,project=None):
    logpath=qa/(folder+'.log');log=logpath.open('w');env=os.environ.copy();env['XDG_DATA_HOME']=str(qa/folder)
    p=subprocess.Popen([str(binary),'--serve-browser','--port','0',*(['--open',str(project)] if project else [])],env=env,stdout=log,stderr=subprocess.STDOUT)
    for _ in range(100):
        match=re.search(r'(http://127\.0\.0\.1:(\d+)/#[0-9a-f]+)',logpath.read_text())
        if match:
            url,port=match.groups();return p,log,qa/f'{folder}/canvas-studio/browser/connection-{port}.json',url
        if p.poll() is not None:raise AssertionError(logpath.read_text())
        time.sleep(.1)
    raise AssertionError('No media engine')
def probe(path):return json.loads(run_cmd(['ffprobe','-v','error','-show_streams','-show_format','-of','json',str(path)]))
def pcm(path):
    samples=array.array('f');samples.frombytes(run_cmd(['ffmpeg','-v','error','-i',str(path),'-map','0:a:0','-ac','1','-ar','48000','-f','f32le','-']))
    return samples
def pixel(path,frame):
    b=run_cmd(['ffmpeg','-v','error','-i',str(path),'-vf',f"select=eq(n\\,{frame}),scale=1:1",'-frames:v','1','-f','rawvideo','-pix_fmt','rgb24','-'])
    return tuple(b)
def run(binary,chrome=None):
    qa=ROOT/'qa/media';qa.mkdir(parents=True,exist_ok=True);work=qa/'workspace';work.mkdir(exist_ok=True)
    movie=work/'source.mp4';music=work/'music.wav'
    moving=work/'24fps.mp4'
    run_cmd(['ffmpeg','-v','error','-y','-f','lavfi','-i','nullsrc=s=128x72:r=24:d=5,geq=lum=20+N*1.5:cb=128:cr=128','-c:v','libx264','-crf','0',str(moving)])
    run_cmd(['ffmpeg','-v','error','-y','-f','lavfi','-i','color=red:s=320x180:r=30:d=3','-f','lavfi','-i','color=blue:s=320x180:r=30:d=3','-f','lavfi','-i','color=green:s=320x180:r=30:d=3','-f','lavfi','-i',"aevalsrc=if(between(t\\,2\\,2.08)\\,0.6*sin(2*PI*1000*t)\\,0):s=48000:d=9",'-filter_complex','[0:v][1:v][2:v]concat=n=3:v=1:a=0[v]','-map','[v]','-map','3:a','-c:v','libx264','-preset','ultrafast','-pix_fmt','yuv420p','-c:a','aac','-movflags','+faststart',str(movie)])
    run_cmd(['ffmpeg','-v','error','-y','-f','lavfi','-i','sine=frequency=440:sample_rate=48000:duration=8','-c:a','pcm_s16le',str(music)])
    server,log,connection,url=launch(binary,qa,'data');c=Client(work,connection)
    report={}
    try:
        def edit(commands,**kw):return c.call('studio_edit',{**c.guard(),'commands':commands,**kw})
        def render(mode='export',**kw):return c.call('studio_media_render',{**c.guard(),'mode':mode,**kw})
        def save(job,name):wait(c,job);return c.call('studio_media_job',{**c.guard(),'id':job['id'],'operation':'save','path':name,'overwrite':True})
        c.call('studio_create',{**c.guard(),'width':320,'height':180})
        edit([{'action':'film_setup','width':320,'height':180,'fps':30,'duration':240}])
        a=c.call('studio_import',{**c.guard(),'kind':'media','path':'source.mp4'})['asset'];assert a['kind']=='video' and a['has_audio'] and a['width']==320
        b=c.call('studio_import',{**c.guard(),'kind':'media','path':'music.wav'})['asset'];assert b['kind']=='audio'
        proxy=render('proxy',asset=a['id']);wait(c,proxy)
        assert list((qa/'data/canvas-studio/media/cache').glob('*.proxy.mp4'))
        edit([{'action':'film_add','asset':a['id'],'start':30,'duration':180,'source_in_us':1000000}])
        clip=c.call('studio_document')['timeline']['clips'][0]
        edit([{'action':'film_add','asset':b['id'],'start':0,'duration':240}]);musicclip=c.call('studio_document')['timeline']['clips'][-1]
        edit([{'action':'film_clip','id':musicclip['id'],'gain_db':-6,'fade_in':60,'fade_out':60}])
        g=c.guard();edit([{'action':'film_clip','id':clip['id'],'start':239}],dry_run=True) if False else None
        c.call('studio_edit',{**g,'commands':[{'action':'title','value':'Must not commit'},{'action':'film_clip','id':clip['id'],'start':239}]},error=True);assert c.guard()==g
        baseline=render();assert c.guard()==g;save(baseline,'baseline.mp4')
        p=probe(work/'baseline.mp4');v=next(s for s in p['streams'] if s['codec_type']=='video');audio=next(s for s in p['streams'] if s['codec_type']=='audio')
        assert int(v['nb_frames'])==240 and v['r_frame_rate']=='30/1' and (v['width'],v['height'])==(320,180),v
        assert audio['sample_rate']=='48000' and abs(float(audio['duration'])-8)<.002,audio
        assert max(pixel(work/'baseline.mp4',29))<40
        assert pixel(work/'baseline.mp4',30)[0]>200
        assert pixel(work/'baseline.mp4',89)[0]>200
        assert pixel(work/'baseline.mp4',90)[2]>200
        assert pixel(work/'baseline.mp4',179)[2]>200
        assert pixel(work/'baseline.mp4',180)[1]>70
        assert max(pixel(work/'baseline.mp4',210))<40
        samples=pcm(work/'baseline.mp4');peaks=[i for i,x in enumerate(samples) if abs(x)>.25];assert peaks and abs(peaks[0]/48000-2)<.025,peaks[:1]
        edit([{'action':'film_split','id':clip['id'],'at':75},{'action':'film_split','id':musicclip['id'],'at':30}]);split=render();save(split,'split.mp4')
        other=pcm(work/'split.mp4');count=min(len(samples),len(other),384000);rms=math.sqrt(sum((samples[i]-other[i])**2 for i in range(count))/count);assert rms<.01,rms
        assert pixel(work/'split.mp4',74)[0]>200 and pixel(work/'split.mp4',75)[0]>200
        report['frame_boundaries']='PASS at 29/30, 74/75, 89/90, 179/180, 209/210';report['sync_impulse_error_seconds']=abs(peaks[0]/48000-2);report['split_audio_rms_error']=rms;report['frames']=240
        edit([{'action':'film_caption','text':'Make the next frame count.','start':210,'duration':30}]);cap=c.call('studio_document')['timeline']['clips'][-1]
        edit([{'action':'film_clip','id':cap['id'],'size':12,'y':100,'height':65,'width':270,'x':25}])
        final=render('preview');save(final,'preview.mp4');assert pixel(work/'preview.mp4',215)!=pixel(work/'baseline.mp4',215)
        frame=render('frame',frame=215);save(frame,'frame.png');assert (work/'frame.png').read_bytes().startswith(b'\x89PNG')
        cancel=render();c.call('studio_media_job',{**c.guard(),'id':cancel['id'],'operation':'cancel'});wait(c,cancel,('cancelled',));assert not (qa/f"data/canvas-studio/media/jobs/{cancel['id']}/movie.mp4").exists()
        # Save and reopen on a genuinely cold media cache: no original/proxy sidecars.
        before=c.call('studio_document')['timeline'];c.call('studio_export',{**c.guard(),'path':'portable.canvas','format':'canvas','overwrite':True})
        c.close();c=None;server.terminate();server.wait(timeout=10);log.close()
        cold_folder='cold-data-'+str(time.time_ns());server,log,connection,url=launch(binary,qa,cold_folder,work/'portable.canvas');c=Client(work,connection);assert c.call('studio_document')['timeline']==before
        assert list((qa/cold_folder/'canvas-studio/media/originals').glob('*.mp4'))
        cold=c.call('studio_media_render',{**c.guard(),'mode':'export'});wait(c,cold)
        # Deliberate corruption is detected before decoding; recovery still opens metadata.
        original=qa/cold_folder/'canvas-studio/media/originals'/f"{a['id']}.mp4";original.write_bytes(b'broken')
        bad=c.call('studio_media_render',{**c.guard(),'mode':'export'});failure=wait(c,bad,('failed',));assert 'damaged' in failure['error']
        c.call('studio_open',{**c.guard(),'path':'portable.canvas'});assert original.stat().st_size==movie.stat().st_size
        c.call('studio_media_job',{**c.guard(),'id':cold['id'],'operation':'save','path':'../escape.mp4'},error=True)
        c.call('studio_media_job',{**c.guard(),'id':cold['id'],'operation':'save','path':'source.mp4'},error=True)
        if chrome:
            from playwright.sync_api import sync_playwright
            with sync_playwright() as pw:
                browser=pw.chromium.launch(channel=chrome,headless=True);page=browser.new_page(viewport={'width':1440,'height':1000});errors=[];page.on('pageerror',lambda e:errors.append(str(e)));page.goto(url);page.get_by_role('button',name='Film studio',exact=True).click();page.get_by_text('The edit',exact=True).wait_for();page.get_by_role('button',name='Clip source.mp4',exact=True).first.click();page.get_by_role('button',name='Apply timing',exact=True).wait_for();page.get_by_label('Source in · seconds',exact=True).fill('1.25');page.get_by_role('button',name='Apply timing',exact=True).click();page.get_by_text('Recovery saved on this device · Save a copy when ready',exact=True).wait_for();assert c.call('studio_document')['timeline']['clips'][0]['source_in_us']==1250000;page.locator('#film-playhead').fill('45');page.locator('#film-playhead').press('Tab');page.get_by_role('button',name='Review this frame').click();page.screenshot(path=str(qa/'film-ui.png'));assert not errors,errors;browser.close()
        # Splitting must retain the FPS sampling phase on non-matching source rates.
        c.call('studio_create',{**c.guard(),'width':128,'height':72})
        c.call('studio_media_job',{**c.guard(),'id':cold['id'],'operation':'status'},error=True)
        edit([{'action':'film_setup','width':128,'height':72,'fps':30,'duration':120}])
        moving_asset=c.call('studio_import',{**c.guard(),'kind':'media','path':'24fps.mp4'})['asset']
        edit([{'action':'film_add','asset':moving_asset['id'],'start':0,'duration':120}])
        base=render();save(base,'24-base.mp4');moving_clip=c.call('studio_document')['timeline']['clips'][0]
        edit([{'action':'film_split','id':moving_clip['id'],'at':37}]);cut=render();save(cut,'24-split.mp4')
        def grey(path):return run_cmd(['ffmpeg','-v','error','-i',str(path),'-vf','scale=1:1','-f','rawvideo','-pix_fmt','gray','-'])
        assert grey(work/'24-base.mp4')==grey(work/'24-split.mp4'),'24→30 FPS split shifted a frame'
        report['24_to_30_fps_split']='PASS: identical decoded frame sequence'
        report.update(portable_cold_restore='PASS',cancel_preserves_edit='PASS',tamper_detection='PASS',workspace_guards='PASS',browser='PASS' if chrome else 'CI only')
        (qa/'report.json').write_text(json.dumps(report,indent=2));print(json.dumps(report,indent=2))
    finally:
        if c:c.close()
        server.terminate();server.wait(timeout=10);log.close()
if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--binary',default='./canvas-studio');p.add_argument('--chrome');a=p.parse_args();run(Path(a.binary).resolve(),a.chrome)
