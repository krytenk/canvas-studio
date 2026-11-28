"""Real local Research imports, cited notes, captions, media and portable recovery.
Only generated QA fixtures are used; no personal Reference Desk files are read.
"""
import argparse,gzip,hashlib,json,os,re,subprocess,time,urllib.request,urllib.error
from pathlib import Path
from PIL import Image
from playwright.sync_api import expect,sync_playwright
ROOT=Path(__file__).resolve().parents[1]
TXT='A useful tutorial\nhttps://example.com/owned-video\nOn-device speech recognition (en)\n\n[0:01] First useful passage.\n[1:02] A second passage with café and <script> as plain text.\n'
SRT='1\n00:00:01,000 --> 00:00:02,500\nFirst caption\n\n2\n00:01:01,000 --> 00:01:02,000\nSecond caption\n'

def run(binary,chrome):
    qa=ROOT/'qa/research';qa.mkdir(parents=True,exist_ok=True)
    folder=qa/('run-'+str(time.time_ns()));folder.mkdir();library=folder/'desk'
    for name in ['Transcripts','Audio','Video']:(library/name).mkdir(parents=True)
    (library/'Transcripts/owned-lesson.txt').write_text(TXT);(library/'Transcripts/owned-lesson.srt').write_text(SRT)
    (folder/'imported-note.txt').write_text('A personal note imported from another folder.')
    subprocess.run(['ffmpeg','-hide_banner','-loglevel','error','-f','lavfi','-i','sine=frequency=440:duration=0.3',str(library/'Audio/owned-tone.wav')],check=True)
    subprocess.run(['ffmpeg','-hide_banner','-loglevel','error','-f','lavfi','-i','testsrc2=size=128x96:rate=30','-t','0.4','-c:v','libx264','-pix_fmt','yuv420p',str(library/'Video/owned-shot.mp4')],check=True)
    originals={str(p):hashlib.sha256(p.read_bytes()).hexdigest() for p in library.rglob('*') if p.is_file()}
    with (library/'Video/oversized-working-copy.mp4').open('wb') as sparse:sparse.truncate(128*1024*1024+1)
    env={**os.environ,'XDG_DATA_HOME':str(folder/'data'),'CANVAS_STUDIO_REFERENCE_ROOT':str(library),'CANVAS_STUDIO_PROFILE':'publishing'}
    log=(folder/'server.log').open('w');process=subprocess.Popen([str(binary),'--serve-browser','--port','0'],cwd=ROOT,env=env,stdout=log,stderr=subprocess.STDOUT)
    try:
        match=None
        for _ in range(100):
            match=re.search(r'http://127\.0\.0\.1:\d+/#[a-f0-9]+',(folder/'server.log').read_text())
            if match:break
            assert process.poll() is None,(folder/'server.log').read_text();time.sleep(.1)
        assert match,'No studio URL';url=match[0];base,token=url.split('#')
        def api(path,value=None):
            headers={'X-Canvas-Token':token}
            if value is not None:
                current=json.load(api('api/state'));headers.update({'Content-Type':'application/json','X-Canvas-Revision':str(current['revision']),'X-Canvas-Document':current['document_id']})
            return urllib.request.urlopen(urllib.request.Request(base+path,data=None if value is None else json.dumps(value).encode(),headers=headers))
        try:api('api/ai/action',{'operation':'create','prompt':'must not run'});raise AssertionError('Generation was enabled')
        except urllib.error.HTTPError as e:assert e.code==403
        errors=[];external=[]
        with sync_playwright() as pw:
            browser=pw.chromium.launch(executable_path=pw.chromium.executable_path if chrome=='chromium' else chrome,headless=True,args=['--no-sandbox'])
            context=browser.new_context(viewport={'width':1440,'height':1000},accept_downloads=True)
            page=context.new_page();page.on('pageerror',lambda e:errors.append(str(e)))
            page.on('request',lambda r:external.append(r.url) if not r.url.startswith(('http://127.0.0.1:','data:','blob:')) else None)
            page.on('dialog',lambda d:d.accept());page.goto(url)
            page.get_by_role('button',name='Bring in research',exact=True).click()
            page.locator('#research-files .research-file').last.wait_for()
            assert not page.get_by_role('button',name='Image and video generation',exact=True).is_visible()
            large=page.locator('#research-files .research-file').filter(has=page.get_by_text('oversized-working-copy.mp4',exact=True));assert large.get_by_role('button',name='Import to video library',exact=True).is_disabled();assert 'working copy' in large.inner_text()
            def preview(name):
                page.locator('#research-files .research-file').filter(has=page.get_by_text(name,exact=True)).get_by_role('button',name='Preview transcript',exact=True).click()
                page.get_by_role('button',name='Copy source into project',exact=True).wait_for()
            preview('owned-lesson.txt');page.get_by_role('button',name='Copy source into project',exact=True).click()
            page.get_by_role('button',name='Add passage to page notes',exact=True).wait_for()
            page.get_by_role('button',name='01:02  A second passage with café and <script> as plain text.',exact=True).click()
            page.get_by_role('button',name='Add passage to page notes',exact=True).click()
            page.wait_for_function("() => state.notes.includes('00:01:02,000')")
            expect(page.locator('#research-notes')).to_contain_text('https://example.com/owned-video')
            page.get_by_role('button',name='Undo',exact=True).click();page.wait_for_function("() => state.notes===''")
            page.get_by_role('button',name='Redo',exact=True).click();page.wait_for_function("() => state.notes.includes('second passage')")
            page.screenshot(path=qa/'research-desktop.png',full_page=True)
            # A changed on-disk preview must fail and leave the project intact.
            preview('owned-lesson.txt');source_file=library/'Transcripts/owned-lesson.txt';source_file.write_text('A changed local source')
            page.get_by_role('button',name='Copy source into project',exact=True).click()
            page.locator('#message.error').filter(has_text='changed').wait_for();assert len(json.load(api('api/state'))['research'])==1;source_file.write_text(TXT)
            preview('owned-lesson.srt');page.get_by_role('button',name='Copy source into project',exact=True).click()
            page.get_by_role('button',name='Add captions to video',exact=True).click();page.locator('#film-studio').wait_for(state='visible')
            state=json.load(api('api/state'));assert len(state['timeline']['clips'])==2 and state['timeline']['duration']==1860
            page.get_by_role('button',name='Personal research',exact=True).click()
            page.get_by_role('button',name='Add captions to video',exact=True).click();page.locator('#message.error').wait_for()
            assert len(json.load(api('api/state'))['timeline']['clips'])==2
            for name in ['owned-tone.wav','owned-shot.mp4']:
                page.locator('#research-files .research-file').filter(has=page.get_by_text(name,exact=True)).get_by_role('button',name='Import to video library',exact=True).click()
                page.locator('#film-studio').wait_for(state='visible')
                page.locator('.film-asset').filter(has=page.get_by_text(name,exact=True)).get_by_role('button',name='+ At playhead',exact=True).click()
                page.wait_for_function('(name) => state.timeline.clips.some(c=>c.name===name)',arg=name)
                page.get_by_role('button',name='Personal research',exact=True).click()
            with page.expect_file_chooser() as chooser:page.get_by_role('button',name='Import transcript / SRT…',exact=True).click()
            chooser.value.set_files(folder/'imported-note.txt');page.wait_for_function('() => state.research.length===5')
            page.set_viewport_size({'width':700,'height':900});assert page.evaluate('document.documentElement.scrollWidth<=innerWidth');page.screenshot(path=qa/'research-narrow.png',full_page=True);page.set_viewport_size({'width':1440,'height':1000})
            # Wait for local proxy jobs, then render a real caption frame.
            for _ in range(600):
                jobs=json.load(api('api/film/jobs'))['jobs']
                if not any(j['state']=='running' for j in jobs):break
                time.sleep(.1)
            page.get_by_role('button',name='Film studio',exact=True).click()
            page.locator('#film-playhead').fill('45');page.locator('#film-playhead').press('Tab');page.get_by_role('button',name='Review this frame',exact=True).click()
            job=None
            for _ in range(600):
                job=next((j for j in json.load(api('api/film/jobs'))['jobs'] if j.get('mode')=='frame'),None)
                if job and job['state']!='running':break
                time.sleep(.1)
            assert job and job['state']=='complete',job
            frame=api('api/film/output?id='+job['id']).read();(qa/'caption-frame.png').write_bytes(frame)
            image=Image.open(qa/'caption-frame.png').convert('RGB');assert max(b-a for a,b in image.getextrema())>100
            with page.expect_download() as saved:page.get_by_role('button',name='Save editable copy',exact=True).click()
            saved.value.save_as(qa/'research.canvas')
            fresh={**env,'XDG_DATA_HOME':str(folder/'fresh-data')};subprocess.run([str(binary),'--verify-project',str(qa/'research.canvas')],env=fresh,cwd=folder,check=True,capture_output=True)
            bundle=json.loads(gzip.decompress((qa/'research.canvas').read_bytes().split(b'\n',2)[2]));assert bundle['version']==6 and len(bundle['media'])==2
            project=json.loads(bundle['project']);assert len(project['production']['research'])==5
            source_id=project['production']['research'][0]['id']
            page.get_by_role('button',name='Studio home',exact=True).click();page.get_by_role('button',name='Design a product',exact=True).click();page.get_by_role('button',name='Preview The Editorial',exact=True).click();page.get_by_role('button',name='Create design',exact=True).click();page.wait_for_function('() => state.research.length===0')
            try:api('api/research/source?id='+source_id);raise AssertionError('Source crossed document boundary')
            except urllib.error.HTTPError as e:assert e.code==422
            assert not errors,errors;assert not external,external
            context.close();browser.close()
        for name,digest in originals.items():assert hashlib.sha256(Path(name).read_bytes()).hexdigest()==digest,name
        report={'oversized_media_guidance':'passed','generation_disabled':'passed','no_external_requests':'passed','source_snapshot_and_changed_preview_guard':'passed','source_citations_and_undo_redo':'passed','captions_and_atomic_overlap_rejection':'passed','audio_and_video_import':'passed','real_caption_frame':'passed','portable_v6_cold_native_load':'passed','document_isolation':'passed','original_files_unchanged':'passed','700px_no_overflow':'passed','page_errors':errors}
        (qa/'report.json').write_text(json.dumps(report,indent=2));print(json.dumps(report,indent=2))
    finally:process.terminate();process.wait(timeout=10);log.close()
if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--binary',default='./canvas-studio');p.add_argument('--chrome',default='/usr/bin/google-chrome');a=p.parse_args();run(Path(a.binary).resolve(),a.chrome)
