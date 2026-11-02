#!/usr/bin/env python3
"""Bounded local FFmpeg worker. Receives only app-owned job specs; never a shell."""
import argparse,hashlib,json,math,os,shutil,signal,subprocess,time
from pathlib import Path
MAX_FILE=128*1024*1024
FORMATS={'.mp4':'mov','.m4v':'mov','.mov':'mov','.m4a':'mov','.webm':'matroska','.mkv':'matroska','.wav':'wav','.mp3':'mp3','.flac':'flac','.ogg':'ogg','.png':'image2','.jpg':'image2','.jpeg':'image2','.webp':'image2'}
def atomic(path,value):
    tmp=path.with_suffix('.tmp');tmp.write_text(json.dumps(value,allow_nan=False));os.replace(tmp,path)
def digest(path):
    h=hashlib.sha256()
    with path.open('rb') as f:
        for b in iter(lambda:f.read(1024*1024),b''):h.update(b)
    return h.hexdigest()
def probe(path,ext,name):
    if ext not in FORMATS or not path.is_file() or not 0<path.stat().st_size<=MAX_FILE:raise ValueError('Use a supported media file up to 128 MiB')
    args=['ffprobe','-v','error','-protocol_whitelist','file,pipe','-f',FORMATS[ext],'-show_streams','-show_format','-of','json',str(path)]
    r=subprocess.run(args,capture_output=True,timeout=15)
    if r.returncode:raise ValueError('Media could not be decoded: '+r.stderr.decode(errors='replace')[-1600:])
    p=json.loads(r.stdout);streams=p.get('streams',[])
    visual=next((s for s in streams if s.get('codec_type')=='video' and not s.get('disposition',{}).get('attached_pic')),None)
    audio=next((s for s in streams if s.get('codec_type')=='audio'),None)
    image=ext in ('.png','.jpg','.jpeg','.webp');kind='image' if image else 'video' if visual else 'audio'
    if not visual and not audio:raise ValueError('No usable video, image or audio stream')
    duration=0 if image else float((visual or audio).get('duration') or p.get('format',{}).get('duration') or 0)
    w,h=int((visual or {}).get('width',0)),int((visual or {}).get('height',0))
    rotation=next((float(s.get('rotation',0)) for s in (visual or {}).get('side_data_list',[]) if 'rotation' in s),0)
    if round(rotation)%180:w,h=h,w
    if (kind != "audio" and (w<=0 or h<=0)) or w>8192 or h>8192 or w*h>33_554_432 or (not image and not 0<duration<=7200):raise ValueError('Media exceeds 8K/2-hour import limits, or has no usable duration')
    return dict(id=digest(path),name=name[:160],extension=ext[1:],kind=kind,bytes=path.stat().st_size,duration_us=round(duration*1e6),width=w,height=h,has_audio=bool(audio),sample_rate=int((audio or {}).get('sample_rate',0)),channels=int((audio or {}).get('channels',0)))
class Cancelled(Exception):pass
class Worker:
    def __init__(self,spec,result,cancel):self.spec=spec;self.result=result;self.cancel=cancel;self.folder=result.parent;self.progress=self.folder/'progress.json'
    def ffmpeg(self,args,duration=1):
        log=self.folder/'ffmpeg.log';progress=self.folder/'ffmpeg-progress.txt'
        progress.unlink(missing_ok=True)
        with log.open('wb') as stderr:
            p=subprocess.Popen(['ffmpeg','-nostdin','-hide_banner','-loglevel','error','-y','-threads','2','-filter_complex_threads','1','-progress',str(progress),*args],stdout=subprocess.DEVNULL,stderr=stderr,start_new_session=True)
            began=time.monotonic()
            try:
                while p.poll() is None:
                    if self.cancel.exists():raise Cancelled()
                    if time.monotonic()-began>900:raise ValueError('Media operation exceeded 15 minutes')
                    percent=0
                    if progress.exists():
                        for line in progress.read_text(errors='replace').splitlines():
                            if line.startswith('out_time_us='):
                                try:percent=min(.99,max(0,int(line.split('=')[1])/1e6/max(duration,.01)))
                                except ValueError:pass
                    atomic(self.progress,dict(state='running',phase='encoding',progress=percent,worker_pid=os.getpid()))
                    output=self.spec.get('output')
                    if output and Path(str(output)+'.partial.mp4').exists() and Path(str(output)+'.partial.mp4').stat().st_size>MAX_FILE:raise ValueError('Movie exceeds 128 MiB; shorten it or use a smaller output size')
                    time.sleep(.1)
                if p.returncode:raise ValueError('FFmpeg: '+log.read_text(errors='replace')[-2600:])
            finally:
                if p.poll() is None:
                    os.killpg(p.pid,signal.SIGTERM)
                    try:p.wait(timeout=3)
                    except subprocess.TimeoutExpired:os.killpg(p.pid,signal.SIGKILL);p.wait()
    def proxy(self):
        a=self.spec['asset'];src=Path(self.spec['source']);cache=Path(self.spec['cache']);cache.mkdir(parents=True,exist_ok=True)
        key=a['id']+'-v1';suffix='.proxy.mp4' if a['kind']=='video' else '.proxy.m4a' if a['kind']=='audio' else '.proxy.png';out=cache/(key+suffix)
        if not out.exists() or self.spec.get("rebuild",False):
            tmp=cache/(key+'.'+self.folder.name+'.partial'+out.suffix)
            args=['-protocol_whitelist','file,pipe','-f',FORMATS['.'+a['extension']],'-i',str(src)]
            if a['kind']=='video':args+=['-map','0:v:0','-map','0:a:0?','-vf','scale=720:720:force_original_aspect_ratio=decrease:force_divisible_by=2','-c:v','libx264','-preset','veryfast','-crf','26','-pix_fmt','yuv420p','-c:a','aac','-b:a','96k','-ar','48000','-movflags','+faststart','-metadata:s:v:0','rotate=0']
            elif a['kind']=='audio':args+=['-vn','-c:a','aac','-b:a','128k','-ar','48000','-ac','2']
            else:args+=['-vf','scale=960:960:force_original_aspect_ratio=decrease','-frames:v','1','-update','1']
            self.ffmpeg(args+[str(tmp)],a['duration_us']/1e6 or 1);os.replace(tmp,out)
        if a['kind']!='audio':
            poster=cache/(key+'.poster.jpg')
            if not poster.exists():self.ffmpeg(['-i',str(out),'-vf','scale=360:360:force_original_aspect_ratio=decrease','-frames:v','1','-update','1',str(poster)],1)
        if a['has_audio']:
            wave=cache/(key+'.wave.png')
            if not wave.exists():self.ffmpeg(['-i',str(out),'-filter_complex','[0:a:0]aresample=2000,aformat=channel_layouts=mono,showwavespic=s=640x80:colors=0x71c9ac','-frames:v','1','-update','1',str(wave)],a['duration_us']/1e6)
        return dict(proxy=str(out))
    def render(self):
        s=self.spec;t=s['timeline'];fps=t['fps'];duration=t['duration']/fps
        frame=s.get('frame');preview=s['mode'] in ('preview','frame');scale=min(1,640/max(t['width'],t['height'])) if preview else 1
        w=max(2,round(t['width']*scale/2)*2);h=max(2,round(t['height']*scale/2)*2)
        total=1 if frame is not None else t['duration'];seconds=total/fps
        c=t['background'];args=['-f','lavfi','-i',f'color=c=0x{c[0]:02x}{c[1]:02x}{c[2]:02x}:s={w}x{h}:r={fps}:d={seconds:.9f}']
        filters=[f'[0:v]settb=1/{fps},setpts=N,format=rgba[vbase]'];current='vbase';streams=[];index=1
        if frame is None:args+=['-f','lavfi','-i','anullsrc=r=48000:cl=stereo'];filters+=[f'[1:a]atrim=end_sample={total*48000//fps},asetpts=N/SR/TB[abase]'];streams=['abase'];index=2
        enabled={tr['id']:tr for tr in t['tracks'] if tr['enabled']}
        clips=[c for tr in t['tracks'] if tr['enabled'] for c in t['clips'] if c['track']==tr['id']]
        for sequence,original in enumerate(clips):
            c=dict(original);start=c['start'];length=c['duration'];source=c.get('source_anchor_us',c['source_in_us'])/1e6;source_offset=c.get('source_offset_frames',0)
            if frame is not None:
                if not start<=frame<start+length:continue
                source_offset+=frame-start;start=0;length=1
            asset=t['assets'].get(c.get('asset')) if c.get('asset') else None
            path=s['sources'].get(c['id'])
            if not path:raise ValueError('A clip source was not prepared')
            still=not asset or asset['kind']=='image'
            if still:source_offset=0
            if still:args+=['-protocol_whitelist','file,pipe','-f','image2','-pattern_type','none','-loop','1','-framerate',str(fps),'-i',path]
            else:
                ext=Path(path).suffix.lower();fmt=FORMATS.get(ext)
                if not fmt:raise ValueError('Unsupported prepared source type')
                args+=['-ss',f'{source:.9f}','-protocol_whitelist','file,pipe','-f',fmt,'-i',path]
            label=f'c{sequence}';track=enabled[c['track']];length_sec=length/fps
            if track['kind']!='audio':
                x,y,cw,ch=c['rect'];x=round(x*scale);y=round(y*scale);cw=max(1,round(cw*scale));ch=max(1,round(ch*scale))
                if c['fit']=='cover':fit=f'scale={cw}:{ch}:force_original_aspect_ratio=increase,crop={cw}:{ch}'
                else:fit=f'scale={cw}:{ch}:force_original_aspect_ratio=decrease,pad={cw}:{ch}:(ow-iw)/2:(oh-ih)/2:color=black@0'
                filters.append(f'[{index}:v:0]setpts=PTS-STARTPTS,fps={fps},trim=start_frame={source_offset}:end_frame={source_offset+length},tpad=stop_mode=clone:stop={length},trim=end_frame={length},settb=1/{fps},setpts=N+{start},format=rgba,{fit},colorchannelmixer=aa={c["opacity"]:.8f}[{label}v]')
                out=f'v{sequence}';begin=(start-.5)/fps;end=(start+length-.5)/fps
                filters.append(f'[{current}][{label}v]overlay=x={x}:y={y}:eof_action=pass:repeatlast=0:enable=\'gte(t,{begin:.9f})*lt(t,{end:.9f})\'[{out}]');current=out
            a=c['audio']
            if frame is None and asset and asset['has_audio'] and a['enabled']:
                gain=10**(a['gain_db']/20);offset=(c['start']-a['origin'])/fps;env=f'{gain:.10f}'
                if a['fade_in']:env+=f'*min(1,max(0,(t+{offset:.9f})/{a["fade_in"]/fps:.9f}))'
                if a['fade_out']:env+=f'*min(1,max(0,({a["span"]/fps:.9f}-(t+{offset:.9f}))/{a["fade_out"]/fps:.9f}))'
                filters.append(f'[{index}:a:0]asetpts=PTS-STARTPTS,aresample=48000,atrim=start_sample={source_offset*48000//fps}:end_sample={(source_offset+length)*48000//fps},asetpts=PTS-STARTPTS,aformat=channel_layouts=stereo,aeval=exprs=\'val(0)*{env}|val(1)*{env}\':c=stereo,adelay=delays={start*48000//fps}S:all=1,apad=whole_len={total*48000//fps},atrim=end_sample={total*48000//fps}[{label}a]');streams.append(label+'a')
            index+=1
        if frame is None:filters.append(''.join('['+a+']' for a in streams)+f'amix=inputs={len(streams)}:duration=first:dropout_transition=0:normalize=0,alimiter=limit=0.95:level=0:latency=1[aout]')
        script=self.folder/'filters.txt';script.write_text(';\n'.join(filters));args+=['-filter_complex_script',str(script),'-map',f'[{current}]']
        output=Path(s['output'])
        if frame is not None:
            tmp=output.with_suffix('.partial.png');args+=['-frames:v','1','-update','1',str(tmp)]
        else:
            tmp=Path(str(output)+'.partial.mp4');args+=['-map','[aout]','-frames:v',str(total),'-t',f'{seconds:.9f}','-r',str(fps),'-c:v','libx264','-preset','veryfast','-crf','25' if preview else '20','-pix_fmt','yuv420p','-c:a','aac','-b:a','160k','-ar','48000','-movflags','+faststart',str(tmp)]
        try:
            self.ffmpeg(args,seconds)
            if self.cancel.exists():raise Cancelled()
            if tmp.stat().st_size>MAX_FILE:raise ValueError('Rendered output exceeds 128 MiB')
            os.replace(tmp,output)
        finally:tmp.unlink(missing_ok=True)
        return dict(output=str(output),width=w,height=h,frames=total,fps=fps,duration=seconds)
    def run(self):
        try:
            if self.spec['op']=='probe':result=probe(Path(self.spec['source']),self.spec['extension'],self.spec['name'])
            elif self.spec['op']=='proxy':result=self.proxy()
            elif self.spec['op']=='render':result=self.render()
            else:raise ValueError('Unknown media operation')
            atomic(self.result,dict(state='complete',result=result))
        except Cancelled:atomic(self.result,dict(state='cancelled',error='Operation cancelled; source project preserved'))
        except Exception as e:atomic(self.result,dict(state='failed',error=str(e)))
if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--spec',type=Path,required=True);p.add_argument('--result',type=Path,required=True);p.add_argument('--cancel',type=Path,required=True);a=p.parse_args();Worker(json.loads(a.spec.read_text()),a.result,a.cancel).run()
