"""Small X11 native-window smoke helper. Requires Pillow, libX11, libXtst and xwininfo.
Run on an isolated Xvfb display: DISPLAY=:93 python3 scripts/native_ui.py shot qa/native.png
Other commands: click X Y, chord Control_L s, type 'Example'. Coordinates are client-relative.
Wayland may block synthetic input; use an isolated X11 test display in that case.
"""
import ctypes as C, subprocess, re, time, sys
from PIL import Image
X=C.CDLL('libX11.so.6'); T=C.CDLL('libXtst.so.6')
X.XOpenDisplay.restype=C.c_void_p; d=X.XOpenDisplay(None)
X.XDefaultRootWindow.argtypes=[C.c_void_p];X.XDefaultRootWindow.restype=C.c_ulong; root=X.XDefaultRootWindow(d)
for name,args,result in [('XRaiseWindow',[C.c_void_p,C.c_ulong],C.c_int),('XSetInputFocus',[C.c_void_p,C.c_ulong,C.c_int,C.c_ulong],C.c_int),('XMoveResizeWindow',[C.c_void_p,C.c_ulong,C.c_int,C.c_int,C.c_uint,C.c_uint],C.c_int),('XFlush',[C.c_void_p],C.c_int),('XKeysymToKeycode',[C.c_void_p,C.c_ulong],C.c_uint),('XStringToKeysym',[C.c_char_p],C.c_ulong),('XGetImage',[C.c_void_p,C.c_ulong,C.c_int,C.c_int,C.c_uint,C.c_uint,C.c_ulong,C.c_int],C.c_void_p),('XGetPixel',[C.c_void_p,C.c_int,C.c_int],C.c_ulong),('XDestroyImage',[C.c_void_p],C.c_int)]:
 f=getattr(X,name);f.argtypes=args;f.restype=result
T.XTestFakeButtonEvent.argtypes=[C.c_void_p,C.c_uint,C.c_int,C.c_ulong];T.XTestFakeMotionEvent.argtypes=[C.c_void_p,C.c_int,C.c_int,C.c_int,C.c_ulong];T.XTestFakeKeyEvent.argtypes=[C.c_void_p,C.c_uint,C.c_int,C.c_ulong]
raw=subprocess.check_output(['xwininfo','-root','-tree'],text=True); m=re.search(r'(0x[0-9a-f]+) "Canvas Studio[^\n]*? (\d+)x(\d+)[+-]\d+[+-]\d+\s+([+-]\d+)([+-]\d+)',raw)
if not m: raise RuntimeError('No Canvas Studio X11 window')
w=int(m[1],16);width,height,x,y=map(int,m.groups()[1:]);print('window',hex(w),width,height,x,y)
def focus(): X.XRaiseWindow(d,w);X.XSetInputFocus(d,w,2,0);X.XFlush(d);time.sleep(.2)
def click(cx,cy,button=1): T.XTestFakeMotionEvent(d,-1,x+cx,y+cy,0);T.XTestFakeButtonEvent(d,button,1,0);T.XTestFakeButtonEvent(d,button,0,0);X.XFlush(d);time.sleep(.2)
def key(name,down): code=X.XKeysymToKeycode(d,X.XStringToKeysym(name.encode()));T.XTestFakeKeyEvent(d,code,down,0);X.XFlush(d)
def chord(*names):
 for name in names:key(name,1)
 for name in reversed(names):key(name,0)
 time.sleep(.2)
def type_text(text):
 for c in text:
  shifted=c.isupper() or c in ':_{}()'
  mapping={':':'semicolon','_':'minus','{':'bracketleft','}':'bracketright','(':'9',')':'0','/':'slash','.':'period','-':'minus',' ':'space','\n':'Return'}
  if shifted:key('Shift_L',1)
  name=mapping.get(c,c.lower());key(name,1);key(name,0)
  if shifted:key('Shift_L',0)
 time.sleep(.2)
def screenshot(path):
 time.sleep(.4);img=X.XGetImage(d,w,0,0,width,height,0xffffffff,2)
 if not img: raise RuntimeError('XGetImage failed')
 # XGetPixel is portable across XImage layouts; screenshots are infrequent.
 pixels=bytearray(width*height*3)
 for py in range(height):
  for px in range(width):
   p=X.XGetPixel(img,px,py);i=(py*width+px)*3;pixels[i:i+3]=bytes([(p>>16)&255,(p>>8)&255,p&255])
 X.XDestroyImage(img);Image.frombytes('RGB',(width,height),bytes(pixels)).save(path);print(path)
if __name__=='__main__':
 focus()
 if sys.argv[1]=='shot':screenshot(sys.argv[2])
 elif sys.argv[1]=='click':click(int(sys.argv[2]),int(sys.argv[3]))
 elif sys.argv[1]=='chord':chord(*sys.argv[2:])
 elif sys.argv[1]=='type':type_text(sys.argv[2])
