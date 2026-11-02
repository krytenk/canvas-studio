"""Exercise the STDIO MCP server against an isolated real studio engine.
Python stdlib; pdftotext/pdfinfo required, Pillow used for proof dimensions.
"""
import argparse,base64,json,os,re,select,subprocess,sys,time
from pathlib import Path
from PIL import Image
ROOT=Path(__file__).resolve().parents[1]
META={"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientCapabilities":{}}
class Client:
    def __init__(self,workspace,connection):
        self.p=subprocess.Popen([sys.executable,str(ROOT/'agents/canvas_studio_mcp.py'),'--workspace',str(workspace),'--connection-file',str(connection)],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,bufsize=1)
        self.counter=0
    def rpc(self,method,params=None,modern=True):
        self.counter+=1;params=dict(params or {})
        if modern:params['_meta']=META.copy()
        request={'jsonrpc':'2.0','id':self.counter,'method':method,'params':params}
        self.p.stdin.write(json.dumps(request)+'\n');self.p.stdin.flush()
        ready,_,_=select.select([self.p.stdout],[],[],100)
        assert ready,'MCP response timed out'
        line=self.p.stdout.readline();assert line,'MCP exited: '+self.p.stderr.read()
        response=json.loads(line);assert response.get('id')==self.counter,response
        return response
    def call(self,name,a=None,error=False,modern=True):
        response=self.rpc('tools/call',{'name':name,'arguments':a or {}},modern)
        assert 'result' in response,response
        result=response['result'];assert bool(result.get('isError'))==error,result
        return result if error else result.get('structuredContent',result)
    def guard(self):
        doc=self.call('studio_document');return {'document_id':doc['document_id'],'expected_revision':doc['revision']}
    def close(self):
        self.p.stdin.close();self.p.wait(timeout=10)
        assert self.p.returncode==0,self.p.stderr.read()
        assert not self.p.stderr.read(),'Unexpected MCP stderr'

def run(binary):
    qa=ROOT/'qa/mcp';qa.mkdir(parents=True,exist_ok=True)
    workspace=qa/'workspace';workspace.mkdir(exist_ok=True)
    env=os.environ.copy();env['XDG_DATA_HOME']=str(qa/'server-data')
    log=(qa/'server.log').open('w')
    server=subprocess.Popen([str(binary),'--serve-browser','--port','0'],env=env,stdout=log,stderr=subprocess.STDOUT)
    clients=[]
    try:
        for _ in range(100):
            match=re.search(r'http://127\.0\.0\.1:(\d+)/#([0-9a-f]+)',(qa/'server.log').read_text())
            if match:break
            if server.poll() is not None:raise RuntimeError((qa/'server.log').read_text())
            time.sleep(.1)
        else:raise AssertionError('Engine did not start')
        port,token=match.groups();connection=qa/f'server-data/canvas-studio/browser/connection-{port}.json'
        assert connection.exists()
        if os.name=='posix':assert connection.stat().st_mode&0o777==0o600
        c=Client(workspace,connection);clients.append(c)
        assert c.rpc('tools/list',modern=False)['error']['code']==-32602
        assert c.rpc('server/discover')['result']['supportedVersions']==['2026-07-28']
        response=c.rpc('tools/list');assert len(response['result']['tools'])==11
        assert token not in json.dumps(response)
        assert c.rpc('tools/call',{'name':'unknown','arguments':{}})['error']['code']==-32602
        bad=c.rpc('ping',{'_meta':{}}) # rpc adds the valid per-request metadata
        assert bad['result']['resultType']=='complete'
        original=META['io.modelcontextprotocol/protocolVersion'];META['io.modelcontextprotocol/protocolVersion']='2099-01-01'
        assert c.rpc('ping')['error']['code']==-32022
        META['io.modelcontextprotocol/protocolVersion']=original
        catalog=c.call('studio_templates');assert len(catalog['templates'])==18
        before=c.guard();c.call('studio_use_template',{**before,'template':'ig_carousel','mode':'replace'})
        c.call('studio_edit',{**before,'commands':[{'action':'title','value':'Stale overwrite'}]},error=True)
        doc=c.call('studio_document');assert len(doc['pages'])==5
        before=c.guard();c.call('studio_create',{**before,'width':1080,'height':1350})
        commands=[
            {'action':'title','value':'Agent campaign proof'},
            {'action':'background','page':0,'color':'#181e24'},
            {'action':'add_shape','page':0,'ref':'disc','shape':'Circle','color':'#d5f171','x':710,'y':90,'width':270,'height':270},
            {'action':'add_text','page':0,'ref':'label','name':'Campaign label','content':'YOUR NEXT CHAPTER / 01','size':12,'color':'#d5f171','tracking':2,'x':80,'y':90,'width':600,'height':55},
            {'action':'add_text','page':0,'ref':'hero','name':'Campaign headline','content':'Make it\nworth sharing.','size':90,'color':'#f5f1e8','bold':True,'leading':1.05,'x':80,'y':470,'width':920,'height':340},
            {'action':'add_text','page':0,'ref':'body','content':'A clear idea. A considered design.\nA reason to stop scrolling.','size':22,'color':'#c5cbd0','leading':1.5,'x':80,'y':900,'width':850,'height':160},
            {'action':'add_shape','page':0,'ref':'cta','shape':'Rectangle','color':'#d5f171','x':80,'y':1150,'width':920,'height':100},
            {'action':'add_text','page':0,'content':'CREATE SOMETHING THAT MATTERS.','size':19,'bold':True,'color':'#181e24','x':110,'y':1180,'width':850,'height':55},
            {'action':'layer','id':'$hero','opacity':1}]
        g=c.guard();dry=c.call('studio_edit',{**g,'commands':commands,'dry_run':True});assert dry['dry_run'] and not dry['preflight']['issues']
        assert c.guard()==g
        failed=c.call('studio_edit',{**g,'commands':[commands[0],{'action':'geometry','id':'no-such-layer','x':0,'y':0,'width':1,'height':1}]},error=True)
        assert c.guard()==g and c.call('studio_document')['title']!='Agent campaign proof'
        applied=c.call('studio_edit',{**g,'commands':commands});assert applied['revision']==g['expected_revision']+1
        c.call('studio_history',{**c.guard(),'direction':'undo'});assert not c.call('studio_document')['elements']
        c.call('studio_history',{**c.guard(),'direction':'redo'});assert len(c.call('studio_document')['elements'])==6
        # A fresh process can use explicit handles without any initialize/context state.
        d=Client(workspace,connection);clients.append(d)
        assert d.call('studio_document')['document_id']==c.guard()['document_id']
        g=c.guard();result=c.rpc('tools/call',{'name':'studio_preview','arguments':{'document_id':g['document_id'],'edge':960}})['result']
        proof=base64.b64decode(next(i['data'] for i in result['content'] if i['type']=='image'))
        (qa/'agent-preview.png').write_bytes(proof);assert Image.open(qa/'agent-preview.png').height==960
        assert not c.call('studio_preflight',{'document_id':g['document_id']})['issues']
        assert token not in json.dumps(result)
        for suffix in ('canvas','pdf','png'):
            c.call('studio_export',{**g,'path':f'campaign.{suffix}','format':suffix,'overwrite':True})
        assert Image.open(workspace/'campaign.png').size==(1080,1350)
        text=subprocess.check_output(['pdftotext',str(workspace/'campaign.pdf'),'-'],text=True)
        assert 'worth sharing.' in text
        c.call('studio_export',{**g,'path':'campaign.png','format':'png'},error=True)
        c.call('studio_export',{**g,'path':'../escape.png','format':'png'},error=True)
        c.call('studio_open',{**g,'path':'../server.log'},error=True)
        outside=qa/'outside';outside.mkdir(exist_ok=True)
        link=workspace/'outside-link';link.unlink(missing_ok=True);link.symlink_to(outside.resolve(),target_is_directory=True)
        c.call('studio_export',{**g,'path':'outside-link/escape.png','format':'png'},error=True)
        # Import licensed local font and generated photo into editable portable delivery.
        import shutil
        shutil.copyfile(ROOT/'assets/fonts/NotoSerif-Regular.ttf',workspace/'font.ttf')
        Image.new('RGB',(80,100),(216,180,150)).save(workspace/'photo.png')
        c.call('studio_import',{**c.guard(),'kind':'font','path':'font.ttf'})
        c.call('studio_import',{**c.guard(),'kind':'photo','path':'photo.png','page':0})
        c.call('studio_export',{**c.guard(),'path':'with-assets.canvas','format':'canvas','overwrite':True})
        c.call('studio_open',{**c.guard(),'path':'with-assets.canvas'})
        loaded=c.call('studio_document');assert any(f['id'].startswith('imported:') for f in loaded['fonts'])
        assert any(e['type']=='image' for e in loaded['elements'])
        cold=subprocess.check_output([str(binary),'--verify-project',str(workspace/'with-assets.canvas')],text=True);assert 'Agent campaign proof' in cold
        wrong={**c.guard(),'document_id':'wrong-document'}
        c.call('studio_edit',{**wrong,'commands':[commands[0]]},error=True)
        c.call('studio_preview',{'document_id':'wrong-document'},error=True)
        # Old clients still use their negotiated initialization format.
        legacy=Client(workspace,connection);clients.append(legacy)
        assert legacy.rpc('initialize',{'protocolVersion':'2025-11-25','capabilities':{},'clientInfo':{'name':'legacy-check','version':'1'}},modern=False)['result']['protocolVersion']=='2025-11-25'
        assert 'resultType' not in legacy.rpc('tools/list',modern=False)['result']
        assert legacy.call('studio_document',modern=False)['title']=='Agent campaign proof'
        report={'tools':11,'templates':18,'protocols':['2026-07-28','2025-11-25'],'atomic_rollback_dry_run_undo':'passed','explicit_handles_fresh_client':'passed','preview_pdf_png_portable_cold_load':'passed','fonts_photos':'passed','stale_revision_wrong_document':'passed','workspace_traversal_symlink_and_no_overwrite':'passed','token_not_in_results':'passed'}
        (qa/'report.json').write_text(json.dumps(report,indent=2));print(json.dumps(report,indent=2))
    finally:
        for client in clients:client.close()
        server.terminate();server.wait(timeout=10);log.close()
if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('--binary',default='./canvas-studio');args=parser.parse_args();run(Path(args.binary).resolve())
