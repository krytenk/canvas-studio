"""Install pinned optional local motion tools. No account, cloud render or paid API."""
import argparse,os,shutil,subprocess,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
def setup(local=False):
    if not shutil.which('node') or int(subprocess.check_output(['node','--version'],text=True).strip().lstrip('v').split('.')[0])<22:raise SystemExit('Install Node.js 22 or later first.')
    target=ROOT/'tools/hyperframes' if local else Path(os.environ.get('XDG_DATA_HOME',Path.home()/'.local/share'))/'canvas-studio/motion-tools'
    target.mkdir(parents=True,exist_ok=True)
    if not local:
        for name in ['package.json','package-lock.json']:shutil.copy2(ROOT/'tools/hyperframes'/name,target/name)
    env={**os.environ,'HYPERFRAMES_NO_TELEMETRY':'1','HYPERFRAMES_SKIP_SKILLS':'1','HYPERFRAMES_NO_UPDATE_CHECK':'1'}
    subprocess.run(['npm','ci','--ignore-scripts','--no-audit','--no-fund','--prefix',str(target)],env=env,check=True)
    cli=target/'node_modules/hyperframes/bin/hyperframes.mjs'
    subprocess.run(['node',str(cli),'browser','ensure'],env=env,check=True)
    print('Local motion tools ready. Restart Canvas Studio if it was open. No subscription or cloud account was created.')
if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--local',action='store_true',help='Install into the source checkout for development');a=p.parse_args();setup(a.local)
