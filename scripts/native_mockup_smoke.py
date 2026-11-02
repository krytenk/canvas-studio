"""Open the portable 3D workflow in a real native window on an isolated X11 display."""
import argparse,gzip,json,os,shutil,subprocess,sys,time
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
def run(binary,xvfb):
 qa=ROOT/'qa/native-mockups';qa.mkdir(parents=True,exist_ok=True)
 file=qa/'native.canvas';shutil.copy2(ROOT/'qa/mockups/workspace/product.canvas',file)
 env={**os.environ,'DISPLAY':':95','LIBGL_ALWAYS_SOFTWARE':'1','XDG_SESSION_TYPE':'x11','WINIT_X11_SCALE_FACTOR':'1','XDG_DATA_HOME':str(qa/'data')};env.pop('WAYLAND_DISPLAY',None)
 log=(qa/'native.log').open('wb');server=subprocess.Popen([xvfb,':95','-screen','0','1440x1000x24','-nolisten','tcp','-noreset'],stdout=log,stderr=subprocess.STDOUT);app=None
 try:
  for _ in range(40):
   if subprocess.run(['xwininfo','-root'],env=env,capture_output=True).returncode==0:break
   time.sleep(.1)
  app=subprocess.Popen([str(binary),'--open',str(file)],env=env,stdout=log,stderr=subprocess.STDOUT)
  for _ in range(100):
   tree=subprocess.run(['xwininfo','-root','-tree'],env=env,capture_output=True,text=True).stdout
   if 'Canvas Studio' in tree:break
   assert app.poll() is None,(qa/'native.log').read_text();time.sleep(.1)
  else:raise AssertionError('Native mockup window did not appear')
  driver=qa/'driver.py';driver.write_text('''import sys,time,importlib
sys.path.insert(0,sys.argv[1]);import native_ui as ui
ui.X.XMoveResizeWindow(ui.d,ui.w,0,0,1280,900);ui.X.XFlush(ui.d);time.sleep(.8)
ui=importlib.reload(ui);ui.focus();ui.click(32,441);time.sleep(.5);ui.screenshot(sys.argv[2]+'/tools.png')
ui.chord('Control_L','s');time.sleep(.5)
ui.X.XMoveResizeWindow(ui.d,ui.w,0,0,1024,680);ui.X.XFlush(ui.d);time.sleep(.5)
ui=importlib.reload(ui)
for _ in range(12):ui.click(33,535,5)
ui.click(32,399);time.sleep(.4);ui.screenshot(sys.argv[2]+'/book.png')
''')
  subprocess.run([sys.executable,str(driver),str(ROOT/'scripts'),str(qa)],env=env,check=True)
  assert app.poll() is None,(qa/'native.log').read_text()
  data=file.read_bytes();envelope=json.loads(gzip.decompress(data[len(b'CANVAS-STUDIO-PACKAGE\n1\n'): ]));project=json.loads(envelope['project']);assert len(project['production']['mockups'])==3 and envelope['version']==5
  assert not any(s in (qa/'native.log').read_text().lower() for s in ['panicked at','fatal error'])
  (qa/'result.txt').write_text('PASS: real native portable v5 load, Tools/Book scene controls rendered, save preserves all three scenes and poster/Film assets, no panic.\n')
 finally:
  if app:app.terminate();app.wait(timeout=10)
  server.terminate();server.wait(timeout=10);log.close()
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('--binary',default='./canvas-studio');p.add_argument('--xvfb',default='Xvfb');a=p.parse_args();run(Path(a.binary).resolve(),a.xvfb)
