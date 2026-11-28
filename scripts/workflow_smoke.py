"""Task-led home, non-destructive navigation and editable delivery in an isolated browser."""
import argparse,json,os,re,subprocess,time
from pathlib import Path
from playwright.sync_api import sync_playwright

ROOT=Path(__file__).resolve().parents[1]

def run(binary,chrome):
    qa=ROOT/'qa/workflows';qa.mkdir(parents=True,exist_ok=True)
    env={**os.environ,'XDG_DATA_HOME':str(qa/('data-'+str(time.time_ns()))),'CANVAS_STUDIO_PROFILE':'publishing'}
    log=(qa/'server.log').open('w');process=subprocess.Popen([str(binary),'--serve-browser','--port','0'],cwd=ROOT,env=env,stdout=log,stderr=subprocess.STDOUT)
    try:
        for _ in range(100):
            match=re.search(r'http://127\.0\.0\.1:\d+/#[a-f0-9]+',(qa/'server.log').read_text())
            if match:break
            assert process.poll() is None,(qa/'server.log').read_text();time.sleep(.1)
        assert match,'No studio URL';errors=[]
        with sync_playwright() as p:
            browser=p.chromium.launch(executable_path=p.chromium.executable_path if chrome=='chromium' else chrome,headless=True,args=['--no-sandbox'])
            context=browser.new_context(viewport={'width':1440,'height':1000},accept_downloads=True)
            page=context.new_page();page.on('pageerror',lambda e:errors.append(str(e)))
            page.goto(match[0]);page.locator('#home-tasks .home-task').last.wait_for()
            assert page.locator('#studio-home').is_visible()
            assert page.locator('#home-tasks .home-task').count()==4
            assert not page.get_by_role('button',name='Image and video generation',exact=True).is_visible()
            page.screenshot(path=qa/'home-desktop.png',full_page=True)
            page.set_viewport_size({'width':700,'height':900})
            assert page.evaluate('document.documentElement.scrollWidth<=innerWidth')
            page.screenshot(path=qa/'home-narrow.png',full_page=True)
            page.set_viewport_size({'width':1440,'height':1000})
            page.get_by_role('button',name='Design a product',exact=True).click()
            assert not page.locator('#studio-home').is_visible()
            page.get_by_role('button',name='Preview The Editorial',exact=True).click()
            page.get_by_role('button',name='Create design',exact=True).click()
            page.wait_for_function('() => state.elements.length>0')
            doc=page.evaluate('state.document_id')
            page.get_by_role('button',name='+ Text',exact=True).click()
            page.locator('#text-content').fill('A draft that must survive Home')
            page.get_by_role('button',name='Studio home',exact=True).click()
            assert 'Unapplied edits' in page.locator('#home-project-status').inner_text()
            for task,selector in [('Make a video','#film-studio'),('Create a campaign','#tab-campaign'),('Bring in research','#research-studio')]:
                page.get_by_role('button',name=task,exact=True).click()
                page.locator(selector).wait_for(state='visible')
                assert page.evaluate('state.document_id')==doc
                page.get_by_role('button',name='Studio home',exact=True).click()
                page.get_by_role('button',name='Continue current project',exact=True).click()
                assert page.locator(selector).is_visible(),task
                page.get_by_role('button',name='Studio home',exact=True).click()
            page.get_by_role('button',name='Continue current project',exact=True).click()
            page.get_by_role('button',name='Text tools',exact=True).click()
            assert page.locator('#text-content').input_value()=='A draft that must survive Home'
            page.get_by_role('button',name='Save editable copy',exact=True).click()
            page.locator('#message.error').wait_for()
            page.get_by_role('button',name='Apply text & styling',exact=True).click()
            page.wait_for_function("() => state.elements.some(e=>e.content==='A draft that must survive Home')")
            with page.expect_download() as download:page.get_by_role('button',name='Save editable copy',exact=True).click()
            download.value.save_as(qa/'home-roundtrip.canvas')
            subprocess.run([str(binary),'--verify-project',str(qa/'home-roundtrip.canvas')],check=True,capture_output=True)
            # Opening saved work from Home must continue into the editor.
            page.get_by_role('button',name='Studio home',exact=True).click()
            with page.expect_file_chooser() as chooser:page.get_by_role('button',name='Open design…',exact=True).click()
            page.on('dialog',lambda d:d.accept())
            chooser.value.set_files(qa/'home-roundtrip.canvas')
            page.locator('#studio-home').wait_for(state='hidden')
            assert '1.20.' in page.title() or '1.21.' in page.title()
            assert not errors,errors
            context.close();browser.close()
        report={'shared_four_tasks':'passed','first_use_and_existing_document':'passed','draft_and_document_preservation':'passed','editable_native_delivery':'passed','open_from_home':'passed','700px_no_overflow':'passed','page_errors':errors}
        (qa/'report.json').write_text(json.dumps(report,indent=2));print(json.dumps(report,indent=2))
    finally:process.terminate();process.wait(timeout=10);log.close()

if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--binary',default='./canvas-studio');p.add_argument('--chrome',default='/usr/bin/google-chrome');a=p.parse_args();run(Path(a.binary).resolve(),a.chrome)
