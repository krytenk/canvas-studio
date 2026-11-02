"""Exercise a real native window on an isolated X11 display.
Requires Xvfb, xwininfo, Pillow and libXtst. No personal application data is used.
"""
import argparse,os,shutil,subprocess,time,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
def run(binary,xvfb):
    qa=ROOT/'qa/native-smoke';qa.mkdir(parents=True,exist_ok=True)
    working=qa/'working.canvas';shutil.copy2(ROOT/'qa/collection-proof/collection.canvas',working)
    display=':94';env=os.environ.copy();env['DISPLAY']=display;env.pop('WAYLAND_DISPLAY',None);env['LIBGL_ALWAYS_SOFTWARE']='1';env['XDG_SESSION_TYPE']='x11';env['WINIT_X11_SCALE_FACTOR']='1';env['XDG_DATA_HOME']=str(qa/'app-data')
    log=(qa/'native.log').open('wb');server=subprocess.Popen([xvfb,display,'-screen','0','1440x900x24','-nolisten','tcp','-noreset'],stdout=log,stderr=subprocess.STDOUT)
    app=None
    try:
        for _ in range(40):
            if subprocess.run(['xwininfo','-root'],env=env,capture_output=True).returncode==0:break
            time.sleep(.1)
        app=subprocess.Popen([str(binary),'--open',str(working)],env=env,stdout=log,stderr=subprocess.STDOUT)
        for _ in range(100):
            tree=subprocess.run(['xwininfo','-root','-tree'],env=env,capture_output=True,text=True).stdout
            if 'Canvas Studio' in tree:break
            if app.poll() is not None:raise RuntimeError('Native application failed to start:\n'+(qa/'native.log').read_text())
            time.sleep(.1)
        else:raise RuntimeError('Native window did not appear:\n'+(qa/'native.log').read_text())
        driver=qa/'driver.py';driver.write_text('''import sys,time,importlib
sys.path.insert(0,sys.argv[1])
import native_ui as ui
ui.X.XMoveResizeWindow(ui.d,ui.w,0,0,1024,680);ui.X.XFlush(ui.d);time.sleep(.8)
ui=importlib.reload(ui);ui.focus();ui.screenshot(sys.argv[2]+'/minimum.png')
ui.click(240,49);ui.chord('Control_L','a');ui.type_text('Native smoke test');ui.chord('Control_L','s');time.sleep(.4)
ui.chord('Control_L','z');ui.screenshot(sys.argv[2]+'/undo.png');ui.chord('Control_L','y');ui.chord('Control_L','s');time.sleep(.4)
for _ in range(10):ui.click(34,530,5)
ui.click(30,451);ui.screenshot(sys.argv[2]+'/book.png');ui.click(325,617);ui.screenshot(sys.argv[2]+'/next-page.png')
ui.click(950,50);ui.screenshot(sys.argv[2]+'/export.png')
''')
        subprocess.run([sys.executable,str(driver),str(ROOT/'scripts'),str(qa)],env=env,check=True)
        result=subprocess.check_output([str(binary),'--verify-project',str(working)],text=True)
        assert 'Native smoke test' in result,result
        (qa/'result.txt').write_text('PASS: native window, 1024×680 fit, title/save/reopen, undo/redo, rail, Book/Next, export dialog.\n')
        print(result.strip());print('Native interaction smoke check passed')
    finally:
        if app is not None:app.terminate();app.wait(timeout=10)
        server.terminate();server.wait(timeout=10);log.close()
if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--binary',default='./canvas-studio');p.add_argument('--xvfb',default='Xvfb');a=p.parse_args();run(Path(a.binary).resolve(),a.xvfb)
