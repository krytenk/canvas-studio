"""Exercise real studio/MCP jobs, portable notebooks and isolated browser controls.
No GPU submission or production weight downloads. --local-proof runs a tiny random
CPU pipeline as an integration fixture, not an example of model image quality.
"""
import argparse,base64,json,os,select,subprocess,sys,time,zipfile
from pathlib import Path
from mcp_smoke import Client,META
from media_smoke import launch,wait
ROOT=Path(__file__).resolve().parents[1]

def run(binary,chrome=None,local=False):
 qa=ROOT/'qa/generation';qa.mkdir(parents=True,exist_ok=True);work=qa/'workspace';work.mkdir(exist_ok=True)
 folder='data-'+str(time.time_ns());server,log,connection,url=launch(binary,qa,folder);c=Client(work,connection);report={}
 try:
  state=c.call('studio_generation',{**c.guard(),'operation':'models'})
  assert len(state['catalog']['models'])>=14
  assert not any(key in state['connections'] for key in ['hf_token','civitai_token','kaggle_token'])
  def job(operation,**kw):return c.call('studio_generation',{**c.guard(),'operation':operation,**kw})
  def edit(cmd):return c.call('studio_edit',{**c.guard(),'commands':cmd})
  def save(j,name,**kw):
   wait(c,j,**kw);return c.call('studio_media_job',{**c.guard(),'id':j['id'],'operation':'save','path':name,'overwrite':True})
  request={'model_id':'SG161222/RealVisXL_V5.0','prompt':'A sunlit ceramic vase on a wooden table','width':512,'height':512,'steps':4,'seed':17,'precision':'fp16','content_mode':'flexible','adapter':'sdxl-lightning-4','timeout':900}
  guard=c.guard();j=job('prepare',**request);save(j,'inference.zip');assert c.guard()==guard
  with zipfile.ZipFile(work/'inference.zip') as z:
   meta=json.loads(z.read('kernel-metadata.json'));manifest=json.loads(z.read('manifest.json'))
   assert meta['is_private'] is True and meta['enable_gpu'] is True
   assert manifest['spec']['steps']==4 and manifest['spec']['guidance']==0 and manifest['timeout']==900
   assert len(manifest['spec']['model']['revision'])==40
   assert not any(key in manifest['spec'] for key in ['hf_token','civitai_token','kaggle_token'])
  c.call('studio_generation',{**c.guard(),'operation':'prepare',**request,'reference_path':'/etc/passwd'},error=True)
  stale=c.guard();edit([{'action':'title','value':'Generation acceptance proof'}])
  c.call('studio_generation',{**stale,'operation':'prepare',**request},error=True)
  invalid=job('prepare',**{**request,'model_id':'Qwen/Qwen-Image-2.1','adapter':''});wait(c,invalid,('failed',))
  missing=job('submit',**request);wait(c,missing,('failed',))
  ref=job('prepare',**{**request,'model_id':'black-forest-labs/FLUX.2-klein-4B','adapter':'','reference_page':0});save(ref,'reference.zip')
  with zipfile.ZipFile(work/'reference.zip') as z:
   manifest=json.loads(z.read('manifest.json'));assert manifest['spec']['reference']=='/kaggle/working/reference.png'
   code=''.join(json.loads(z.read('inference.ipynb'))['cells'][2]['source']);assert 'reference.png' in code
  report['mcp']='15 tools; private pinned notebooks; compatible adapter; explicit reference; stale guards; research and missing-connection failures'
  # A full, real inference through the app worker and import/undo/cold-open path.
  if local:
   data=qa/folder/'canvas-studio/ai';data.mkdir(parents=True,exist_ok=True)
   tiny={'id':'dg845/tiny-random-stable-diffusion-xl','name':'TEST ONLY · tiny random CPU pipeline','provider':'huggingface','pipeline':'StableDiffusionXLPipeline','task':'image','revision':'7ae769dd7f63b06308f2f23881d329a0cb1c89d2','runnable':True,'commercial':'research','license':'test fixture','weights_free':True,'tier':'test','safety_profile':'publisher','url':'https://huggingface.co/dg845/tiny-random-stable-diffusion-xl','steps':1,'guidance':0,'vram_gb':0,'notes':'Random integration test weights; no creative quality claim.'}
   (data/'custom-models.json').write_text(json.dumps([tiny]))
   generated=job('generate',model_id=tiny['id'],prompt='A ceramic vase',width=256,height=256,steps=1,seed=17,guidance=0,precision='fp32',device='cpu',offload=False,use='research',content_mode='flexible',download_limit_gb=2)
   save(generated,'tiny-test-only.png',timeout=240)
   before=c.call('studio_document');edit([{'action':'ai_import','id':generated['id'],'page':0}]);after=c.call('studio_document')
   assert len(after['elements'])==len(before['elements'])+1 and after['elements'][-1]['type']=='image'
   c.call('studio_history',{**c.guard(),'direction':'undo'});assert len(c.call('studio_document')['elements'])==len(before['elements'])
   c.call('studio_history',{**c.guard(),'direction':'redo'})
   c.call('studio_export',{**c.guard(),'format':'canvas','path':'generated.canvas','overwrite':True})
   subprocess.run([str(binary),'--verify-project',str(work/'generated.canvas')],check=True,capture_output=True)
   c.call('studio_media_job',{**c.guard(),'document_id':'wrong','id':generated['id'],'operation':'status'},error=True)
   report['cpu_inference']='Real safetensors Diffusers execution through app worker, PNG delivery, image import, undo/redo, portable cold-open; random test weights only'
  # Standalone Kaggle MCP uses the same backend and both protocol versions.
  env={**os.environ,'XDG_DATA_HOME':str(qa/folder)}
  p=subprocess.Popen([sys.executable,str(ROOT/'agents/kaggle_studio_mcp.py'),'--workspace',str(work)],env=env,stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
  try:
   def rpc(id,method,params):
    p.stdin.write(json.dumps({'jsonrpc':'2.0','id':id,'method':method,'params':{'_meta':META,**params}})+'\n');p.stdin.flush()
    assert select.select([p.stdout],[],[],60)[0],'Kaggle MCP timeout'
    r=json.loads(p.stdout.readline());assert 'error' not in r,r;return r['result']
   assert len(rpc(1,'tools/list',{})['tools'])==6
   r=rpc(2,'tools/call',{'name':'kaggle_prepare_inference','arguments':{'document_id':'standalone-proof',**request}})
   assert not r.get('isError'),r
   package=Path(r['structuredContent']['package']);assert package.is_file() and package.parent==work.resolve()
   report['kaggle_mcp']='Six tools; modern protocol; notebook exported inside workspace'
  finally:
   p.stdin.close();p.wait(timeout=10);assert p.returncode==0,p.stderr.read()
  if chrome:
   from playwright.sync_api import sync_playwright
   errors=[]
   with sync_playwright() as pw:
    browser=pw.chromium.launch(headless=True,**({} if chrome=='chromium' else {'executable_path':chrome}))
    context=browser.new_context(viewport={'width':1600,'height':1100},accept_downloads=True);page=context.new_page();page.on('pageerror',lambda e:errors.append(str(e)))
    page.goto(url);page.wait_for_function('() => !!state')
    page.get_by_role('button',name='Image and video generation',exact=True).click();page.wait_for_selector('.ai-model-card')
    assert page.locator('#ai-models .ai-model-card').count()>=8
    page.locator('#ai-models .ai-model-card').filter(has_text='FLUX.2 Klein').first.click()
    page.locator('#ai-form [name=prompt]').fill('A ceramic vase on a sunlit wooden desk')
    with page.expect_download() as dl:
     page.get_by_role('button',name='Prepare private GPU notebook',exact=True).click()
     page.locator('#ai-jobs').get_by_role('button',name='Save inference ZIP').first.click(timeout=20000)
    dl.value.save_as(work/'browser-inference.zip');assert zipfile.is_zipfile(work/'browser-inference.zip')
    page.locator('#ai-video-tab').click();page.wait_for_function("() => aiSelected().task==='video'")
    assert not page.locator('#ai-video-options').is_hidden()
    page.screenshot(path=qa/'generation-video.png',full_page=True)
    page.locator('#ai-edit').click();page.wait_for_selector('#film-studio:visible')
    page.get_by_role('button',name='Image and video generation',exact=True).click();page.wait_for_selector('#generation-studio:visible')
    page.locator('#ai-image-tab').click();page.locator('#ai-connections summary').click()
    page.locator('#ai-connections-form [name=kaggle_username]').fill('draft-proof')
    page.evaluate('aiLoad()');assert page.locator('#ai-connections-form [name=kaggle_username]').input_value()=='draft-proof'
    page.screenshot(path=qa/'generation-image.png',full_page=True)
    page.set_viewport_size({'width':720,'height':1000});assert page.evaluate('document.documentElement.scrollWidth<=window.innerWidth'),'Generation layout overflows'
    page.screenshot(path=qa/'generation-narrow.png',full_page=True)
    assert not errors,errors
    context.close();browser.close()
   report['browser']='Image/video selectors; source ZIP download; Video studio handoff; unsaved connection draft; narrow layout; no JS errors'
  report['production_gpu']='Not tested: requires user Kaggle authentication and account GPU quota. No remote job submitted.'
  (qa/'report.json').write_text(json.dumps(report,indent=2));print(json.dumps(report,indent=2))
 finally:c.close();server.terminate();server.wait(timeout=10);log.close()
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('--binary',default='./canvas-studio');p.add_argument('--chrome');p.add_argument('--local-proof',action='store_true');a=p.parse_args();run(Path(a.binary).resolve(),a.chrome,a.local_proof)
