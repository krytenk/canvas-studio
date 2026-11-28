"""Cancellable subprocess bridge used by the shared media job manager."""
import json,os,signal,subprocess,time
from pathlib import Path
def run(w):
 from studio_media import Cancelled,atomic
 if w.cancel.exists():raise Cancelled()
 folder=w.folder;request=folder/'ai-request.json';request.write_text(json.dumps(w.spec['request']));request.chmod(0o600)
 result=folder/'ai-result.json';log=folder/'ai-worker.log'
 with log.open('wb') as output:
  p=subprocess.Popen([w.spec['python'],str(Path(__file__).with_name('studio_ai.py')),'--request',str(request),'--result',str(result)],stdin=subprocess.DEVNULL,stdout=output,stderr=subprocess.STDOUT,start_new_session=True);start=time.monotonic()
  try:
   while p.poll() is None:
    if w.cancel.exists():raise Cancelled()
    if time.monotonic()-start>7200:raise ValueError('Generation exceeded the local two-hour limit; project preserved')
    workflow={}
    try:workflow=json.loads((folder/'workflow.json').read_text())
    except (OSError,ValueError):pass
    atomic(w.progress,dict(workflow,state='running',phase=workflow.get('phase','Getting ready'),progress=0,elapsed_seconds=round(time.monotonic()-start),worker_pid=os.getpid()));time.sleep(.25)
   if p.returncode or not result.exists():raise ValueError('Generation worker stopped; see local tools setup and available GPU')
   data=json.loads(result.read_text())
   if not data.get('ok'):raise ValueError(data.get('error','Generation failed'))
   return data['result']
  finally:
   if p.poll() is None:
    os.killpg(p.pid,signal.SIGTERM)
    try:p.wait(timeout=5)
    except subprocess.TimeoutExpired:os.killpg(p.pid,signal.SIGKILL);p.wait()
