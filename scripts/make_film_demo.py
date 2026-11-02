"""Build a 30-second editing demonstration through the public MCP adapter.
Downloads only the official CC BY 3.0 Sintel trailer; see docs/FILM-STUDIO.md.
No user/customer media, paid generation, remote rendering or public posting.
"""
import argparse,json,subprocess,time,urllib.request
from pathlib import Path
from mcp_smoke import Client
from media_smoke import launch,wait,probe
ROOT=Path(__file__).resolve().parents[1]
def run(binary):
    qa=ROOT/'qa/film-demo';qa.mkdir(parents=True,exist_ok=True)
    src=qa/'sintel-trailer.mp4';url='https://download.blender.org/durian/trailer/sintel_trailer-720p.mp4'
    if not src.exists():urllib.request.urlretrieve(url,src)
    server,log,connection,_=launch(binary,qa,'engine-data');c=Client(qa,connection)
    try:
        def edit(cmd):return c.call('studio_edit',{**c.guard(),'commands':cmd})
        c.call('studio_create',{**c.guard(),'width':1280,'height':720})
        edit([{'action':'title','value':'Make the next frame count · 30-second editing demo'},
          {'action':'background','page':0,'color':'#121c26'},
          {'action':'add_shape','page':0,'shape':'Circle','x':980,'y':50,'width':230,'height':230,'color':'#92dfbd'},
          {'action':'add_text','page':0,'name':'Film label','content':'CANVAS STUDIO / FILM WORKSPACE','size':15,'tracking':2,'color':'#92dfbd','x':80,'y':70,'width':800,'height':55},
          {'action':'add_text','page':0,'name':'Call to action','content':'Make the next\nframe count.','family':'DisplaySerif','size':66,'color':'#f4f0e5','x':80,'y':205,'width':1100,'height':255},
          {'action':'add_shape','page':0,'shape':'Rectangle','x':80,'y':525,'width':650,'height':65,'color':'#92dfbd'},
          {'action':'add_text','page':0,'content':'START YOUR NEXT FILM  →','bold':True,'size':19,'color':'#121c26','x':105,'y':541,'width':600,'height':42},
          {'action':'add_text','page':0,'name':'Source attribution','content':'Footage & original sound: © copyright Blender Foundation | durian.blender.org\nSintel trailer · CC BY 3.0 · Edited for this demonstration; no endorsement.','size':10,'color':'#aec1c9','x':80,'y':632,'width':1120,'height':64},
          {'action':'film_setup','width':1280,'height':720,'fps':30,'duration':900}])
        asset=c.call('studio_import',{**c.guard(),'kind':'media','path':src.name})['asset']
        job=c.call('studio_media_render',{**c.guard(),'mode':'proxy','asset':asset['id']});wait(c,job)
        for start,duration,source in [(0,180,0),(180,240,11000000),(420,360,23000000)]:
            edit([{'action':'film_add','asset':asset['id'],'start':start,'duration':duration,'source_in_us':source}])
            clip=c.call('studio_document')['timeline']['clips'][-1]
            edit([{'action':'film_clip','id':clip['id'],'name':{0:'The journey',180:'The question',420:'The discovery'}[start],'fade_in':3,'fade_out':4,'gain_db':-1}])
        edit([{'action':'film_track','kind':'audio','name':'Original score · closing tail'}]);track=c.call('studio_document')['timeline']['tracks'][-1]['id']
        edit([{'action':'film_add','asset':asset['id'],'track':track,'start':780,'duration':120,'source_in_us':35000000}]);clip=c.call('studio_document')['timeline']['clips'][-1]
        edit([{'action':'film_clip','id':clip['id'],'name':'Music into the invitation','gain_db':-7,'fade_in':4,'fade_out':45}])
        captions=[(18,138,'A story starts with a question.'),(195,180,'Every cut changes the feeling.'),(435,180,'Picture. Dialogue. Music.'),(630,126,'One editable timeline.')]
        for start,duration,text in captions:
            edit([{'action':'film_caption','start':start,'duration':duration,'text':text}]);clip=c.call('studio_document')['timeline']['clips'][-1]
            edit([{'action':'film_clip','id':clip['id'],'size':25,'y':530,'height':90}])
        edit([{'action':'film_design','page':0,'start':780,'duration':120,'transparent':False}])
        # A genuine split inside 24 fps footage on a 30 fps timeline, retaining its phase.
        clip=c.call('studio_document')['timeline']['clips'][2]
        edit([{'action':'film_split','id':clip['id'],'at':577}])
        def render(mode,name):
            j=c.call('studio_media_render',{**c.guard(),'mode':mode});wait(c,j,timeout=240)
            c.call('studio_media_job',{**c.guard(),'id':j['id'],'operation':'save','path':name,'overwrite':True})
            return j
        preview=render('preview','film-preview.mp4');final=render('export','film-30s.mp4')
        c.call('studio_export',{**c.guard(),'path':'film-30s.canvas','format':'canvas','overwrite':True})
        p=probe(qa/'film-30s.mp4');v=next(s for s in p['streams'] if s['codec_type']=='video');a=next(s for s in p['streams'] if s['codec_type']=='audio')
        assert int(v['nb_frames'])==900 and (v['width'],v['height'])==(1280,720) and v['r_frame_rate']=='30/1',v
        assert a['sample_rate']=='48000' and abs(float(a['duration'])-30)<.002,a
        report={'frames':900,'fps':30,'duration':30,'size':[1280,720],'audio_hz':48000,'preview_job':preview,'export_job':final,'source_revision':c.guard()['expected_revision'],'source_url':url,'license':'CC BY 3.0','attribution':'© copyright Blender Foundation | durian.blender.org','adaptation':'Trimmed, recomposed, captioned and mixed; CTA artwork by Canvas Studio. Not an endorsement.'}
        (qa/'report.json').write_text(json.dumps(report,indent=2));print(json.dumps(report,indent=2))
    finally:c.close();server.terminate();server.wait(timeout=10);log.close()
if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--binary',default='./canvas-studio');a=p.parse_args();run(Path(a.binary).resolve())
