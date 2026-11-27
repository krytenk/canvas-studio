"""Finite, reproducible Diffusers inference. Used locally and inside private Kaggle jobs.
No remote Python, pickle weights, safety-checker removal or arbitrary shell commands.
"""
import hashlib,json,os,re,shutil,time
from pathlib import Path
SUPPORTED={'StableDiffusionXLPipeline','StableDiffusionPipeline','Flux2KleinPipeline','Krea2Pipeline','FluxPipeline','ZImagePipeline','QwenImagePipeline','QwenImage21Pipeline','WanPipeline','WanImageToVideoPipeline','LTX2Pipeline','LTXPipeline','LTXImageToVideoPipeline'}
REPO=re.compile(r'^[A-Za-z0-9][A-Za-z0-9_.-]{0,95}/[A-Za-z0-9][A-Za-z0-9_.-]{0,95}$')
def validate(s):
 m=s['model']
 if not REPO.fullmatch(m['id']) or not re.fullmatch('[a-f0-9]{64}' if m.get('provider')=='civitai' else '[a-f0-9]{40}',m['revision']):raise ValueError('Use a verified model and pinned revision')
 if m.get('pipeline') not in SUPPORTED or not m.get('runnable'):raise ValueError('This model needs a dedicated recipe; browse its model card rather than executing custom code')
 if s.get('use','commercial')=='commercial' and m['commercial']!='commercial' and not s.get('license_acknowledged'):raise ValueError('Review the model licence and confirm your rights, or choose a commercial model')
 if m['commercial']=='research' and s.get('use','commercial')=='commercial':raise ValueError('This research licence does not cover advertising; choose research use or another model')
 if not isinstance(s.get('prompt'),str) or not 1<=len(s['prompt'].strip())<=8000:raise ValueError('Write a prompt up to 8000 characters')
 if len(s.get('negative_prompt',''))>8000:raise ValueError('Negative prompt is too long')
 for k in ('width','height'):
  if type(s.get(k)) is not int or not 256<=s[k]<=1536 or s[k]%64:raise ValueError('Use dimensions from 256–1536 in multiples of 64')
 if type(s.get('steps')) is not int or not 1<=s['steps']<=60:raise ValueError('Use 1–60 inference steps')
 if type(s.get('seed')) is not int or not 0<=s['seed']<2**32:raise ValueError('Seed must be an integer from 0 to 4294967295')
 if not 0<=float(s.get('guidance',4))<=12:raise ValueError('Guidance must be between 0 and 12')
 if s.get('precision') not in ('fp16','bf16','fp32'):raise ValueError('Choose FP16, BF16 or FP32')
 if s.get('content_mode') not in ('standard','flexible'):raise ValueError('Choose standard or flexible content mode')
 if s.get('quantization','none') not in ('none','nf4'):raise ValueError('Choose full precision or NF4 quantization')
 if s.get('quantization')=='nf4' and (m['pipeline'] in ('StableDiffusionXLPipeline','StableDiffusionPipeline') or m.get('provider')=='civitai'):raise ValueError('NF4 is available for transformer-based Diffusers recipes, not full SDXL/SD1.5 checkpoints')
 if s.get('device','cuda') not in ('cuda','cpu'):raise ValueError('Choose CPU or CUDA')
 if s['model']['task']=='video':
  if type(s.get('frames')) is not int or not 9<=s['frames']<=121 or s['frames']%4!=1:raise ValueError('Use 9–121 video frames, in the form 4n+1')
  if s.get('fps') not in (8,16,24):raise ValueError('Choose 8, 16 or 24 fps')
 if not 2<=float(s.get('download_limit_gb',20))<=100:raise ValueError('Set a 2–100 GB weight-download limit')
 for a in s.get('adapters',[]):
  if a.get('family')!=m['pipeline'] or not REPO.fullmatch(a.get('repository','')) or not re.fullmatch('[a-f0-9]{40}',a.get('revision','')) or not a.get('weight_name','').endswith('.safetensors') or '/' in a['weight_name']:raise ValueError('Choose a compatible pinned safetensors adapter')
 if s.get('reference') and m['pipeline'] not in ('Flux2KleinPipeline','QwenImage21Pipeline','WanImageToVideoPipeline','LTXImageToVideoPipeline'):raise ValueError('This recipe is text-only; choose an image-editing/image-to-video recipe for references')
 return s

def select_files(siblings,precision):
 """Select one tensor variant per component; never download duplicate full checkpoints or .bin/.py files."""
 files=[];groups={}
 for f in siblings:
  name=f.rfilename
  if name.endswith('.py') or '..' in name.split('/'):continue
  if name.endswith('.safetensors') and '/' in name:
   # Variant keys remain distinct from sharded sequence numbers.
   key=re.sub(r'\.(fp16|bf16|fp32)\.safetensors$','.safetensors',name)
   groups.setdefault(key,[]).append(f)
  elif name=='model_index.json' or ('/' in name and name.endswith(('.json','.txt','.model','.tiktoken','.jinja'))):files.append(f)
 for choices in groups.values():
  preferred=next((f for f in choices if '.'+precision+'.safetensors' in f.rfilename),None)
  generic=next((f for f in choices if not re.search(r'\.(fp16|bf16|fp32)\.safetensors$',f.rfilename)),None)
  if preferred or generic:files.append(preferred or generic)
 if not any(f.rfilename.endswith('.safetensors') for f in files):raise ValueError('No compatible safetensors pipeline weights found')
 return files

def download_civitai(model,cache,limit,progress):
 import urllib.request,urllib.parse
 class Redirect(urllib.request.HTTPRedirectHandler):
  def redirect_request(self,req,fp,code,msg,headers,newurl):
   if urllib.parse.urlsplit(newurl).scheme!='https':raise ValueError('Refusing a non-HTTPS weight redirect')
   redirected=super().redirect_request(req,fp,code,msg,headers,newurl)
   if urllib.parse.urlsplit(req.full_url).netloc!=urllib.parse.urlsplit(newurl).netloc:redirected.remove_header('Authorization')
   return redirected
 expected=model['revision'];dest=Path(cache)/(expected+'.safetensors')
 if dest.is_file():
  h=hashlib.sha256()
  with dest.open('rb') as f:
   for block in iter(lambda:f.read(4*1024**2),b''):h.update(block)
  if h.hexdigest()==expected:return dest
  raise ValueError('Cached checkpoint failed its SHA-256 check')
 size=model.get('weight_bytes',0)
 if not 0<size<=limit*1024**3:raise ValueError('Checkpoint exceeds the download limit or has no verified size')
 if shutil.disk_usage(cache).free<size+3*1024**3:raise ValueError('Insufficient free cache space for this checkpoint')
 version=model['civitai_version']
 if type(version) is not int or version<=0:raise ValueError('Invalid Civitai version')
 headers={'User-Agent':'CanvasStudio/1.16'}
 if os.environ.get('CIVITAI_API_KEY'):headers['Authorization']='Bearer '+os.environ['CIVITAI_API_KEY']
 query={'type':'Model','format':'SafeTensor'}
 if model.get('weight_precision') in ('fp16','fp32','bf16'):query['fp']=model['weight_precision']
 if model.get('weight_size') in ('full','pruned'):query['size']=model['weight_size']
 request=urllib.request.Request('https://civitai.com/api/download/models/'+str(version)+'?'+urllib.parse.urlencode(query),headers=headers)
 temporary=dest.with_suffix('.'+str(os.getpid())+'.partial');total=0;digest=hashlib.sha256();progress('Downloading checksum-pinned safetensors checkpoint')
 try:
  with urllib.request.build_opener(Redirect()).open(request,timeout=60) as response,temporary.open('wb') as f:
   while True:
    chunk=response.read(4*1024**2)
    if not chunk:break
    total+=len(chunk)
    if total>min(limit*1024**3,size*1.05):raise ValueError('Weight download exceeded the expected size')
    
    if shutil.disk_usage(cache).free<3*1024**3:raise ValueError('Cache has less than 3 GB free; weight download stopped')
    digest.update(chunk);f.write(chunk)
  if digest.hexdigest()!=expected:raise ValueError('Downloaded checkpoint did not match the publisher SHA-256')
  temporary.replace(dest);return dest
 finally:temporary.unlink(missing_ok=True)

def run(s,output_dir,cache_dir,progress=lambda v:None):
 validate(s);import torch
 from huggingface_hub import HfApi,snapshot_download
 import diffusers
 output=Path(output_dir);output.mkdir(parents=True,exist_ok=True);cache=Path(cache_dir);cache.mkdir(parents=True,exist_ok=True)
 cuda=s.get('device','cuda')=='cuda'
 if cuda and not torch.cuda.is_available():raise ValueError('No CUDA GPU is available here. Choose Kaggle, connect a GPU machine, or use CPU explicitly')
 dtype={'fp16':torch.float16,'bf16':torch.bfloat16,'fp32':torch.float32}[s['precision']]
 if not cuda and dtype!=torch.float32:raise ValueError('CPU inference requires FP32')
 if cuda and dtype==torch.bfloat16 and not torch.cuda.is_bf16_supported():raise ValueError('This GPU lacks native BF16 support. Choose FP16 or another model; two T4s do not combine their VRAM')
 model=s['model'];token=os.environ.get('HF_TOKEN') or None;cls=getattr(diffusers,model['pipeline'],None)
 if cls is None:raise ValueError('The installed Diffusers version does not yet support this pipeline')
 if model.get('provider')=='civitai':
  checkpoint=download_civitai(model,cache,float(s.get('download_limit_gb',20)),progress)
  info=HfApi(token=token).model_info(model['configuration_repo'],revision=model['configuration_revision'],files_metadata=True)
  configs=[f.rfilename for f in info.siblings if f.rfilename.endswith(('.json','.txt','.model','.tiktoken','.jinja')) and not f.rfilename.endswith('.safetensors.index.json')]
  snapshot=snapshot_download(model['configuration_repo'],revision=model['configuration_revision'],allow_patterns=configs,cache_dir=str(cache),token=token)
  progress('Loading full safetensors checkpoint');pipe=cls.from_single_file(str(checkpoint),config=snapshot,torch_dtype=dtype,local_files_only=True,use_safetensors=True)
 else:
  info=HfApi(token=token).model_info(model['id'],revision=model['revision'],files_metadata=True);selected=select_files(info.siblings,s['precision']);size=sum(f.size or 0 for f in selected)
  if size>float(s.get('download_limit_gb',20))*1024**3:raise ValueError(f'Weights need about {size/1024**3:.1f} GB, above this job\'s download limit')
  if shutil.disk_usage(cache).free<size+3*1024**3:raise ValueError('Not enough free cache space for these weights plus a 3 GB reserve')
  progress('Downloading pinned pipeline weights');snapshot=snapshot_download(model['id'],revision=model['revision'],allow_patterns=[f.rfilename for f in selected],cache_dir=str(cache),token=token)
  progress('Loading '+model['name']);kwargs=dict(torch_dtype=dtype,local_files_only=True,use_safetensors=True)
  if any('.'+s['precision']+'.safetensors' in f.rfilename for f in selected):kwargs['variant']=s['precision']
  if s.get('quantization')=='nf4':
   if not cuda:raise ValueError('This NF4 recipe requires CUDA')
   from diffusers.quantizers import PipelineQuantizationConfig
   kwargs['quantization_config']=PipelineQuantizationConfig(quant_backend='bitsandbytes_4bit',quant_kwargs={'load_in_4bit':True,'bnb_4bit_quant_type':'nf4','bnb_4bit_compute_dtype':dtype},components_to_quantize=['transformer'])
  pipe=cls.from_pretrained(snapshot,**kwargs)
 for adapter in s.get('adapters',[]):
  pipe.load_lora_weights(adapter['repository'],revision=adapter['revision'],weight_name=adapter['weight_name'],use_safetensors=True)
  if adapter['id']=='sdxl-lightning-4':
   from diffusers import EulerDiscreteScheduler
   pipe.scheduler=EulerDiscreteScheduler.from_config(pipe.scheduler.config,timestep_spacing='trailing')
 if hasattr(pipe,'enable_vae_slicing'):pipe.enable_vae_slicing()
 if hasattr(pipe,'vae') and hasattr(pipe.vae,'enable_tiling'):pipe.vae.enable_tiling()
 if cuda and s.get('offload',True):pipe.enable_model_cpu_offload()
 else:pipe.to('cuda' if cuda else 'cpu')
 options=dict(prompt=s['prompt'],width=s['width'],height=s['height'],num_inference_steps=s['steps'],guidance_scale=float(s['guidance']),generator=torch.Generator(device='cpu').manual_seed(s['seed']))
 import inspect
 params=inspect.signature(pipe.__call__).parameters
 if s.get('negative_prompt') and 'negative_prompt' in params:options['negative_prompt']=s['negative_prompt']
 if s.get('reference'):
  from PIL import Image
  options['image']=Image.open(s['reference']).convert('RGB')
 if model['task']=='video':options['num_frames']=s['frames']
 progress('Generating '+model['task']);began=time.monotonic();result=pipe(**options)
 content_check='not added; model defaults preserved'
 samples=[result.images[0]] if model['task']=='image' else list(result.frames[0])
 if s['content_mode']=='standard':
  from transformers import pipeline
  progress('Running the selected output check')
  checker=pipeline('image-classification',model='Falconsai/nsfw_image_detection',revision='96cb0d0342c7afb80cab76ecc58b265fa44da256',device=-1,trust_remote_code=False)
  nsfw=0.
  for sample in samples:
   from PIL import Image
   if not isinstance(sample,Image.Image):
    import numpy as np
    array=np.asarray(sample);array=(array*255).clip(0,255).astype('uint8') if array.dtype.kind=='f' else array.astype('uint8');sample=Image.fromarray(array)
   scores=checker(sample);nsfw=max(nsfw,max((a['score'] for a in scores if a['label'].lower()=='nsfw'),default=0))
   if nsfw>=.8:raise ValueError('Standard mode flagged adult content. The model\'s own safeguards were preserved; no asset was imported')
  content_check={'checker':'Falconsai/nsfw_image_detection','max_nsfw_score':nsfw,'threshold':.8,'frames_checked':len(samples)}
 if model['task']=='image':target=output/'output.png';samples[0].save(target)
 else:
  from diffusers.utils import export_to_video
  target=output/'output.mp4';export_to_video(samples,str(target),fps=s['fps'])
 receipt={'schema':1,'model':model,'settings':{k:v for k,v in s.items() if k not in ('model','reference')},'reference_used':bool(s.get('reference')),'elapsed_seconds':round(time.monotonic()-began,2),'device':torch.cuda.get_device_name(0) if cuda else 'CPU','content_check':content_check,'diffusers_version':diffusers.__version__,'torch_version':torch.__version__,'output_sha256':hashlib.sha256(target.read_bytes()).hexdigest()}
 (output/'receipt.json').write_text(json.dumps(receipt,indent=2));return target,receipt
