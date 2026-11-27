"""Install free SDK tools outside the Documents/source drive. Does not download model weights."""
import argparse,os,subprocess,venv
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--local-inference',action='store_true',help='Also install CPU PyTorch/Diffusers for local testing or GPU tooling');a=p.parse_args()
base=Path(os.environ.get('XDG_DATA_HOME',Path.home()/'.local/share'))/'canvas-studio/generation-tools';base.parent.mkdir(parents=True,exist_ok=True)
if not (base/'bin/python').exists():venv.EnvBuilder(with_pip=True).create(base)
python=base/'bin/python'
subprocess.run([str(python),'-m','pip','install','--disable-pip-version-check','kaggle==2.2.5','six==1.17.0'],check=True)
if a.local_inference:
 subprocess.run([str(python),'-m','pip','install','--index-url','https://download.pytorch.org/whl/cpu','torch==2.9.1+cpu'],check=True)
 subprocess.run([str(python),'-m','pip','install','diffusers==0.43.0','transformers==5.24.1','accelerate==1.16.0','safetensors','sentencepiece','protobuf','imageio[ffmpeg]','peft==0.22.0','bitsandbytes==0.50.2'],check=True)
print('Generation tools:',base)
print('No model weights downloaded. Add your Kaggle username/API token in the studio Connections screen; keep tokens out of chat.')
