"""Regression for launcher migration and launching independently of cwd."""
import os, subprocess, tempfile
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
with tempfile.TemporaryDirectory(prefix='canvas-launcher-test-') as folder:
    base=Path(folder);(base/'Desktop').mkdir();(base/'applications').mkdir()
    old=base/'Desktop/Camva Studio.desktop';old.write_text('old desktop shortcut')
    alias=base/'applications/camva-studio.desktop';alias.write_text('old menu')
    subprocess.run(['python3',str(ROOT/'scripts/install_desktop.py'),'--prefix',str(base)],check=True,capture_output=True)
    entry=base/'Desktop/Canvas Studio.desktop'
    assert not old.exists() and entry.exists()
    assert str(ROOT/'launch-desktop.sh') in entry.read_text()
    assert 'NoDisplay=true' in alias.read_text()
    assert any(p.read_text()=='old desktop shortcut' for p in (base/'launcher-backups').rglob('*') if p.is_file())
    subprocess.run(['desktop-file-validate',str(entry)],check=True)
    result=subprocess.run([str(ROOT/'launch-desktop.sh'),'--version'],cwd='/tmp',env={**os.environ,'XDG_STATE_HOME':str(base/'state')},capture_output=True,timeout=15)
    assert result.returncode==0,result.stderr
    log=base/'state/canvas-studio/desktop.log'
    assert 'canvas-studio 1.' in log.read_text()
    assert log.stat().st_mode&0o777==0o600
print('Desktop migration, backup, entry validation, cwd-independent launch and private log: passed')
