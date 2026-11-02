"""End-to-end local browser workflow. Requires Playwright/Pillow and Chrome.
Run cargo test --release first to generate generic QA fixtures, then:
python3 scripts/browser_smoke.py --binary ./canvas-studio
The browser and server use separate test profiles/data; no personal browser is read.
"""
import argparse,base64,gzip,json,os,re,subprocess,time,urllib.request,urllib.error
from pathlib import Path
from PIL import Image
from playwright.sync_api import sync_playwright

ROOT=Path(__file__).resolve().parents[1]
MAGIC=b'CANVAS-STUDIO-PACKAGE\n1\n'
def unpack(path):return json.loads(gzip.decompress(path.read_bytes()[len(MAGIC):]))
def fixture(output):
    collection=unpack(ROOT/'qa/collection-proof/collection.canvas');book=unpack(ROOT/'qa/book-proof/workbook.canvas')
    project=json.loads(collection['project']);source=json.loads(book['project'])
    image=next(e for e in source['pages'][0]['elements'] if 'Image' in e['kind'])
    image['id']=project['next_id'];project['next_id']+=1
    image.update(name='Collection portrait',x=470,y=430,width=180,height=180,binding='portrait',style_id=None)
    project['pages'][0]['elements'].append(image)
    key=image['kind']['Image']['raw_rgba'].split('asset://')[1]
    png=book['assets'][key+'.png'];collection['assets'][key+'.png']=png
    project['production']['fields']['portrait']={'image':True,'value':'Initial photo','required':True,'image_png':png}
    collection['project']=json.dumps(project,ensure_ascii=False,separators=(',',':'))
    output.write_bytes(MAGIC+gzip.compress(json.dumps(collection,ensure_ascii=False).encode()))

def run(binary,chrome):
    qa=ROOT/'qa/browser';qa.mkdir(parents=True,exist_ok=True);fixture(qa/'collection.canvas')
    Image.new('RGB',(600,600),(191,71,98)).save(qa/'portrait.jpg',quality=95)
    env=os.environ.copy();env['XDG_DATA_HOME']=str(qa/'server-data')
    log=(qa/'server.log').open('w');process=subprocess.Popen([str(binary),'--serve-browser','--port','0','--open',str(qa/'collection.canvas')],cwd=ROOT,env=env,stdout=log,stderr=subprocess.STDOUT)
    try:
        url=None
        for _ in range(100):
            text=(qa/'server.log').read_text();match=re.search(r'http://127\.0\.0\.1:\d+/#[0-9a-f]+',text)
            if match:url=match[0];break
            if process.poll() is not None:raise RuntimeError(text)
            time.sleep(.1)
        assert url,'Server did not start';base,token=url.split('#')
        def api(path,body=None,revision=None,origin=None):
            headers={'X-Canvas-Token':token}
            if revision is not None:headers['X-Canvas-Revision']=str(revision)
            if origin:headers['Origin']=origin
            data=None if body is None else json.dumps(body).encode()
            return urllib.request.urlopen(urllib.request.Request(base.rstrip('/')+path,data=data,headers=headers))
        # Separate HTTP clients must not silently overwrite a changed document.
        initial=json.load(api('/api/state'));api('/api/command',{'action':'title','value':'Browser collection proof'},initial['revision']).read()
        try:api('/api/command',{'action':'title','value':'Stale overwrite'},initial['revision']);raise AssertionError('Stale edit accepted')
        except urllib.error.HTTPError as e:assert e.code==409
        for headers in [{},{'X-Canvas-Token':token,'Origin':'https://example.invalid'}]:
            try:urllib.request.urlopen(urllib.request.Request(base+'api/state',headers=headers));raise AssertionError('Unauthorized request accepted')
            except urllib.error.HTTPError as e:assert e.code==403
        errors=[]
        with sync_playwright() as p:
            browser=p.chromium.launch(executable_path=(p.chromium.executable_path if chrome=="chromium" else chrome),headless=True,args=['--no-sandbox'])
            context=browser.new_context(viewport={'width':1440,'height':1000},accept_downloads=True)
            page=context.new_page();page.on('pageerror',lambda error:errors.append(str(error)))
            page.goto(url);page.wait_for_function("() => document.querySelector('#document-title').value==='Browser collection proof'")
            page.get_by_label('guest_name',exact=True).fill('Maya & Alex')
            page.locator('form.field-row').filter(has=page.get_by_label('guest_name',exact=True)).get_by_role('button').click()
            page.wait_for_function("() => state.fields.find(f=>f.key==='guest_name').value==='Maya & Alex'")
            # Linked photo replacement comes from a JPEG and is normalized by the engine.
            with page.expect_file_chooser() as chooser:page.get_by_role('button',name='Choose photo…').click()
            chooser.value.set_files(qa/'portrait.jpg')
            page.wait_for_function("() => state.fields.find(f=>f.key==='portrait').value==='uploaded-photo.png'")
            page.get_by_role('button',name='+ Text',exact=True).click();page.wait_for_selector('#text-content:visible')
            page.locator('#text-content').fill('Unapplied draft')
            page.get_by_role('button',name='Save editable copy',exact=True).click()
            page.wait_for_selector('#message.error:visible')
            assert 'Apply your form edits' in page.locator('#message').inner_text()
            page.get_by_role('button',name='Refresh session',exact=True).click()
            page.wait_for_function("() => !working")
            assert page.locator('#text-content').input_value()=='Unapplied draft'
            page.locator('#text-content').fill('A fresh start — Zoë')
            page.locator('#font').select_option('DisplaySerif');page.locator('#font-size').fill('22')
            page.locator('#leading').fill('1.35');page.locator('#text-form details').locator('summary').click()
            page.locator('#paragraph-inset_left').fill('12');page.locator('#paragraph-space_after').fill('7')
            page.get_by_role('button',name='Apply text & styling').click()
            page.wait_for_function("() => state.elements.find(e=>e.id===selected).content==='A fresh start — Zoë'")
            for name,value in [('x','60'),('y','420'),('width','370'),('height','120')]:page.locator('#geometry-'+name).fill(value)
            page.get_by_role('button',name='Apply geometry',exact=True).click();page.wait_for_function("() => state.elements.find(e=>e.id===selected).y===420")
            page.locator('#text-content').evaluate("e=>{e.focus();e.setSelectionRange(0,13);e.dispatchEvent(new Event('select'));}")
            page.get_by_role('button',name='Underline selection',exact=True).click()
            page.wait_for_function("() => state.elements.find(e=>e.id===selected).paragraph.runs.some(r=>r.underline)")
            with page.expect_file_chooser() as chooser:page.get_by_role('button',name='Import .ttf font…').click()
            chooser.value.set_files(ROOT/'assets/fonts/NotoSansMono-Regular.ttf')
            page.wait_for_function("() => state.fonts.some(f=>f.id.startsWith('imported:'))")
            imported=page.evaluate("state.fonts.find(f=>f.id.startsWith('imported:')).id")
            page.locator('#font').select_option(imported);page.get_by_role('button',name='Apply text & styling').click()
            page.wait_for_function("() => state.elements.find(e=>e.id===selected).family.startsWith('imported:')")
            page.locator('#page-title').fill('Invitation proof')
            page.locator('#page-title').press('Tab');page.wait_for_function("() => state.pages[0].title==='Invitation proof'")
            # Undo/redo operates on the native model, not a screenshot history.
            page.get_by_role('button',name='Undo',exact=True).click();page.wait_for_function("() => state.pages[0].title==='Invitation'")
            page.get_by_role('button',name='Redo',exact=True).click();page.wait_for_function("() => state.pages[0].title==='Invitation proof'")
            page.wait_for_function("() => document.querySelector('#preview').complete && document.querySelector('#preview').naturalWidth>0")
            page.screenshot(path=qa/'browser-desktop.png',full_page=True)
            with page.expect_download() as download:page.get_by_role('button',name='Export PDF',exact=True).click()
            download.value.save_as(qa/'edited.pdf')
            with page.expect_download() as download:page.get_by_role('button',name='Save editable copy',exact=True).click()
            download.value.save_as(qa/'edited.canvas')
            # Open the actual downloaded project through the browser file picker.
            with page.expect_file_chooser() as chooser:page.get_by_role('button',name='Open .canvas',exact=True).click()
            chooser.value.set_files(qa/'edited.canvas')
            page.wait_for_function("() => state.fields.find(f=>f.key==='guest_name').value==='Maya & Alex'")
            # New photos can also be inserted into blank/existing designs.
            count=page.evaluate('state.elements.length')
            with page.expect_file_chooser() as chooser:page.get_by_role('button',name='+ Photo',exact=True).click()
            chooser.value.set_files(qa/'portrait.jpg')
            page.wait_for_function(f"() => state.elements.length==={count+1}")
            page.set_viewport_size({'width':700,'height':900});page.screenshot(path=qa/'browser-narrow.png',full_page=True)
            assert page.evaluate('document.documentElement.scrollWidth<=window.innerWidth'),'Narrow layout overflows'
            assert not errors,errors
            context.close();browser.close()
        text=subprocess.check_output(['pdftotext',str(qa/'edited.pdf'),'-'],text=True)
        assert text.count('Maya & Alex')==5,text
        assert 'A fresh start' in text and 'Zoë' in text,text
        result=json.loads(unpack(qa/'edited.canvas')['project'])
        assert result['imported_fonts']
        added=next(e for e in result['pages'][0]['elements'] if e['name']=='New text')
        assert added['text_format']['inset_left']==12
        photo=Image.open(__import__('io').BytesIO(base64.b64decode(result['production']['fields']['portrait']['image_png'])))
        assert photo.size==(600,600)
        subprocess.run([str(binary),'--verify-project',str(qa/'edited.canvas')],check=True,capture_output=True)
        recovered=list((qa/'server-data/canvas-studio/browser').glob('recovery-*.canvas'));assert recovered
        subprocess.run([str(binary),'--verify-project',str(recovered[-1])],check=True,capture_output=True)
        # Restart on the same port without --open and verify the recovery document.
        process.terminate();process.wait(timeout=10)
        port=base.rstrip('/').rsplit(':',1)[1]
        process=subprocess.Popen([str(binary),'--serve-browser','--port',port],cwd=ROOT,env=env,stdout=log,stderr=subprocess.STDOUT)
        restarted=None
        for _ in range(100):
            matches=re.findall(r'http://127\.0\.0\.1:\d+/#[0-9a-f]+',(qa/'server.log').read_text())
            if matches and matches[-1]!=url:restarted=matches[-1];break
            if process.poll() is not None:raise RuntimeError('Recovery restart failed: '+(qa/'server.log').read_text())
            time.sleep(.1)
        assert restarted,'Recovery server did not restart'
        recovered_base,recovered_token=restarted.split('#')
        recovered_state=json.load(urllib.request.urlopen(urllib.request.Request(recovered_base+'api/state',headers={'X-Canvas-Token':recovered_token})))
        assert next(f['value'] for f in recovered_state['fields'] if f['key']=='guest_name')=='Maya & Alex'
        assert len(recovered_state['elements'])==4
        report={'browser':'Chrome/Chromium, isolated headless profile','recovery_restart':'passed','native_round_trip':'passed','unicode_and_font_import':'passed','linked_text_all_five_pages':'passed','jpeg_photo_replacement':'passed','paragraph_and_character_styles':'passed','undo_redo':'passed','revision_conflict_and_origin_auth':'passed','responsive_700px':'passed','page_errors':errors}
        (qa/'report.json').write_text(json.dumps(report,indent=2));print(json.dumps(report,indent=2))
    finally:process.terminate();process.wait(timeout=10);log.close()
if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('--binary',default='./canvas-studio');parser.add_argument('--chrome',default='/usr/bin/google-chrome');args=parser.parse_args();run(Path(args.binary).resolve(),args.chrome)
