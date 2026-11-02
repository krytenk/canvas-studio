"""Animate the licensed 1.13 film through MCP, retaining its source attribution."""
import argparse,json,os,shutil,time
from pathlib import Path
from mcp_smoke import Client
from media_smoke import launch,wait,probe
ROOT=Path(__file__).resolve().parents[1]
def run(binary):
    qa=ROOT/'qa/motion-demo';qa.mkdir(parents=True,exist_ok=True)
    source=ROOT/'StudioProjects/Film Studio 1.13 Demo/Film studio demo.canvas'
    if not source.exists():source=ROOT/'qa/film-demo/film-30s.canvas'
    if not source.exists():raise SystemExit('Run scripts/make_film_demo.py first to create the credited source example.')
    shutil.copy2(source,qa/'original.canvas');os.environ['CANVAS_STUDIO_HYPERFRAMES']=str(ROOT/'tools/hyperframes/node_modules/hyperframes/bin/hyperframes.mjs')
    server,log,connection,_=launch(binary,qa,'data-'+str(time.time_ns()),qa/'original.canvas');c=Client(qa,connection)
    try:
        def edit(commands):c.call('studio_edit',{**c.guard(),'commands':commands})
        t=c.call('studio_document')['timeline'];first=t['clips'][0]
        edit([{'action':'title','value':'Make it move · HyperFrames motion demo'},{'action':'film_clip','id':first['id'],'source_in_us':1000000},{'action':'film_motion','id':first['id'],'preset':'drift'}])
        discovery=[c for c in t['clips'] if c['name']=='The discovery'];left,right=sorted(discovery,key=lambda c:c['start'])
        edit([{'action':'film_delete','id':right['id']},{'action':'film_clip','id':left['id'],'duration':left['duration']+right['duration']},{'action':'film_motion','id':left['id'],'preset':'drift'},{'action':'film_split','id':left['id'],'at':right['start']}])
        for clip in c.call('studio_document')['timeline']['clips']:
            if clip['text'] is not None:edit([{'action':'film_motion','id':clip['id'],'preset':'rise'}])
            if clip['design'] is not None:edit([{'action':'film_motion','id':clip['id'],'preset':'punch'}])
        jobs={}
        for mode,name in [('preview','motion-preview.mp4'),('export','motion-30s.mp4')]:
            job=c.call('studio_media_render',{**c.guard(),'mode':mode});wait(c,job,timeout=600)
            c.call('studio_media_job',{**c.guard(),'id':job['id'],'operation':'save','path':name,'overwrite':True});jobs[mode]=job
        c.call('studio_media_job',{**c.guard(),'id':jobs['export']['id'],'operation':'source','path':'motion-source.zip','overwrite':True})
        c.call('studio_export',{**c.guard(),'path':'motion-30s.canvas','format':'canvas','overwrite':True})
        p=probe(qa/'motion-30s.mp4');v=next(x for x in p['streams'] if x['codec_type']=='video');assert int(v['nb_frames'])==900 and (v['width'],v['height'])==(1280,720),v
        report={'frames':900,'fps':30,'duration':30,'engine':'HyperFrames 0.8.206','native_editable_master':True,'source':'Sintel trailer, © copyright Blender Foundation | durian.blender.org, CC BY 3.0','changes':'Source cuts/score, animated native captions/design card and video transforms; no endorsement.','jobs':jobs};(qa/'report.json').write_text(json.dumps(report,indent=2));print(json.dumps(report,indent=2))
    finally:c.close();server.terminate();server.wait(timeout=10);log.close()
if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--binary',default='./canvas-studio');a=p.parse_args();run(Path(a.binary).resolve())
