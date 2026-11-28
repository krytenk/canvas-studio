"""Real native generation/video navigation on an isolated X11 display."""
import argparse,os,subprocess,sys,time
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
def run(binary):
 qa=ROOT/'qa/native-generation';qa.mkdir(parents=True,exist_ok=True)
 env={**os.environ,'DISPLAY':':96','LIBGL_ALWAYS_SOFTWARE':'1','XDG_SESSION_TYPE':'x11','WINIT_X11_SCALE_FACTOR':'1','XDG_DATA_HOME':str(qa/'data')};env.pop('WAYLAND_DISPLAY',None)
 private=qa/'data/canvas-studio/ai';private.mkdir(parents=True,exist_ok=True);private.chmod(0o700);(private/'connections.json').write_text(__import__('json').dumps({'kaggle_username':'isolated-native-test','kaggle_token':'fake-native-test-token'}));(private/'connections.json').chmod(0o600)
 log=(qa/'native.log').open('wb');xvfb=subprocess.Popen(['Xvfb',':96','-screen','0','1440x1000x24','-nolisten','tcp','-noreset'],stdout=log,stderr=subprocess.STDOUT);app=None
 try:
  for _ in range(50):
   if subprocess.run(['xwininfo','-root'],env=env,capture_output=True).returncode==0:break
   time.sleep(.1)
  app=subprocess.Popen([str(binary)],env=env,stdout=log,stderr=subprocess.STDOUT)
  for _ in range(100):
   if 'Canvas Studio' in subprocess.run(['xwininfo','-root','-tree'],env=env,capture_output=True,text=True).stdout:break
   assert app.poll() is None,(qa/'native.log').read_text();time.sleep(.1)
  else:raise AssertionError('No native window')
  driver=qa/'driver.py';driver.write_text('''import sys,time,importlib
sys.path.insert(0,sys.argv[1]);import native_ui as ui
ui.X.XMoveResizeWindow(ui.d,ui.w,0,0,1280,900);ui.X.XFlush(ui.d);time.sleep(.8)
ui=importlib.reload(ui);ui.focus();ui.screenshot(sys.argv[2]+'/home.png');ui.click(274,20);ui.screenshot(sys.argv[2]+'/generate.png')
ui.click(351,20);ui.screenshot(sys.argv[2]+'/video.png')
ui.X.XMoveResizeWindow(ui.d,ui.w,0,0,1024,680);ui.X.XFlush(ui.d);time.sleep(.5)
ui=importlib.reload(ui);ui.click(158,20);ui.screenshot(sys.argv[2]+'/home-minimum.png');ui.click(274,20);ui.screenshot(sys.argv[2]+'/minimum.png')

''')
  subprocess.run([sys.executable,str(driver),str(ROOT/'scripts'),str(qa)],env=env,check=True)
  assert app.poll() is None,(qa/'native.log').read_text()
  assert not any(s in (qa/'native.log').read_text().lower() for s in ['panicked at','fatal error'])
  (qa/'result.txt').write_text('PASS: native startup, Generate/Video navigation, 1280x900 and 1024x680 screenshots, simplified model/prompt/Generate controls; visual review required for widget content.\n')
 finally:
  if app:app.terminate();app.wait(timeout=10)
  xvfb.terminate();xvfb.wait(timeout=10);log.close()
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('--binary',default='./canvas-studio');a=p.parse_args();run(Path(a.binary).resolve())
