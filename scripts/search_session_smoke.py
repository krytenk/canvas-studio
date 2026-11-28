"""Regression for opening a bare local URL, reconnecting without losing keys,
credential persistence and live-search loading/results/empty/error UI.
Uses an isolated app/profile and deterministic provider-job fixtures, never real keys.
"""
import argparse,json,os,subprocess,urllib.request,urllib.error
from pathlib import Path
from playwright.sync_api import sync_playwright
from media_smoke import launch
ROOT=Path(__file__).resolve().parents[1]
def run(binary,chrome):
 qa=ROOT/'qa/search-session';qa.mkdir(parents=True,exist_ok=True)
 import time
 folder='data-'+str(time.time_ns());server,log,connection,url=launch(binary,qa,folder)
 credentials=json.loads(connection.read_text());base=url.split('/#')[0];jobs=[];errors=[]
 try:
  # The repair must not relax loopback authentication.
  try:urllib.request.urlopen(base+'/api/ai/state');raise AssertionError('Anonymous API request was allowed')
  except urllib.error.HTTPError as e:assert e.code==403
  with sync_playwright() as pw:
   browser=pw.chromium.launch(headless=True,**({} if chrome=='chromium' else {'executable_path':chrome}))
   context=browser.new_context(viewport={'width':1440,'height':1100});page=context.new_page();page.on('pageerror',lambda e:errors.append(str(e)))
   page.goto(base);page.wait_for_selector('#session-access[open]');assert page.locator('#document-title').input_value()=='Connecting…'
   page.screenshot(path=qa/'private-link-required.png',full_page=True)
   page.get_by_label('Private studio session link',exact=True).fill('https://example.invalid/#'+'a'*64)
   page.get_by_role('button',name='Connect this browser',exact=True).click()
   page.wait_for_function("() => document.querySelector('#session-access-error').textContent.includes('this running Canvas Studio address')")
   page.get_by_label('Private studio session link',exact=True).fill(url);page.get_by_role('button',name='Connect this browser',exact=True).click()
   page.wait_for_selector('#session-access:not([open])',state='attached');page.wait_for_function("() => document.querySelector('#document-title').value==='Untitled Design'")
   page.get_by_role('button',name='Image and video generation',exact=True).click();page.wait_for_selector('#ai-model-select option',state='attached')
   if not page.locator('#ai-connections').evaluate('(e)=>e.open'):page.locator('#ai-connections summary').click()
   fake={'kaggle_username':'isolated-test','kaggle_token':'fake-kaggle-key','hf_token':'fake-hf-key','civitai_token':'fake-civitai-key'}
   for key,value in fake.items():page.locator(f'#ai-connections-form [name={key}]').fill(value)
   # Expire the in-memory session, then follow the complete link in the same tab.
   # Hash reconnection must not reload or erase a credential form already in progress.
   page.evaluate("token='0'.repeat(64)")
   page.get_by_role('button',name='Refresh session',exact=True).click();page.wait_for_selector('#session-access[open]')
   page.goto(url);page.wait_for_selector('#session-access:not([open])',state='attached')
   for key,value in fake.items():assert page.locator(f'#ai-connections-form [name={key}]').input_value()==value
   page.get_by_role('button',name='Save connections privately',exact=True).click()
   page.wait_for_function("() => document.querySelector('#message').textContent.includes('Connections saved privately')")
   for key in ('hf_token','civitai_token','kaggle_token'):assert page.locator(f'#ai-connections-form [name={key}]').input_value()==''
   private=qa/folder/'canvas-studio/ai/connections.json';assert json.loads(private.read_text())['hf_token']==fake['hf_token']
   if os.name=='posix':assert private.stat().st_mode&0o777==0o600
   req=urllib.request.Request(base+'/api/ai/state',headers={'X-Canvas-Token':credentials['token']})
   public=json.load(urllib.request.urlopen(req));assert not any(value in json.dumps(public) for key,value in fake.items() if key.endswith('token'))
   def action(route):
    value=route.request.post_data_json
    if value.get('operation')!='search':route.continue_();return
    id='fixture-'+str(len(jobs));query=value['query'];result={'provider':value['provider'],'query':query,'models':[],'metadata_only':True}
    error='Civitai denied this request (HTTP 403). Check the saved provider token and any required model access or website verification.' if query=='error' else None
    if query not in ('empty','error'):result['models']=[{'id':'owner/realistic-test','provider':'huggingface','name':'Realistic test model','license':'apache-2.0','url':'https://huggingface.co/owner/realistic-test'}]
    jobs.insert(0,{'id':id,'state':'failed' if error else 'complete','mode':'ai_task','source_revision':0,'result':result if not error else {},'error':error})
    route.fulfill(status=200,content_type='application/json',body=json.dumps({'id':id}))
   page.route('**/api/ai/action',action)
   page.route('**/api/film/jobs',lambda route:route.fulfill(status=200,content_type='application/json',body=json.dumps({'jobs':jobs})))
   page.locator('#ai-model-browser > summary').click()
   page.locator('#ai-search').locator('xpath=..').locator('summary').click()
   for query,expected in [('realistic','1 results'),('empty','No models matched'),('error','denied this request')]:
    page.locator('#ai-search [name=query]').fill(query);page.get_by_role('button',name='Search live catalog',exact=True).click()
    page.wait_for_function('(text) => document.querySelector("#ai-search-status").textContent.includes(text)',arg=expected)
    assert page.get_by_role('button',name='Search live catalog',exact=True).is_enabled()
    if query=='realistic':assert page.locator('#ai-search-results .ai-found').count()==1
    if query=='empty':assert page.locator('#ai-search-results .ai-found').count()==0
   page.locator('#ai-search [name=query]').fill('realistic');page.get_by_role('button',name='Search live catalog',exact=True).click();page.wait_for_function("() => document.querySelector('#ai-search-status').textContent.includes('1 results')")
   page.screenshot(path=qa/'search-results.png',full_page=True)
   assert not errors,errors;context.close();browser.close()
  report={'bare_url_gate':'passed','anonymous_api_still_rejected':'passed','wrong_origin_private_link_rejected':'passed','hash_reconnect_preserves_entered_keys':'passed','private_save_and_no_key_echo':'passed','search_results_empty_errors_and_retry':'passed','page_errors':errors,'provider_responses':'Deterministic fixtures; real authenticated HF/Civitai searches are checked separately on the user account.'}
  (qa/'report.json').write_text(json.dumps(report,indent=2));print(json.dumps(report,indent=2))
 finally:server.terminate();server.wait(timeout=10);log.close()
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('--binary',default='./canvas-studio');p.add_argument('--chrome',default='chromium');a=p.parse_args();run(Path(a.binary).resolve(),a.chrome)
