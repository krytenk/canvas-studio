"""Install/repair user-owned Linux launchers without admin privileges."""
import argparse,os,shlex,shutil,time
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
def install(prefix=None):
    home=Path.home();data=Path(os.environ.get('XDG_DATA_HOME',home/'.local/share'))
    desktop=home/'Desktop';apps=data/'applications'
    if prefix:desktop=prefix/'Desktop';apps=prefix/'applications'
    desktop.mkdir(parents=True,exist_ok=True);apps.mkdir(parents=True,exist_ok=True)
    backup=(prefix or Path(os.environ.get('XDG_STATE_HOME',home/'.local/state'))/'canvas-studio')/'launcher-backups'/str(time.time_ns());backup.mkdir(parents=True,exist_ok=True)
    icon=ROOT/'assets/canvas-studio.svg'
    entry=f'''[Desktop Entry]\nVersion=1.0\nType=Application\nName=Canvas Studio\nGenericName=Design, Print & Motion Studio\nComment=Local visual design, books, film and mockups\nExec="{ROOT / 'launch-desktop.sh'}" %f\nTryExec={ROOT / 'launch-desktop.sh'}\nPath={ROOT}\nIcon={icon}\nTerminal=false\nCategories=Graphics;2DGraphics;3DGraphics;\nStartupNotify=true\nStartupWMClass=Canvas Studio\n'''
    for parent in [desktop,apps]:
        for name in ['Camva Studio.desktop','Canvas Studio.desktop','camva-studio.desktop','canvas-studio.desktop']:
            p=parent/name
            if p.exists():shutil.copy2(p,backup/(parent.name+'-'+name))
    old=desktop/'Camva Studio.desktop';dest=desktop/'Canvas Studio.desktop'
    if old.exists() and not dest.exists():old.rename(dest)
    dest.write_text(entry);dest.chmod(0o755)
    canonical=apps/'canvas-studio.desktop';canonical.write_text(entry)
    alias=apps/'camva-studio.desktop'
    if alias.exists():alias.write_text(entry+'NoDisplay=true\n')
    legacy=home/'Applications/Camva-Studio/launch-camva-studio'
    if not prefix and legacy.is_file():
        shutil.copy2(legacy,backup/'legacy-launcher.sh');legacy.write_text('#!/usr/bin/env bash\nexec '+shlex.quote(str(ROOT/'launch-desktop.sh'))+' "$@"\n');legacy.chmod(0o755)
    print('Desktop launcher:',dest);print('Application menu:',canonical);print('Previous launchers backed up:',backup)
if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--prefix',type=Path,help='Isolated installation root for tests');a=p.parse_args();install(a.prefix)
