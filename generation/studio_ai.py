"""Canvas Studio model discovery, private settings and finite Kaggle orchestration.
All network hosts/commands are fixed providers; model data and prompts are never shell code.
"""
import argparse,base64,hashlib,json,os,re,shutil,subprocess,sys,time,urllib.parse,urllib.request,urllib.error,zipfile
from pathlib import Path
from types import SimpleNamespace
from studio_inference import validate,run as infer,REPO,select_files,reference_items,reference_fingerprint
HERE=Path(__file__).resolve().parent
VERSION='1.19.0';KAGGLE_VERSION='2.2.5'
def root():
 p=Path(os.environ.get('XDG_DATA_HOME',Path.home()/'.local/share'))/'canvas-studio/ai';p.mkdir(parents=True,exist_ok=True);p.chmod(0o700);return p
def write(path,value):
 path=Path(path);path.parent.mkdir(parents=True,exist_ok=True);temp=path.with_name(path.name+'.tmp-'+str(os.getpid()));temp.write_text(json.dumps(value,indent=2,ensure_ascii=False));temp.chmod(0o600);temp.replace(path)
def settings():
 p=root()/'connections.json';return json.loads(p.read_text()) if p.exists() else {}
def save_settings(v):
 allowed={'kaggle_username','kaggle_token','hf_token','civitai_token','cache_dir'}
 if set(v)-allowed:raise ValueError('Unknown connection setting')
 data=settings()
 for k,value in v.items():
  if not isinstance(value,str) or len(value)>4096:raise ValueError('Invalid connection setting')
  if k=='kaggle_username' and value and not re.fullmatch('[A-Za-z0-9_-]{2,50}',value):raise ValueError('Enter your Kaggle username')
  if k=='cache_dir' and value:
   p=Path(value).expanduser();p.mkdir(parents=True,exist_ok=True);data[k]=str(p.resolve());continue
  if value:data[k]=value
 write(root()/'connections.json',data);return connection_state()
def tool_python():
 candidates=[os.environ.get('CANVAS_STUDIO_AI_PYTHON'),str(Path(os.environ.get('XDG_DATA_HOME',Path.home()/'.local/share'))/'canvas-studio/generation-tools/bin/python')]
 return next((p for p in candidates if p and Path(p).is_file()),sys.executable)
def kaggle():
 # Always invoke the official SDK; no shell lookup supplied by model metadata.
 return [tool_python(),'-m','kaggle']
def connection_state():
 s=settings();cache=Path(s.get('cache_dir',root()/'models'));cache.mkdir(parents=True,exist_ok=True)
 return {'kaggle_configured':bool(s.get('kaggle_token') or os.environ.get('KAGGLE_API_TOKEN') or (Path.home()/'.kaggle/access_token').exists() or (Path.home()/'.kaggle/kaggle.json').exists()),'kaggle_username':s.get('kaggle_username',os.environ.get('KAGGLE_USERNAME','')),'hf_configured':bool(s.get('hf_token') or os.environ.get('HF_TOKEN') or (Path.home()/'.cache/huggingface/token').exists()),'civitai_configured':bool(s.get('civitai_token') or os.environ.get('CIVITAI_API_KEY')),'cache_dir':str(cache),'free_cache_gb':round(shutil.disk_usage(cache).free/1024**3,1),'tools_installed':tool_python()!=sys.executable,'kaggle_version':KAGGLE_VERSION,'gpu_note':'Kaggle availability and remaining GPU quota are account-controlled; two T4s do not pool memory automatically.'}
def env():
 result=os.environ.copy();s=settings()
 for key,name in [('kaggle_token','KAGGLE_API_TOKEN'),('hf_token','HF_TOKEN'),('civitai_token','CIVITAI_API_KEY')]:
  if s.get(key):result[name]=s[key]
 return result
def redact(text,max_length=5000):
 for key in ('hf_token','kaggle_token','civitai_token'):
  secret=settings().get(key) or os.environ.get({'hf_token':'HF_TOKEN','kaggle_token':'KAGGLE_API_TOKEN','civitai_token':'CIVITAI_API_KEY'}[key])
  if secret:text=text.replace(secret,'[redacted]')
 clean=re.sub(r'(Bearer\s+)[^\s\"\']+',r'\1[redacted]',text)
 return clean if max_length is None else clean[:max_length]
def cli(args,timeout=90,max_length=5000):
 p=subprocess.run([*kaggle(),*args],env=env(),stdin=subprocess.DEVNULL,capture_output=True,text=True,timeout=timeout)
 if p.returncode:raise ValueError(redact(p.stderr or p.stdout or 'Kaggle CLI failed'))
 # The official push CLI can print an API error while exiting with status zero.
 if (args[:2]==['kernels','push'] and re.search(r'(?im)^Kernel push error:',p.stdout)) or (args[:2]==['datasets','create'] and re.search(r'(?im)^Dataset creation error:',p.stdout)):raise ValueError(redact(p.stdout))
 return redact(p.stdout,max_length)
def catalog():
 data=json.loads((HERE/'catalog.json').read_text());custom=root()/'custom-models.json'
 if custom.exists():data['models'].extend(json.loads(custom.read_text()))
 return data
def request_json(url,token=None):
 class Redirect(urllib.request.HTTPRedirectHandler):
  def redirect_request(self,req,fp,code,msg,headers,newurl):
   if urllib.parse.urlsplit(newurl).scheme!='https':raise ValueError('Refusing a non-HTTPS model metadata redirect')
   redirected=super().redirect_request(req,fp,code,msg,headers,newurl)
   if urllib.parse.urlsplit(req.full_url).netloc!=urllib.parse.urlsplit(newurl).netloc:redirected.remove_header('Authorization')
   return redirected
 headers={'User-Agent':'CanvasStudio/1.16 (+model metadata discovery)','Accept':'application/json'}
 if token:headers['Authorization']='Bearer '+token
 provider='Hugging Face' if urllib.parse.urlsplit(url).hostname=='huggingface.co' else 'Civitai'
 try:
  with urllib.request.build_opener(Redirect()).open(urllib.request.Request(url,headers=headers),timeout=25) as r:
   data=r.read(4*1024**2+1)
   if len(data)>4*1024**2:raise ValueError('Model metadata exceeds response limit')
   return json.loads(data)
 except urllib.error.HTTPError as error:
  if error.code in (401,403):raise ValueError(f'{provider} denied this request (HTTP {error.code}). Check the saved provider token and any required model access or website verification.') from None
  if error.code==429:raise ValueError(f'{provider} is rate limiting requests. Wait before trying again.') from None
  raise ValueError(f'{provider} returned HTTP {error.code}. Try again later or check the model page.') from None
 except urllib.error.URLError:raise ValueError(f'Could not connect to {provider}. Check your network and try again.') from None
def search(v):
 provider=v.get('provider','huggingface');query=v.get('query','').strip()
 if len(query)>160:raise ValueError('Search up to 160 characters')
 if provider=='huggingface':
  params={'limit':20,'sort':'trendingScore' if not query else 'downloads','direction':-1,'full':'true'}
  if query:params['search']=query
  else:params['pipeline_tag']='text-to-video' if v.get('task')=='video' else 'text-to-image'
  rows=request_json('https://huggingface.co/api/models?'+urllib.parse.urlencode(params),settings().get('hf_token') or os.environ.get('HF_TOKEN'));models=[]
  for m in rows:
   id=m.get('id','');card=m.get('cardData') or {};tags=m.get('tags',[])
   if not REPO.fullmatch(id):continue
   models.append({'id':id,'name':id,'provider':provider,'url':'https://huggingface.co/'+id,'revision':m.get('sha'),'license':card.get('license_name') or card.get('license','unverified'),'downloads':m.get('downloads',0),'likes':m.get('likes',0),'task':m.get('pipeline_tag','unknown'),'gated':m.get('gated',False),'safety_profile':'declared_uncensored' if 'uncensored' in tags or 'uncensored' in id.lower() else 'unknown','custom_code':'custom_code' in tags,'note':'Discovery metadata is not a quality benchmark or proof of commercial rights. Select a reviewed recipe before generation.'})
 elif provider=='civitai':
  params={'limit':20,'sort':'Most Downloaded','period':'Month','types':v.get('type','Checkpoint'),'nsfw':'true' if v.get('include_mature',False) else 'false'}
  if params['types'] not in ('Checkpoint','LORA'):raise ValueError('Search checkpoints or LoRAs')
  if query:params['query']=query
  rows=request_json('https://civitai.com/api/v1/models?'+urllib.parse.urlencode(params),settings().get('civitai_token'));models=[]
  for m in rows.get('items',[]):
   versions=m.get('modelVersions',[]);version=versions[0] if versions else {};files=[]
   for f in version.get('files',[]):
    if not str(f.get('name','')).endswith('.safetensors'):continue
    files.append({'name':f['name'],'size_kb':f.get('sizeKB'),'sha256':f.get('hashes',{}).get('SHA256'),'format':f.get('metadata',{}).get('format'),'precision':f.get('metadata',{}).get('fp')})
   models.append({'id':str(m['id']),'name':m.get('name','Unnamed'),'provider':provider,'url':'https://civitai.com/models/'+str(m['id']),'version_id':version.get('id'),'base_model':version.get('baseModel'),'type':m.get('type'),'safety_profile':'unknown','mature_listing':m.get('nsfw',False),'license':{'allowCommercialUse':m.get('allowCommercialUse'),'allowDerivatives':m.get('allowDerivatives'),'allowDifferentLicense':m.get('allowDifferentLicense'),'credit_required':not m.get('allowNoCredit',True)},'files':files,'note':'Mature listing tags do not establish censorship behaviour. Downloads require compatible safetensors, checksum and licence review; browse-only until a recipe is added.'})
 else:raise ValueError('Choose Hugging Face or Civitai')
 return {'models':models,'provider':provider,'query':query,'fetched_at':time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime()),'metadata_only':True}
def register(v):
 provider=v.get('provider');id=str(v.get('id',''))
 if provider=='huggingface':
  if not REPO.fullmatch(id):raise ValueError('Use a Hugging Face owner/model ID')
  token=settings().get('hf_token') or os.environ.get('HF_TOKEN');meta=request_json('https://huggingface.co/api/models/'+id,token);sha=meta['sha'];card=meta.get('cardData') or {};index=request_json('https://huggingface.co/'+id+'/resolve/'+sha+'/model_index.json',token)
  from studio_inference import SUPPORTED
  cls=index.get('_class_name','')
  if cls not in SUPPORTED:raise ValueError('This repository needs a dedicated audited recipe; no remote Python will be executed')
  license=card.get('license_name') or card.get('license','unverified');use='commercial' if license in ('apache-2.0','mit','openrail++','creativeml-openrail-m') else 'research' if 'research' in license else 'review'
  model=dict(id=id,provider=provider,name=id.split('/')[-1],task='video' if 'Video' in cls or cls in ('WanPipeline','WanVACEPipeline','LTXPipeline','LTX2Pipeline') else 'image',pipeline=cls,reference_required=cls in ('WanImageToVideoPipeline','LTXImageToVideoPipeline','WanVACEPipeline'),reference_max=4 if cls=='WanVACEPipeline' else 1,reference_kind='subject' if cls=='WanVACEPipeline' else 'first_frame',frame_multiple=8 if cls.startswith('LTX') else 4,revision=sha,gated=meta.get('gated',False),weights_free=True,commercial=use,license=license,license_url='https://huggingface.co/'+id+'/tree/'+sha,url='https://huggingface.co/'+id,notes='Live-discovered recipe. Compatibility and memory use must be checked on your GPU; not an independently benchmarked recommendation.',safety_profile='declared_uncensored' if 'uncensored' in id.lower() else 'unknown',tier='discovered',steps=30,guidance=4.0,runnable=True,vram_gb=None,verified_at=time.strftime('%Y-%m-%d',time.gmtime()))
 elif provider=='civitai':
  if not id.isdigit():raise ValueError('Use a Civitai model ID')
  meta=request_json('https://civitai.com/api/v1/models/'+id,settings().get('civitai_token'));version=next((x for x in meta.get('modelVersions',[]) if not v.get('version_id') or x['id']==v['version_id']),None)
  if not version:raise ValueError('Model version is missing')
  if version.get('availability') not in (None,'Public','Available'):raise ValueError('Only publicly downloadable weights are supported; this listing may require paid early access')
  base=version.get('baseModel','');family='StableDiffusionXLPipeline' if base.startswith('SDXL') else 'StableDiffusionPipeline' if base.startswith('SD 1.5') else None
  if not family:raise ValueError('This Civitai base family needs a dedicated recipe. Currently full SDXL / SD 1.5 safetensors checkpoints are supported')
  file=next((f for f in version.get('files',[]) if f.get('type')=='Model' and f.get('name','').endswith('.safetensors') and (not v.get('file_name') or f['name']==v['file_name'])),None)
  if not file or not re.fullmatch('[A-Fa-f0-9]{64}',file.get('hashes',{}).get('SHA256','')):raise ValueError('A checksum-verified safetensors model file is required')
  if meta.get('type')!='Checkpoint':raise ValueError('For inference select a full checkpoint. LoRA listings can be inspected but need a compatible adapter recipe')
  base_repo='stabilityai/stable-diffusion-xl-base-1.0' if family=='StableDiffusionXLPipeline' else 'stable-diffusion-v1-5/stable-diffusion-v1-5'
  base_info=request_json('https://huggingface.co/api/models/'+base_repo)
  model=dict(id='civitai/model-'+id+'-'+str(version['id']),provider=provider,name=meta['name']+' / '+version['name'],task='image',pipeline=family,revision=file['hashes']['SHA256'].lower(),configuration_repo=base_repo,configuration_revision=base_info['sha'],civitai_version=version['id'],weight_bytes=round(file.get('sizeKB',0)*1024),weight_precision=file.get('metadata',{}).get('fp'),weight_size=file.get('metadata',{}).get('size'),weights_free=True,commercial='review',license='Civitai publisher and base-model terms',license_metadata={key:meta.get(key) for key in ('allowCommercialUse','allowNoCredit','allowDerivatives')},license_url='https://civitai.com/models/'+id,url='https://civitai.com/models/'+id,notes='Full safetensors checkpoint, pinned by SHA-256. Review publisher and base-model rights before commercial use. Mature tags do not establish prompt-filter behaviour.',safety_profile='declared_uncensored' if 'uncensored' in meta['name'].lower() else 'unknown',tier='community',steps=28,guidance=5.0,runnable=True,vram_gb=10 if family=='StableDiffusionXLPipeline' else 6,verified_at=time.strftime('%Y-%m-%d',time.gmtime()))
 else:raise ValueError('Choose Hugging Face or Civitai')
 path=root()/'custom-models.json';existing=json.loads(path.read_text()) if path.exists() else [];existing=[m for m in existing if m['id']!=model['id']]
 if len(existing)>=100:raise ValueError('Keep at most 100 custom recipes')
 existing.append(model);write(path,existing);return {'registered':model}
def spec(v):
 data=catalog();model=next((m for m in data['models'] if m['id']==v.get('model_id')),None)
 if not model:raise ValueError('Select a reviewed model from the catalog')
 result={'model':model,'prompt':v.get('prompt',''),'negative_prompt':v.get('negative_prompt',''),'width':v.get('width',model.get('default_width',768)),'height':v.get('height',model.get('default_height',448 if model['task']=='video' else 1024)),'steps':v.get('steps',model['steps']),'guidance':v.get('guidance',model['guidance']),'seed':v.get('seed',20261127),'precision':v.get('precision',model.get('preferred_precision','fp16')),'quantization':v.get('quantization','none'),'offload':v.get('offload',True),'device':v.get('device','cuda'),'content_mode':v.get('content_mode','flexible'),'use':v.get('use','commercial'),'license_acknowledged':bool(v.get('license_acknowledged')),'download_limit_gb':v.get('download_limit_gb',model.get('recommended_download_limit_gb',20)),'frames':v.get('frames',model.get('default_frames',33)),'fps':v.get('fps',model.get('default_fps',8)),'adapters':[]}
 if v.get('adapter'):
  a=next((a for a in data['adapters'] if a['id']==v['adapter']),None)
  if not a:raise ValueError('Choose an available adapter')
  result['adapters']=[a];result['steps']=a['steps'];result['guidance']=a['guidance']
 sources=sum(bool(v.get(k)) for k in ('reference_id','reference_ids','reference','references'))
 if sources>1 and not (sources==2 and ((v.get('reference_id') and v.get('reference')) or (v.get('reference_ids') and v.get('references')))):raise ValueError('Choose one reference source')
 if v.get('reference_ids') is not None and v.get('references') is None:
  ids=v['reference_ids'];document=v.get('document_id','')
  if not isinstance(ids,list) or not 1<=len(ids)<=4 or not re.fullmatch('[A-Za-z0-9_-]{1,160}',document):raise ValueError('Choose one to four uploaded references for this document')
  items=[]
  for id in ids:
   if not isinstance(id,str) or not re.fullmatch('[a-f0-9]{64}',id):raise ValueError('Invalid reference image identity')
   path=root()/'references'/document/(id+'.png')
   if not path.is_file() or path.stat().st_size>25_000_000 or hashlib.sha256(path.read_bytes()).hexdigest()!=id:raise ValueError('Choose references uploaded for this document')
   items.append({'path':str(path),'sha256':id})
  result['references']=items
 elif v.get('references') is not None:
  paths=v['references']
  if not isinstance(paths,list) or not 1<=len(paths)<=4:raise ValueError('Choose one to four reference images')
  items=[]
  for value in paths:
   if not isinstance(value,str):raise ValueError('Invalid reference path')
   path=Path(value)
   if not path.is_file() or path.stat().st_size>25_000_000:raise ValueError('Choose reference images up to 25 MB each')
   items.append({'path':str(path),'sha256':hashlib.sha256(path.read_bytes()).hexdigest()})
  result['references']=items
 elif v.get('reference_id') and not v.get('reference'):
  document=v.get('document_id','');id=v['reference_id']
  if not re.fullmatch('[A-Za-z0-9_-]{1,160}',document) or not isinstance(id,str) or not re.fullmatch('[a-f0-9]{64}',id):raise ValueError('Invalid document/reference identity')
  path=root()/'references'/document/(id+'.png')
  if not path.is_file() or path.stat().st_size>25_000_000 or hashlib.sha256(path.read_bytes()).hexdigest()!=id:raise ValueError('Choose a reference uploaded for this document')
  result['reference']=str(path)
 elif v.get('reference'):result['reference']=v['reference']
 if result.get('reference'):
  path=Path(result['reference'])
  if not path.is_file() or path.stat().st_size>25_000_000:raise ValueError('Choose a reference image up to 25 MB')
  result['reference_sha256']=hashlib.sha256(path.read_bytes()).hexdigest()
 result['reference_fit']=v.get('reference_fit','contain');result['reference_strength']=v.get('reference_strength',1)
 if reference_items(result):result['reference_set_sha256']=reference_fingerprint(result)
 return validate(result)
def prepare(v,folder):
 remote=dict(v);remote['device']='cuda';s=spec(remote);timeout=v.get('timeout',1800)
 if type(timeout) is not int or not 300<=timeout<=3600:raise ValueError('Kaggle time limit must be 300–3600 seconds')
 folder=Path(folder);dest=folder/'inference';dest.mkdir(exist_ok=True)
 username=connection_state()['kaggle_username'] or 'your-kaggle-username';slug='canvas-studio-'+folder.name[-24:].lower().replace('_','-');handle=username+'/'+slug
 portable=json.loads(json.dumps(s));reference=[]
 for index,(path,digest) in enumerate(reference_items(s)):
  name='reference.png' if s.get('reference') else 'reference-'+str(index+1).zfill(2)+'.png'
  shutil.copyfile(path,dest/name);target='/kaggle/working/'+name;reference.append((target,digest))
  if s.get('reference'):portable['reference']=target
  else:portable['references'][index]={**s['references'][index],'path':target}
 code=(HERE/'studio_inference.py').read_text();runtime_sha=hashlib.sha256(code.encode()).hexdigest()
 # Kaggle's GPU image currently carries torchao 0.10. PEFT 0.22 rejects that
 # optional backend even for ordinary LoRA. This runner uses bitsandbytes NF4,
 # not torchao; remove it only in this disposable remote notebook environment.
 setup="import sys,subprocess\nsubprocess.check_call([sys.executable,'-m','pip','uninstall','--yes','torchao'])\nsubprocess.check_call([sys.executable,'-m','pip','install','--quiet','diffusers==0.43.0','transformers==5.24.1','accelerate==1.16.0','safetensors','sentencepiece','ftfy==6.3.1','protobuf','imageio[ffmpeg]','peft==0.22.0','bitsandbytes==0.50.2'])\nimport torch,platform\nprint('Python',platform.python_version(),'Torch',torch.__version__,'CUDA available',torch.cuda.is_available())\nif torch.cuda.is_available():print('GPU',torch.cuda.get_device_name(0))"
 program="import base64,json,os\nfrom pathlib import Path\nPath('/kaggle/working/studio_inference.py').write_bytes(base64.b64decode("+repr(base64.b64encode(code.encode()).decode())+"))\n"
 if reference:program+="import hashlib,shutil\ninputs="+repr(reference)+"\nfor location,expected in inputs:\n target=Path(location)\n if not target.exists():\n  for candidate in Path('/kaggle/input').rglob('*.png'):\n   if candidate.stat().st_size<=25000000 and hashlib.sha256(candidate.read_bytes()).hexdigest()==expected:\n    shutil.copyfile(candidate,target);break\n if not target.exists() or hashlib.sha256(target.read_bytes()).hexdigest()!=expected:raise ValueError('Attach every matching private reference input before running this notebook')\n"
 program+="try:\n from kaggle_secrets import UserSecretsClient\n os.environ['HF_TOKEN']=UserSecretsClient().get_secret('HF_TOKEN')\nexcept Exception:\n pass\n"
 program+="try:\n from kaggle_secrets import UserSecretsClient\n os.environ['CIVITAI_API_KEY']=UserSecretsClient().get_secret('CIVITAI_API_KEY')\nexcept Exception:\n pass\n"
 program+="from studio_inference import run\nSPEC=json.loads("+repr(json.dumps(portable))+ ")\nCACHE='/kaggle/temp/canvas-studio-model-cache'\ntry:\n run(SPEC,'/kaggle/working',CACHE,print)\nfinally:\n import shutil\n shutil.rmtree(CACHE,ignore_errors=True)\n"
 # Do not retain huge model caches as notebook outputs.
 cells=[{'cell_type':'markdown','metadata':{},'source':['# Canvas Studio private inference\nFinite GPU job. Model weights/licence, quota and access requirements still apply. No credentials are embedded.']},{'cell_type':'code','metadata':{},'execution_count':None,'outputs':[],'source':setup.splitlines(True)},{'cell_type':'code','metadata':{},'execution_count':None,'outputs':[],'source':program.splitlines(True)}]
 for i,cell in enumerate(cells):cell['id']='studio-'+str(i)
 write(dest/'inference.ipynb',{'nbformat':4,'nbformat_minor':5,'metadata':{'kernelspec':{'name':'python3','display_name':'Python 3','language':'python'}},'cells':cells})
 if (dest/'inference.ipynb').stat().st_size>900_000:raise ValueError('Notebook source exceeds its safe upload limit; no GPU was submitted')
 # For a new notebook Kaggle derives its canonical slug from the title.
 write(dest/'kernel-metadata.json',{'id':handle,'title':slug,'code_file':'inference.ipynb','language':'python','kernel_type':'notebook','is_private':True,'enable_gpu':True,'enable_internet':True,'dataset_sources':[],'competition_sources':[],'kernel_sources':[],'model_sources':[]})
 manifest={'schema':1,'studio_version':VERSION,'kernel':handle,'timeout':timeout,'accelerator':'NvidiaTeslaT4','spec':portable,'runtime_sha256':runtime_sha,'requires_connection':not connection_state()['kaggle_configured'],'private':True,'upload_note':'Submitting uploads this prompt to a private notebook and stores any reference as a private Kaggle input dataset. Quota is controlled by Kaggle. Private inputs remain on Kaggle until you remove them.'}
 write(dest/'manifest.json',manifest)
 with zipfile.ZipFile(folder/'motion-source.zip','w',zipfile.ZIP_DEFLATED) as z:
  for path in sorted(dest.iterdir()):z.write(path,path.name)
 return {'source_available':True,'kernel':handle,'private':True,'model':s['model']['id'],'requires_connection':manifest['requires_connection'],'package_bytes':(folder/'motion-source.zip').stat().st_size,'note':manifest['upload_note']}
def preflight(v):
 """Check weight access and byte budget locally before reserving any Kaggle GPU."""
 s=spec(v);model=s['model']
 if model.get('provider')=='civitai':size=model.get('weight_bytes',0)
 else:
  token=settings().get('hf_token') or os.environ.get('HF_TOKEN')
  meta=request_json('https://huggingface.co/api/models/'+model['id']+'/revision/'+model['revision']+'?blobs=true',token)
  files=[SimpleNamespace(rfilename=f['rfilename'],size=f.get('size') or (f.get('lfs') or {}).get('size')) for f in meta.get('siblings',[])]
  selected=select_files(files,s['precision'])
  if any(f.rfilename.endswith('.safetensors') and not f.size for f in selected):raise ValueError('The provider did not report all weight sizes. Check the model before submitting a GPU run')
  size=sum(f.size or 0 for f in selected)
 if not size:raise ValueError('Weight size could not be verified; no GPU notebook was submitted')
 gib=size/1024**3
 if gib>s['download_limit_gb']:raise ValueError(f'Model weights require {gib:.1f} GiB, above the {s["download_limit_gb"]} GiB download limit. Increase Download limit to at least {int(gib)+1} GiB or choose another model. No GPU notebook was submitted')
 return {'weight_bytes':size,'download_gib':round(gib,2),'checked_before_gpu':True}
def reference_dataset(s,folder):
 """Keep lossless input bytes outside Kaggle's one-megabyte notebook source limit."""
 items=reference_items(s);hashes=[h for _,h in items];identity=hashes[0] if len(hashes)==1 else reference_fingerprint(s)
 username=connection_state()['kaggle_username'];key=hashlib.sha256((username+'|'+identity).encode()).hexdigest()[:24]
 slug='canvas-studio-ref-'+key;handle=username+'/'+slug;cache=root()/'reference-datasets'/(key+'.json')
 folder=Path(folder);upload=folder/'reference-upload';upload.mkdir(exist_ok=True)
 metadata=folder/'reference-metadata';metadata.mkdir(exist_ok=True)
 # An early record makes a timed-out upload discoverable without another dataset.
 known=cache.exists()
 if known:
  saved=json.loads(cache.read_text())
  if saved.get('id')!=handle or saved.get('sha256')!=identity:raise ValueError('Private reference cache identity mismatch')
 else:
  for index,(path,digest) in enumerate(items):shutil.copyfile(path,upload/('reference.png' if len(items)==1 else 'reference-'+str(index+1).zfill(2)+'.png'))
  write(upload/'dataset-metadata.json',{'id':handle,'title':slug,'licenses':[{'name':'other'}],'description':'Private Canvas Studio generation input. No public redistribution licence is granted. All rights remain with the respective rights holders.','isPrivate':True})
  write(cache,{'id':handle,'sha256':identity,'reference_hashes':hashes,'state':'uploading'})
  try:response=cli(['datasets','create','-p',str(upload),'--keep-tabular'],120)
  except subprocess.TimeoutExpired:raise ValueError('Private reference upload is unconfirmed. Retry to check this same input; no GPU notebook was submitted') from None
  except Exception:
   cache.unlink(missing_ok=True);raise
  if 'Your private Dataset is being created.' not in response or 'https://www.kaggle.com/datasets/'+handle not in response:raise ValueError('Kaggle did not confirm a private reference dataset. No GPU notebook was submitted')
 deadline=time.monotonic()+120
 while True:
  status=json.loads(cli(['datasets','status',handle,'--format','json'],30))
  state=status.get('status','').lower()
  if state=='ready':break
  if state in ('error','failed','deleted') or time.monotonic()>deadline:raise ValueError('Private reference input is not ready ('+state+'). Retry to check it before submitting; no GPU notebook was submitted')
  time.sleep(2)
 if status.get('current_version_number')!=1:raise ValueError('Private reference dataset was changed; use a new input rather than a different dataset version')
 cli(['datasets','metadata',handle,'-p',str(metadata)],30)
 info=json.loads((metadata/'dataset-metadata.json').read_text());info=info.get('info',info)
 if info.get('isPrivate') is not True:raise ValueError('Reference dataset is not private. No GPU notebook was submitted')
 result={'id':handle,'version':1,'private':True,'sha256':identity,'reference_hashes':hashes,'state':'ready'};write(cache,result);return result

def submit(v,folder):
 state=connection_state()
 if not state['kaggle_configured'] or not state['kaggle_username']:raise ValueError('Connect your Kaggle username and API token in Connections before submitting')
 estimate=preflight(v)
 result=prepare(v,folder);folder=Path(folder);meta=json.loads((folder/'inference/kernel-metadata.json').read_text());assert meta['is_private'] is True
 reference=None
 if reference_items(spec(v)):
  reference=reference_dataset(spec(v),folder);meta['dataset_sources']=[reference['id']+'/'+str(reference['version'])];write(folder/'inference/kernel-metadata.json',meta)
  manifest=json.loads((folder/'inference/manifest.json').read_text());manifest['reference_dataset']=reference;write(folder/'inference/manifest.json',manifest)
  with zipfile.ZipFile(folder/'motion-source.zip','w',zipfile.ZIP_DEFLATED) as z:
   for path in sorted((folder/'inference').iterdir()):z.write(path,path.name)
  result['package_bytes']=(folder/'motion-source.zip').stat().st_size
 requested=result['kernel'];run_id='run-'+hashlib.sha256(requested.encode()).hexdigest()[:24]
 record={'kernel':requested,'requested_kernel':requested,'model':spec(v)['model'],'request':v,'reference_sha256':spec(v).get('reference_sha256'),'reference_hashes':[h for _,h in reference_items(spec(v))],'reference_dataset':reference,'created_at':time.time(),'document_id':v.get('document_id'),'timeout':v.get('timeout',1800)}
 try:response=cli(['kernels','push','-p',str(folder/'inference'),'--timeout',str(v.get('timeout',1800)),'--accelerator','NvidiaTeslaT4'],120)
 except subprocess.TimeoutExpired:
  record['submission']='unconfirmed';write(root()/'runs'/(run_id+'.json'),record)
  return {**result,'submitted':False,'run_id':run_id,'url':'https://www.kaggle.com/code/'+requested,'note':'Kaggle did not confirm the submission before the request timed out. Check this GPU run before retrying; it may already be running.'}
 success=re.search(r'Kernel version(?:\s+(\d+))?\s+successfully pushed',response,re.I)
 links=re.findall(r'https://www\.kaggle\.com/code/([A-Za-z0-9_-]+/[A-Za-z0-9_-]+)',response)
 if not success or not links:raise ValueError('Kaggle did not confirm a running notebook submission. Check your account before retrying: '+redact(response))
 handle=links[-1]
 if handle.split('/')[0].lower()!=state['kaggle_username'].lower():raise ValueError('Kaggle returned a notebook owned by a different account; check your username and token')
 record.update(kernel=handle,version=int(success[1]) if success[1] else None,submission='confirmed');write(root()/'runs'/(run_id+'.json'),record)
 if handle!=requested:
  manifest=json.loads((folder/'inference/manifest.json').read_text());manifest['requested_kernel']=requested;manifest['kernel']=handle;write(folder/'inference/manifest.json',manifest)
  meta['id']=handle;meta['title']=handle.split('/')[1];write(folder/'inference/kernel-metadata.json',meta)
  with zipfile.ZipFile(folder/'motion-source.zip','w',zipfile.ZIP_DEFLATED) as z:
   for path in sorted((folder/'inference').iterdir()):z.write(path,path.name)
  result['package_bytes']=(folder/'motion-source.zip').stat().st_size
 return {**result,'submitted':True,'kernel':handle,'run_id':run_id,'version':record['version'],'preflight':estimate,'url':'https://www.kaggle.com/code/'+handle,'response':response,'note':'Finite private notebook submitted. Poll status, then download its output. A local Cancel button does not stop an already submitted remote notebook; use Kaggle controls.'}
def run_record(id,document):
 if not re.fullmatch('run-[a-f0-9]{24}',id):raise ValueError('Invalid inference identity')
 p=root()/'runs'/(id+'.json');data=json.loads(p.read_text())
 if data.get('document_id')!=document:raise ValueError('This inference belongs to another document')
 return data
def versioned_kernel(record):
 version=record.get('version')
 return record['kernel']+('/'+str(version) if type(version) is int and version>0 else '')
def remote_status(v):
 record=run_record(v['run_id'],v.get('document_id'));response=cli(['kernels','status',versioned_kernel(record)]);lower=response.lower();state='complete' if 'complete' in lower else 'error' if any(w in lower for w in ['error','failed','cancel']) else 'running' if 'running' in lower else 'queued'
 result={'run_id':v['run_id'],'kernel':record['kernel'],'model':record['model']['id'],'remote_state':state,'response':response,'url':'https://www.kaggle.com/code/'+record['kernel']}
 if state=='error':
  try:result['log_tail']=format_log_tail(cli(['kernels','logs',versioned_kernel(record)],30,None))
  except Exception as error:result['log_tail']='Execution log unavailable: '+redact(str(error))
 return result
def format_log_tail(raw):
 try:
  entries=json.loads(raw)
  text=''.join(str(e.get('data','')) for e in entries) if isinstance(entries,list) else raw
 except (ValueError,TypeError):text=raw
 return redact(text,None)[-4000:]
def remote_download(v,folder):
 record=run_record(v['run_id'],v.get('document_id'));state=remote_status(v)
 if state['remote_state']!='complete':raise ValueError('Kaggle inference is not complete yet')
 folder=Path(folder);dest=folder/'download';dest.mkdir(exist_ok=True)
 cli(['kernels','output',versioned_kernel(record),'-p',str(dest),'--file-pattern',r'^(output\.(png|mp4)|receipt\.json)$'],180)
 task=record['model']['task'];source=dest/('output.png' if task=='image' else 'output.mp4');receipt=json.loads((dest/'receipt.json').read_text())
 if not source.is_file() or hashlib.sha256(source.read_bytes()).hexdigest()!=receipt['output_sha256']:raise ValueError('Downloaded output is missing or does not match its receipt')
 if receipt.get('model',{}).get('id')!=record['model']['id'] or receipt.get('model',{}).get('revision')!=record['model']['revision']:raise ValueError('Downloaded receipt belongs to a different model/revision; nothing was imported')
 if record.get('reference_hashes'):
  hashes=record['reference_hashes'];fingerprint=hashlib.sha256(json.dumps(hashes,separators=(',',':')).encode()).hexdigest()
  if not receipt.get('reference_used') or [i.get('sha256') for i in receipt.get('references',[])]!=hashes or receipt.get('reference_set_sha256')!=fingerprint:raise ValueError('Downloaded result did not use the complete reference set; nothing was imported')
 if record.get('reference_sha256') and (not receipt.get('reference_used') or (receipt.get('reference') or {}).get('sha256')!=record['reference_sha256']):raise ValueError('Downloaded result used a different reference image; nothing was imported')
 shutil.copyfile(source,folder/('frame.png' if task=='image' else 'movie.mp4'));write(folder/'receipt.json',receipt)
 return {'task':task,'model':record['model']['id'],'kernel':record['kernel'],'run_id':v['run_id'],'receipt':receipt,'downloaded':True}
def creation_defaults(v):
 """One-click requests use reviewed presets; explicit advanced choices win."""
 model=next((m for m in catalog()['models'] if m['id']==v.get('model_id')),None)
 if not model:raise ValueError('Choose a model')
 return {**model.get('easy',{}),**v}
def create(v,folder,progress=lambda value:None):
 """Submit once, monitor in the worker, then deliver the verified local asset."""
 folder=Path(folder);v=creation_defaults(v) if not v.get('run_id') else dict(v)
 run_id=v.get('run_id');document=v.get('document_id');started=time.monotonic()
 def update(phase,**extra):
  progress({'phase':phase,'run_id':run_id,'automatic':True,**extra})
 if not run_id:
  update('Getting ready')
  try:submitted=submit(v,folder)
  except Exception:
   # A timed-out push may already exist. Preserve its identity, never resubmit it.
   manifest=folder/'inference/manifest.json'
   if manifest.exists():
    kernel=json.loads(manifest.read_text())['kernel'];candidate='run-'+hashlib.sha256(kernel.encode()).hexdigest()[:24]
    if (root()/'runs'/(candidate+'.json')).exists():run_id=candidate;update('Check connection',needs_attention=True)
   raise
  run_id=submitted['run_id']
 record=run_record(run_id,document)
 limit=min(7200,max(300,record.get('timeout',1800)+600));failures=0
 while time.monotonic()-started<limit:
  try:state=remote_status({'run_id':run_id,'document_id':document});failures=0
  except Exception as error:
   failures+=1;update('Reconnecting to Kaggle',needs_attention=failures>=5)
   if failures>=5:raise ValueError('Connection interrupted. Use Continue to check the same generation; it will not submit another GPU run.') from error
   time.sleep(min(30,failures*5));continue
  remote=state['remote_state'];update({'queued':'Waiting for a GPU','running':'Generating on Kaggle','complete':'Saving your creation','error':'Generation stopped'}.get(remote,'Getting ready'),remote_state=remote)
  if remote=='complete':
   result=remote_download({'run_id':run_id,'document_id':document},folder);update('Ready',remote_state='complete')
   return {**result,'automatic':True,'source_available':(folder/'motion-source.zip').is_file(),'url':'https://www.kaggle.com/code/'+record['kernel']}
  if remote=='error':raise ValueError('Kaggle could not finish this generation. '+state.get('log_tail','Open generation details for the provider response.'))
  time.sleep(10)
 update('Still running on Kaggle',needs_attention=True)
 raise ValueError('Kaggle is still working. Use Continue to check this same generation; no new GPU run is needed.')

def execute(v,folder,progress=lambda value:None):
 action=v.get('operation');folder=Path(folder)
 if action=='create':return create(v,folder,progress)
 if action=='connections':return connection_state()
 if action=='save_connections':return save_settings(v['settings'])
 if action=='search':return search(v)
 if action=='register':return register(v)
 if action=='prepare':return prepare(v,folder)
 if action=='submit':return submit(v,folder)
 if action=='remote_status':return remote_status(v)
 if action=='download':return remote_download(v,folder)
 if action=='generate':
  s=spec(v);env_vars=env()
  for k in ('HF_TOKEN','CIVITAI_API_KEY'):
   if env_vars.get(k):os.environ[k]=env_vars[k]
  output,receipt=infer(s,folder,connection_state()['cache_dir'],progress);shutil.copyfile(output,folder/('frame.png' if s['model']['task']=='image' else 'movie.mp4'));return {'task':s['model']['task'],'model':s['model']['id'],'receipt':receipt}
 raise ValueError('Unknown generation operation')
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('--request',type=Path,required=True);p.add_argument('--result',type=Path,required=True);args=p.parse_args()
 try:write(args.result,{'ok':True,'result':execute(json.loads(args.request.read_text()),args.result.parent,lambda value:write(args.result.parent/'workflow.json',value if isinstance(value,dict) else {'phase':str(value)}))})
 except Exception as error:write(args.result,{'ok':False,'error':redact(str(error))})
