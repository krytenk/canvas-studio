"""Provider-independent tests for private finite inference jobs, input validation and weight selection."""
import ast,hashlib,json,os,sys,tempfile,unittest,zipfile,urllib.request,urllib.error,subprocess
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch,MagicMock
ROOT=Path(__file__).resolve().parents[1];sys.path.insert(0,str(ROOT/'generation'))
import studio_ai as ai
REAL_PREFLIGHT=ai.preflight
from studio_inference import validate,select_files,component_overrides
class GenerationTests(unittest.TestCase):
 def setUp(self):
  self.temp=tempfile.TemporaryDirectory();self.folder=Path(self.temp.name);self.env=patch.dict(os.environ,{'XDG_DATA_HOME':str(self.folder/'data')});self.env.start()
  self.preflight_patch=patch.object(ai,'preflight',return_value={'weight_bytes':1024,'download_gib':.01,'checked_before_gpu':True});self.preflight_patch.start()
  self.request={'model_id':'SG161222/RealVisXL_V5.0','prompt':'Fictional adult fashion creator in a tailored jacket, natural window light','width':512,'height':512,'steps':4,'seed':17,'precision':'fp16','content_mode':'flexible','document_id':'test-document'}
 def tearDown(self):self.preflight_patch.stop();self.env.stop();self.temp.cleanup()
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
  self.assertEqual(meta['title'],meta['id'].split('/')[1])
  notebook=json.loads((job/'inference/inference.ipynb').read_text());joined='\n'.join(''.join(c['source']) for c in notebook['cells'])
  self.assertEqual(len({cell['id'] for cell in notebook['cells']}),3)
  setup=''.join(notebook['cells'][1]['source']);self.assertIn("'uninstall','--yes','torchao'",setup);self.assertIn('peft==0.22.0',setup);self.assertIn('/kaggle/temp/canvas-studio-model-cache',joined);self.assertNotIn('/kaggle/working/model-cache',joined)
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
  with patch.object(ai,'cli',return_value='Kernel version 1 successfully pushed. Please check progress at https://www.kaggle.com/code/studio-test/canonical-notebook') as cli:
   result=ai.submit(self.request,self.job());args=cli.call_args.args[0];self.assertIn('NvidiaTeslaT4',args);self.assertIn('--timeout',args);self.assertTrue(result['submitted']);self.assertTrue(result['private'])
   with self.assertRaises(ValueError):ai.run_record(result['run_id'],'another-document')
   self.assertEqual(ai.run_record(result['run_id'],'test-document')['kernel'],result['kernel'])
   self.assertEqual(result['kernel'],'studio-test/canonical-notebook');self.assertEqual(ai.versioned_kernel(ai.run_record(result['run_id'],'test-document')),'studio-test/canonical-notebook/1')
   meta=json.loads((self.job()/'inference/kernel-metadata.json').read_text());self.assertEqual(meta['id'],result['kernel']);self.assertEqual(meta['title'],'canonical-notebook')
  with self.assertRaises(ValueError):ai.prepare({**self.request,'timeout':999999},self.job('job-too-long'))
 def test_download_checks_receipt_and_document(self):
  ai.save_settings({'kaggle_username':'studio-test','kaggle_token':'test-token'})
  with patch.object(ai,'cli',return_value='Kernel version 1 successfully pushed. Please check progress at https://www.kaggle.com/code/studio-test/canonical-notebook'):result=ai.submit(self.request,self.job())
  folder=self.job('job-download')
  def fake_cli(args,*a):
   if args[1]=='status':return 'Kernel has status complete'
   path=Path(args[args.index('-p')+1]);(path/'output.png').write_bytes(b'file');(path/'receipt.json').write_text(json.dumps({'output_sha256':'bad'}));return 'downloaded'
  with patch.object(ai,'cli',side_effect=fake_cli),self.assertRaises(ValueError):ai.remote_download({'run_id':result['run_id'],'document_id':'test-document'},folder)
  self.assertFalse((folder/'frame.png').exists())
 def test_download_budget_is_checked_before_submitting_a_gpu_notebook(self):
  meta={'siblings':[{'rfilename':'model_index.json','size':100},{'rfilename':'unet/diffusion_pytorch_model.safetensors','size':27*1024**3}]}
  ai.save_settings({'kaggle_username':'studio-test','kaggle_token':'test-token'})
  with patch.object(ai,'request_json',return_value=meta):
   with self.assertRaisesRegex(ValueError,'No GPU notebook was submitted'):REAL_PREFLIGHT(self.request)
   self.assertGreater(REAL_PREFLIGHT({**self.request,'download_limit_gb':32})['download_gib'],26)
  with patch.object(ai,'preflight',side_effect=ValueError('Download budget too small')),patch.object(ai,'cli') as cli,self.assertRaises(ValueError):ai.submit(self.request,self.job())
  cli.assert_not_called()
 def test_zero_exit_push_error_or_auth_help_is_not_a_successful_submission(self):
  process=SimpleNamespace(returncode=0,stdout='Kernel push error: GPU unavailable',stderr='')
  with patch.object(ai.subprocess,'run',return_value=process),self.assertRaisesRegex(ValueError,'GPU unavailable'):ai.cli(['kernels','push'])
  ai.save_settings({'kaggle_username':'studio-test','kaggle_token':'test-token'})
  with patch.object(ai,'cli',return_value='Authentication required. Run kaggle auth login.'),self.assertRaisesRegex(ValueError,'did not confirm'):ai.submit(self.request,self.job())
 def test_timed_out_submission_can_be_checked_without_creating_a_duplicate(self):
  ai.save_settings({'kaggle_username':'studio-test','kaggle_token':'test-token'})
  with patch.object(ai,'cli',side_effect=subprocess.TimeoutExpired('kaggle',120)) as cli:
   result=ai.submit(self.request,self.job());self.assertFalse(result['submitted']);self.assertEqual(cli.call_count,1)
   self.assertEqual(ai.run_record(result['run_id'],'test-document')['submission'],'unconfirmed')
 def test_remote_failure_log_is_readable_and_redacted_before_truncation(self):
  ai.save_settings({'hf_token':'fake-split-secret'})
  raw=json.dumps([{'data':'old line\n'*1000},{'data':'token: fake-split-'},{'data':'secret\nImportError: incompatible optional backend'}])
  tail=ai.format_log_tail(raw);self.assertNotIn('fake-split-secret',tail);self.assertIn('[redacted]',tail);self.assertTrue(tail.endswith('ImportError: incompatible optional backend'));self.assertLessEqual(len(tail),4000)
 def test_wan_decoder_loads_in_fp32_without_changing_image_pipelines(self):
  loader=MagicMock();torch=SimpleNamespace(float32='FP32');diffusers=SimpleNamespace(AutoencoderKLWan=SimpleNamespace(from_pretrained=loader))
  files=[SimpleNamespace(rfilename='vae/diffusion_pytorch_model.fp16.safetensors')]
  kwargs=component_overrides({'pipeline':'WanPipeline'},'/snapshot',files,'fp16',torch,diffusers)
  self.assertIn('vae',kwargs);self.assertEqual(loader.call_args.kwargs['torch_dtype'],'FP32');self.assertEqual(loader.call_args.kwargs['variant'],'fp16')
  loader.reset_mock();self.assertEqual(component_overrides({'pipeline':'StableDiffusionXLPipeline'},'/snapshot',files,'fp16',torch,diffusers),{});loader.assert_not_called()
 def test_valid_hash_from_another_model_is_not_imported(self):
  ai.save_settings({'kaggle_username':'studio-test','kaggle_token':'test-token'})
  with patch.object(ai,'cli',return_value='Kernel version 2 successfully pushed. Please check progress at https://www.kaggle.com/code/studio-test/canonical-notebook'):result=ai.submit(self.request,self.job())
  folder=self.job('wrong-receipt')
  def fake_cli(args,*a):
   self.assertEqual(args[2],'studio-test/canonical-notebook/2')
   if args[1]=='status':return 'KernelWorkerStatus.COMPLETE'
   dest=Path(args[args.index('-p')+1]);(dest/'output.png').write_bytes(b'file');(dest/'receipt.json').write_text(json.dumps({'output_sha256':hashlib.sha256(b'file').hexdigest(),'model':{'id':'other/model','revision':'a'*40}}));return 'downloaded'
  with patch.object(ai,'cli',side_effect=fake_cli),self.assertRaisesRegex(ValueError,'different model'):ai.remote_download({'run_id':result['run_id'],'document_id':'test-document'},folder)
 def test_weight_variant_selection_skips_pickle_and_python(self):
  files=[SimpleNamespace(rfilename=n,size=10) for n in ['model_index.json','unet/config.json','unet/diffusion_pytorch_model.safetensors','unet/diffusion_pytorch_model.fp16.safetensors','unet/diffusion_pytorch_model.bin','custom_pipeline.py','tokenizer/vocab.json','sdxl-full.safetensors']]
  names=[f.rfilename for f in select_files(files,'fp16')]
  self.assertIn('unet/diffusion_pytorch_model.fp16.safetensors',names);self.assertNotIn('unet/diffusion_pytorch_model.safetensors',names);self.assertNotIn('custom_pipeline.py',names);self.assertFalse(any(n.endswith('.bin') for n in names));self.assertNotIn('sdxl-full.safetensors',names)
 def test_discovery_returns_text_only_and_does_not_fetch_preview_images(self):
  rows=[{'id':'owner/new-model','sha':'a'*40,'downloads':2,'likes':1,'tags':['uncensored'],'cardData':{'license':'apache-2.0'},'pipeline_tag':'text-to-image','images':[{'url':'https://example.invalid/private.png'}]}]
  with patch.object(ai,'request_json',return_value=rows) as call:
   result=ai.search({'provider':'huggingface','query':'new-model'});self.assertEqual(call.call_count,1);self.assertNotIn('images',result['models'][0]);self.assertEqual(result['models'][0]['safety_profile'],'declared_uncensored')
 def test_saved_huggingface_token_is_used_for_search_and_gated_registration(self):
  ai.save_settings({'hf_token':'fake-provider-token'})
  with patch.object(ai,'request_json',return_value=[]) as request:
   ai.search({'provider':'huggingface','query':'test'});self.assertEqual(request.call_args.args[1],'fake-provider-token')
  with patch.object(ai,'request_json',side_effect=[{'sha':'a'*40,'gated':True,'cardData':{'license':'apache-2.0'}},{'_class_name':'StableDiffusionXLPipeline'}]) as request:
   result=ai.register({'provider':'huggingface','id':'owner/test-model'});self.assertEqual(request.call_count,2)
   self.assertTrue(all(call.args[1]=='fake-provider-token' for call in request.call_args_list));self.assertNotIn('fake-provider-token',json.dumps(result))
 def test_metadata_redirects_do_not_forward_bearer_tokens_to_other_hosts(self):
  def builder(handler):
   req=urllib.request.Request('https://huggingface.co/file',headers={'Authorization':'Bearer fake-secret'})
   same=handler.redirect_request(req,None,302,'',{},'https://huggingface.co/next');self.assertEqual(same.get_header('Authorization'),'Bearer fake-secret')
   other=handler.redirect_request(req,None,302,'',{},'https://cdn.example.invalid/next');self.assertIsNone(other.get_header('Authorization'))
   with self.assertRaises(ValueError):handler.redirect_request(req,None,302,'',{},'http://cdn.example.invalid/next')
   response=MagicMock();response.__enter__.return_value.read.return_value=b'[]';opener=MagicMock();opener.open.return_value=response;return opener
  with patch.object(ai.urllib.request,'build_opener',side_effect=builder):self.assertEqual(ai.request_json('https://huggingface.co/api/models','fake-secret'),[])
 def test_provider_access_and_rate_limit_errors_are_actionable_without_secret_echo(self):
  for status,expected in [(403,'denied this request'),(429,'rate limiting'),(503,'HTTP 503')]:
   opener=MagicMock();opener.open.side_effect=urllib.error.HTTPError('https://huggingface.co/secret-query',status,'',None,None)
   with patch.object(ai.urllib.request,'build_opener',return_value=opener),self.assertRaisesRegex(ValueError,expected):ai.request_json('https://huggingface.co/api/models','fake-secret')
 def test_civitai_paid_access_and_pickle_are_not_registered(self):
  for version in [{'availability':'EarlyAccess','baseModel':'SDXL 1.0','id':12},{'availability':'Public','baseModel':'SDXL 1.0','id':12,'files':[{'type':'Model','name':'weights.ckpt','hashes':{'SHA256':'a'*64}}]}]:
   with patch.object(ai,'request_json',return_value={'name':'Model','type':'Checkpoint','modelVersions':[version]}),self.assertRaises(ValueError):ai.register({'provider':'civitai','id':'123'})
 def test_secrets_are_redacted(self):
  ai.save_settings({'hf_token':'secret-HF-token'});self.assertNotIn('secret-HF-token',ai.redact('request failed with secret-HF-token'));self.assertNotIn('xyz',ai.redact('Authorization: Bearer xyz'))
if __name__=='__main__':unittest.main(verbosity=2)
