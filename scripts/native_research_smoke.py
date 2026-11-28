"""Open the Research workspace in a real, isolated native window."""
import argparse,gzip,json,os,shutil,subprocess,sys,time
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
def run(binary):
    qa=ROOT/'qa/native-research';qa.mkdir(parents=True,exist_ok=True)
    working=qa/'working.canvas';shutil.copy2(ROOT/'qa/research/research.canvas',working)
    library=qa/'library'
    for name in ['Transcripts','Audio','Video']:(library/name).mkdir(parents=True,exist_ok=True)
    (library/'Transcripts/lesson.txt').write_text('A local source\nhttps://example.com/owned\nCaptions (en)\n\n[0:01] A useful passage.\n')
    env={**os.environ,'DISPLAY':':97','LIBGL_ALWAYS_SOFTWARE':'1','XDG_SESSION_TYPE':'x11','WINIT_X11_SCALE_FACTOR':'1','XDG_DATA_HOME':str(qa/'data'),'CANVAS_STUDIO_REFERENCE_ROOT':str(library),'CANVAS_STUDIO_PROFILE':'publishing'};env.pop('WAYLAND_DISPLAY',None)
    log=(qa/'native.log').open('wb');server=subprocess.Popen(['Xvfb',':97','-screen','0','1440x1000x24','-nolisten','tcp','-noreset'],stdout=log,stderr=subprocess.STDOUT);app=None
    try:
        for _ in range(50):
            if subprocess.run(['xwininfo','-root'],env=env,capture_output=True).returncode==0:break
            time.sleep(.1)
        app=subprocess.Popen([str(binary),'--open',str(working)],env=env,stdout=log,stderr=subprocess.STDOUT)
        for _ in range(100):
            if 'Canvas Studio' in subprocess.run(['xwininfo','-root','-tree'],env=env,capture_output=True,text=True).stdout:break
            assert app.poll() is None,(qa/'native.log').read_text();time.sleep(.1)
        else:raise AssertionError('No native window')
        driver=qa/'driver.py';driver.write_text('''import sys,time,importlib
sys.path.insert(0,sys.argv[1]);import native_ui as ui
ui.X.XMoveResizeWindow(ui.d,ui.w,0,0,1280,900);ui.X.XFlush(ui.d);time.sleep(.8)
ui=importlib.reload(ui);ui.focus();ui.click(350,20);ui.screenshot(sys.argv[2]+'/research.png')
ui.click(185,406);ui.screenshot(sys.argv[2]+'/reader.png')
ui.click(730,330);ui.chord('Control_L','s');time.sleep(.5);ui.screenshot(sys.argv[2]+'/notes-added.png')
ui.X.XMoveResizeWindow(ui.d,ui.w,0,0,1024,680);ui.X.XFlush(ui.d);time.sleep(.5)
ui=importlib.reload(ui);ui.screenshot(sys.argv[2]+'/minimum.png')
ui.click(148,20);ui.screenshot(sys.argv[2]+'/home-publishing.png')
''')
        subprocess.run([sys.executable,str(driver),str(ROOT/'scripts'),str(qa)],env=env,check=True)
        assert app.poll() is None,(qa/'native.log').read_text()
        assert 'panicked at' not in (qa/'native.log').read_text().lower()
        subprocess.run([str(binary),'--verify-project',str(working)],env=env,check=True,capture_output=True)
        bundle=json.loads(gzip.decompress(working.read_bytes().split(b'\n',2)[2]));project=json.loads(bundle['project']);assert 'First useful passage.' in project['pages'][0]['notes']
        (qa/'result.txt').write_text('PASS: real native Research and publishing Home, 1280x900 and 1024x680 proofs, selected passage added through native UI, saved and verified in portable project; visual review required.\n')
    finally:
        if app:app.terminate();app.wait(timeout=10)
        server.terminate();server.wait(timeout=10);log.close()
if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--binary',default='./canvas-studio');a=p.parse_args();run(Path(a.binary).resolve())
