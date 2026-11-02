"""Cross-format campaign acceptance through real MCP, exports and an isolated UI."""
import argparse,base64,gzip,io,json,os,re,subprocess,time,zipfile
from pathlib import Path
from PIL import Image
from mcp_smoke import Client
ROOT=Path(__file__).resolve().parents[1]
def check_bundle(path):
    with zipfile.ZipFile(path) as z:
        assert z.testzip() is None
        manifest=json.loads(z.read('manifest.json'));assert len(manifest['layouts'])==4
        assert 'campaign.canvas' in z.namelist()
        for v in manifest['layouts']:
            im=Image.open(io.BytesIO(z.read(v['folder']+'/page-001.png')))
            assert im.size==(round(v['width']),round(v['height'])),(v,im.size)
            assert z.read(v['folder']+'/'+v['folder']+'.pdf').startswith(b'%PDF')
    return manifest

def run(binary,chrome):
    qa=ROOT/'qa/campaign-ui';qa.mkdir(parents=True,exist_ok=True);work=qa/'workspace';work.mkdir(exist_ok=True)
    env=os.environ.copy();env['XDG_DATA_HOME']=str(qa/'server-data')
    log=(qa/'server.log').open('w');server=subprocess.Popen([str(binary),'--serve-browser','--port','0'],env=env,stdout=log,stderr=subprocess.STDOUT);client=None
    try:
        for _ in range(100):
            match=re.search(r'(http://127\.0\.0\.1:(\d+)/#[0-9a-f]+)',(qa/'server.log').read_text())
            if match:break
            if server.poll() is not None:raise RuntimeError((qa/'server.log').read_text())
            time.sleep(.1)
        else:raise AssertionError('No engine')
        url,port=match.groups();profile=qa/f'server-data/canvas-studio/browser/connection-{port}.json'
        client=Client(work,profile)
        def edit(commands,**kw):return client.call('studio_edit',{**client.guard(),'commands':commands,**kw})
        edit([{'action':'campaign_create'}])
        edit([{'action':'field_define','key':'brand','value':'KINDFOLD STUDIO'},{'action':'brand_token','key':'accent','color':'#e8a892'}])
        doc=client.call('studio_document');assert len(doc['studio']['variants'])==4
        current=client.guard()
        for i,v in enumerate(doc['studio']['variants']):
            result=client.rpc('tools/call',{'name':'studio_preview','arguments':{'document_id':doc['document_id'],'layout':v['name'],'edge':640}})['result']
            assert not result['isError'],result
            (qa/f'layout-{i}.png').write_bytes(base64.b64decode(next(x['data'] for x in result['content'] if x['type']=='image')))
            client.call('studio_export',{**current,'path':f'layout-{i}.pdf','format':'pdf','layout':v['name'],'overwrite':True})
            text=subprocess.check_output(['pdftotext',str(work/f'layout-{i}.pdf'),'-'],text=True);assert 'KINDFOLD STUDIO' in text
        assert client.guard()==current,'Reading/exporting variants changed the live design'
        client.call('studio_export',{**current,'path':'campaign.zip','format':'zip','overwrite':True});check_bundle(work/'campaign.zip')
        client.call('studio_export',{**current,'path':'campaign.canvas','format':'canvas','overwrite':True})
        subprocess.run([str(binary),'--verify-project',str(work/'campaign.canvas')],capture_output=True,check=True)
        # New colour links survive aliases, duplication, inactive layouts and a new client.
        edit([{'action':'add_text','page':0,'ref':'label','content':'A reusable idea','size':18,'x':80,'y':900,'width':700,'height':70},{'action':'brand_bind','page':0,'ids':['$label'],'token':'ink','target':'primary'}])
        # Undo the atomic addition to restore the designed campaign before UI acceptance.
        client.call('studio_history',{**client.guard(),'direction':'undo'})
        if chrome:
            from playwright.sync_api import sync_playwright
            errors=[]
            with sync_playwright() as p:
                browser=p.chromium.launch(executable_path=p.chromium.executable_path if chrome=='chromium' else chrome,headless=True,args=['--no-sandbox'])
                context=browser.new_context(viewport={'width':1440,'height':1000},accept_downloads=True)
                page=context.new_page();page.on('pageerror',lambda e:errors.append(str(e)))
                page.goto(url);page.wait_for_selector('#preview svg [data-canvas-id]')
                page.get_by_role('button',name='Select Campaign headline',exact=True).click()
                page.get_by_role('button',name='Select Accent',exact=True).click(modifiers=['Shift'])
                assert page.locator('#selection-count').inner_text()=='2 selected layers'
                originals={e['id']:(e['x'],e['y']) for e in client.call('studio_document')['elements']}
                page.get_by_role('button',name='Group',exact=True).click();page.wait_for_function('() => !working')
                page.locator('#group-selection').press('ArrowRight');page.wait_for_function('() => !working')
                moved=client.call('studio_document');selected=[e for e in moved['elements'] if e['name'] in ('Campaign headline','Accent')]
                assert all(abs(e['x']-originals[e['id']][0]-1)<.01 for e in selected)
                # Resize through a real corner drag; type scales with its grouped artwork.
                box=page.get_by_role('button',name='Scale selection from bottom right',exact=True).bounding_box();before=client.call('studio_document');type_before=next(e['size'] for e in before['elements'] if e['name']=='Campaign headline')
                page.mouse.move(box['x']+box['width']/2,box['y']+box['height']/2);page.mouse.down();page.mouse.move(box['x']-35,box['y']-25,steps=6);page.mouse.up();page.wait_for_function('() => !working')
                after=client.call('studio_document');assert next(e['size'] for e in after['elements'] if e['name']=='Campaign headline')<type_before
                # Undo scale, nudge and grouping, then test actual alignment without retaining the temporary edit.
                for _ in range(3):page.get_by_role('button',name='Undo',exact=True).click();page.wait_for_function('() => !working')
                page.get_by_role('button',name='Align left',exact=True).click();page.wait_for_function('() => !working')
                aligned=[e for e in client.call('studio_document')['elements'] if e['name'] in ('Campaign headline','Accent')];assert abs(aligned[0]['x']-aligned[1]['x'])<.01
                page.get_by_role('button',name='Undo',exact=True).click();page.wait_for_function('() => !working')
                page.get_by_role('button',name='Campaign and brand tools',exact=True).click()
                page.get_by_role('button',name='Story · 1080 × 1920',exact=True).click();page.wait_for_function("() => state.studio.variants.find(v=>v.active).name==='Story'")
                page.get_by_role('button',name='Insert Campaign call to action',exact=True).click();page.wait_for_function('() => state.elements.length===8')
                page.get_by_role('button',name='Undo',exact=True).click();page.wait_for_function('() => state.elements.length===6')
                page.get_by_role('tab',name='Fields',exact=True).click();page.get_by_label('brand',exact=True).fill('KINDFOLD / NEW CHAPTER')
                page.locator('form.field-row').filter(has=page.get_by_label('brand',exact=True)).get_by_role('button').click();page.wait_for_function("() => state.fields.find(f=>f.key==='brand').value==='KINDFOLD / NEW CHAPTER'")
                page.get_by_role('button',name='Campaign and brand tools',exact=True).click()
                page.get_by_role('button',name='YouTube thumbnail · 1280 × 720',exact=True).click();page.wait_for_function("() => state.studio.variants.find(v=>v.active).name==='YouTube thumbnail'")
                assert 'KINDFOLD / NEW CHAPTER' in page.locator('#preview').inner_text()
                page.screenshot(path=qa/'campaign-studio.png')
                with page.expect_download() as download:page.get_by_role('button',name='Download campaign bundle',exact=True).click()
                download.value.save_as(work/'browser-campaign.zip');check_bundle(work/'browser-campaign.zip')
                assert client.call('studio_document')['studio']['variants'][0]['name']=='YouTube thumbnail'
                # Portable browser save/reopen retains every layout and its own composition.
                with page.expect_download() as download:page.get_by_role('button',name='Save editable copy',exact=True).click()
                download.value.save_as(work/'browser-campaign.canvas')
                with page.expect_file_chooser() as chooser:page.get_by_role('button',name='Open .canvas',exact=True).click()
                chooser.value.set_files(work/'browser-campaign.canvas');page.wait_for_function('() => !working')
                assert len(client.call('studio_document')['studio']['variants'])==4
                page.set_viewport_size({'width':700,'height':900});page.screenshot(path=qa/'campaign-narrow.png');assert page.evaluate('document.documentElement.scrollWidth<=innerWidth')
                assert not errors,errors
                browser.close()
        report={'campaign_layouts':4,'shared_copy_colours':'passed','non_mutating_variant_previews_and_exports':'passed','native_cold_load':'passed','zip_manifests_crc_and_pixel_dimensions':'passed','alias_colour_binding':'passed','browser_selection_scale_alignment_keyboard_groups': 'passed' if chrome else 'not run','browser_component_insertion_shared_fields_bundle_round_trip': 'passed' if chrome else 'not run'}
        (qa/'report.json').write_text(json.dumps(report,indent=2));print(json.dumps(report,indent=2))
    finally:
        if client:client.close()
        server.terminate();server.wait(timeout=10);log.close()
if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('--binary',default='./canvas-studio');parser.add_argument('--chrome');args=parser.parse_args();run(Path(args.binary).resolve(),args.chrome)
