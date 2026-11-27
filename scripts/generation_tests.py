"""Provider-independent tests for private finite inference jobs, input validation and weight selection."""
import ast,hashlib,json,os,sys,tempfile,unittest,zipfile
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch
ROOT=Path(__file__).resolve().parents[1];sys.path.insert(0,str(ROOT/'generation'))
import studio_ai as ai
from studio_inference import validate,select_files
class GenerationTests(unittest.TestCase):
 def setUp(self):
  self.temp=tempfile.TemporaryDirectory();self.folder=Path(self.temp.name);self.env=patch.dict(os.environ,{'XDG_DATA_HOME':str(self.folder/'data')});self.env.start()
  self.request={'model_id':'SG161222/RealVisXL_V5.0','prompt':'Fictional adult fashion creator in a tailored jacket, natural window light','width':512,'height':512,'steps':4,'seed':17,'precision':'fp16','content_mode':'flexible','document_id':'test-document'}
 def tearDown(self):self.env.stop();self.temp.cleanup()
 def job(self,name='job-1234567890abcdef'):p=self.folder/name;p.mkdir(exist_ok=True);return p
 def test_validation_rejects_research_advertising_unsafe_limits_and_bad_adapter(self):
  for update in [{'model_id':'Qwen/Qwen-Image-2.1'},{'width':99999},{'frames':122,'model_id':'Wan-AI/Wan2.1-T2V-1.3B-Diffusers'},{'seed':-1},{'quantization':'nf4'},{'model_id':'Wan-AI/Wan2.1-T2V-1.3B-Diffusers','adapter':'sdxl-lightning-4'}]:
   with self.subTest(update=update),self.assertRaises(ValueError):ai.spec({**self.request,**update})
  self.assertEqual(ai.spec({**self.request,'model_id':'Qwen/Qwen-Image-2.1','use':'research'})['use'],'research')
 def test_every_curated_runnable_recipe_accepts_its_documented_research_defaults(self):
  for model in ai.catalog()['models']:
   if not model['runnable']:continue
   with self.subTest(model=model['id']):
    s=ai.spec({**self.request,'model_id':model['id'],'steps':model['steps'],'guidance':model['guidance'],'use':'research','license_acknowledged':True})
    self.assertEqual(s['model']['id'],model['id'])
 def test_package_private_pinned_and_has_no_credentials(self):
  ai.save_settings({'kaggle_username':'studio-test','kaggle_token':'secret-kaggle-test-value','hf_token':'secret-hf-test-value','civitai_token':'secret-civi-test-value'})
  job=self.job();request={**self.request,'prompt':"An adult designer says: '); __import__('os').system('false'); #",'device':'cpu','precision':'fp32'}
  result=ai.prepare(request,job)
  self.assertTrue(result['private']);meta=json.loads((job/'inference/kernel-metadata.json').read_text());self.assertIs(meta['is_private'],True);self.assertTrue(meta['enable_gpu'])
  notebook=json.loads((job/'inference/inference.ipynb').read_text());joined='\n'.join(''.join(c['source']) for c in notebook['cells'])
  for secret in ('secret-kaggle-test-value','secret-hf-test-value','secret-civi-test-value'):self.assertNotIn(secret,joined)
  for c in notebook['cells']:
   if c['cell_type']=='code':ast.parse(''.join(c['source']))
  manifest=json.loads((job/'inference/manifest.json').read_text());self.assertEqual(manifest['spec']['device'],'cuda');self.assertEqual(manifest['spec']['model']['revision'],ai.spec(self.request)['model']['revision']);self.assertEqual(len(manifest['runtime_sha256']),64)
  self.assertEqual((ai.root()/'connections.json').stat().st_mode&0o777,0o600)
  with zipfile.ZipFile(job/'motion-source.zip') as z:self.assertIsNone(z.testzip());self.assertEqual(set(z.namelist()),{'inference.ipynb','kernel-metadata.json','manifest.json'})
 def test_reference_is_explicit_notebook_data(self):
  image=self.folder/'reference.png';image.write_bytes(b'fixture bytes');job=self.job();ai.prepare({**self.request,'model_id':'black-forest-labs/FLUX.2-klein-4B','reference':str(image)},job)
  manifest=json.loads((job/'inference/manifest.json').read_text());self.assertEqual(manifest['spec']['reference'],'/kaggle/working/reference.png');self.assertNotIn(str(self.folder),(job/'inference/inference.ipynb').read_text())
 def test_submit_requires_connection_and_bounded_private_gpu(self):
  with self.assertRaises(ValueError):ai.submit(self.request,self.job())
  ai.save_settings({'kaggle_username':'studio-test','kaggle_token':'test-token'})
  with patch.object(ai,'cli',return_value='Kernel successfully pushed') as cli:
   result=ai.submit(self.request,self.job());args=cli.call_args.args[0];self.assertIn('NvidiaTeslaT4',args);self.assertIn('--timeout',args);self.assertTrue(result['submitted']);self.assertTrue(result['private'])
   with self.assertRaises(ValueError):ai.run_record(result['run_id'],'another-document')
   self.assertEqual(ai.run_record(result['run_id'],'test-document')['kernel'],result['kernel'])
  with self.assertRaises(ValueError):ai.prepare({**self.request,'timeout':999999},self.job('job-too-long'))
 def test_download_checks_receipt_and_document(self):
  ai.save_settings({'kaggle_username':'studio-test','kaggle_token':'test-token'})
  with patch.object(ai,'cli',return_value='pushed'):result=ai.submit(self.request,self.job())
  folder=self.job('job-download')
  def fake_cli(args,*a):
   if args[1]=='status':return 'Kernel has status complete'
   path=Path(args[args.index('-p')+1]);(path/'output.png').write_bytes(b'file');(path/'receipt.json').write_text(json.dumps({'output_sha256':'bad'}));return 'downloaded'
  with patch.object(ai,'cli',side_effect=fake_cli),self.assertRaises(ValueError):ai.remote_download({'run_id':result['run_id'],'document_id':'test-document'},folder)
  self.assertFalse((folder/'frame.png').exists())
 def test_weight_variant_selection_skips_pickle_and_python(self):
  files=[SimpleNamespace(rfilename=n,size=10) for n in ['model_index.json','unet/config.json','unet/diffusion_pytorch_model.safetensors','unet/diffusion_pytorch_model.fp16.safetensors','unet/diffusion_pytorch_model.bin','custom_pipeline.py','tokenizer/vocab.json','sdxl-full.safetensors']]
  names=[f.rfilename for f in select_files(files,'fp16')]
  self.assertIn('unet/diffusion_pytorch_model.fp16.safetensors',names);self.assertNotIn('unet/diffusion_pytorch_model.safetensors',names);self.assertNotIn('custom_pipeline.py',names);self.assertFalse(any(n.endswith('.bin') for n in names));self.assertNotIn('sdxl-full.safetensors',names)
 def test_discovery_returns_text_only_and_does_not_fetch_preview_images(self):
  rows=[{'id':'owner/new-model','sha':'a'*40,'downloads':2,'likes':1,'tags':['uncensored'],'cardData':{'license':'apache-2.0'},'pipeline_tag':'text-to-image','images':[{'url':'https://example.invalid/private.png'}]}]
  with patch.object(ai,'request_json',return_value=rows) as call:
   result=ai.search({'provider':'huggingface','query':'new-model'});self.assertEqual(call.call_count,1);self.assertNotIn('images',result['models'][0]);self.assertEqual(result['models'][0]['safety_profile'],'declared_uncensored')
 def test_civitai_paid_access_and_pickle_are_not_registered(self):
  for version in [{'availability':'EarlyAccess','baseModel':'SDXL 1.0','id':12},{'availability':'Public','baseModel':'SDXL 1.0','id':12,'files':[{'type':'Model','name':'weights.ckpt','hashes':{'SHA256':'a'*64}}]}]:
   with patch.object(ai,'request_json',return_value={'name':'Model','type':'Checkpoint','modelVersions':[version]}),self.assertRaises(ValueError):ai.register({'provider':'civitai','id':'123'})
 def test_secrets_are_redacted(self):
  ai.save_settings({'hf_token':'secret-HF-token'});self.assertNotIn('secret-HF-token',ai.redact('request failed with secret-HF-token'));self.assertNotIn('xyz',ai.redact('Authorization: Bearer xyz'))
if __name__=='__main__':unittest.main(verbosity=2)
