#!/usr/bin/env python3
"""Standalone STDIO MCP for the same private, finite Kaggle inference backend.
No credential tools, arbitrary commands, public notebooks, or paid providers.
"""
import argparse,hashlib,json,sys,tempfile
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];sys.path.insert(0,str(ROOT/'generation'));sys.path.insert(0,str(ROOT/'agents'))
import studio_ai
from canvas_studio_mcp import obj,enum,tool,STRING,GUARD,GENERATION,validate
PROTOCOL='2026-07-28'
NAMES={'kaggle_model_catalog':'models','kaggle_model_search':'search','kaggle_prepare_inference':'prepare','kaggle_submit_inference':'submit','kaggle_inference_status':'remote_status','kaggle_download_output':'download'}
def schema(operation):
 fields={k:v for k,v in GENERATION.items() if k not in ('operation','reference_page')};fields['document_id']=STRING
 return obj(fields,('document_id',))
TOOLS=[tool(name,name.replace('_',' ').title(),'Use the Canvas Studio reviewed model backend. Prepare is offline and embeds no credentials. Submit creates a private, finite GPU notebook and uses your Kaggle quota. Configure credentials in the app; never pass them to a tool. No arbitrary shell/code execution or paid endpoint. Results remain in the private local AI workspace.',schema(operation),readonly=operation=='models') for name,operation in NAMES.items()]
for t in TOOLS:t['annotations']['openWorldHint']=True

def call(name,args,workspace):
 if name not in NAMES:raise ValueError('Unknown Kaggle studio tool')
 validate(args,next(t['inputSchema'] for t in TOOLS if t['name']==name));operation=NAMES[name]
 if operation=='models':return {'catalog':studio_ai.catalog(),'connections':studio_ai.connection_state()}
 values={**args,'operation':operation}
 # Managed job directories have no caller-controlled path components.
 folder=studio_ai.root()/'mcp-jobs'/('mcp-'+hashlib.sha256((json.dumps(values,sort_keys=True)+str(__import__('time').time_ns())).encode()).hexdigest()[:24]);folder.mkdir(parents=True,mode=0o700)
 result=studio_ai.execute(values,folder)
 if operation=='prepare':
  package=folder/'motion-source.zip';out=workspace/(folder.name+'.zip');out.write_bytes(package.read_bytes());out.chmod(0o600);result['package']=str(out)
 if operation=='download':
  source=folder/('movie.mp4' if result['task']=='video' else 'frame.png');out=workspace/(folder.name+source.suffix);out.write_bytes(source.read_bytes());out.chmod(0o600);result['output']=str(out)
 return result

def main():
 parser=argparse.ArgumentParser();parser.add_argument('--workspace',type=Path,required=True);a=parser.parse_args();workspace=a.workspace.expanduser().resolve(strict=True)
 if not workspace.is_dir():raise SystemExit('Workspace must be an existing directory')
 import canvas_studio_mcp as protocol
 protocol.TOOLS=TOOLS
 protocol.INSTRUCTIONS='Canvas Studio Kaggle: inspect the catalog and connection state; prepare private finite notebooks, submit authorized GPU jobs, poll and download outputs. Configure credentials in the app, never in tool arguments. Respect licences, account quota and provider terms.'
 class Adapter:
  def call(self,name,args):
   try:return call(name,args,workspace),None
   except Exception as error:raise ValueError(studio_ai.redact(str(error))) from None
 protocol.Server(Adapter()).run()
if __name__=='__main__':main()
