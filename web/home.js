'use strict';
let homeOpen=false,homeFirst=true,homeReturn='design',homeDocument='';
const homeRoot=elem('section',{id:'studio-home',hidden:true,'aria-label':'Studio starting screen'});
homeRoot.innerHTML=`<div class="home-intro"><span class="eyebrow">YOUR CREATIVE WORKSPACE</span><h1>What would you like to make?</h1><p>Choose a starting point. Your current project stays here while you explore.</p></div><div id="home-tasks" class="home-tasks" aria-live="polite"><p>Loading starting points…</p></div><section class="home-current"><div><span class="eyebrow">YOUR CURRENT PROJECT</span><h2 id="home-project-title"></h2><p id="home-project-status"></p></div><div class="home-current-actions"><button id="home-continue" class="primary">Continue current project</button><button id="home-open-file">Open design…</button></div></section><details class="home-more"><summary>More tools and formats</summary><p id="home-desktop-tools"></p><p>Use the sidebar for pages, text, media, mockups and linked campaign layouts. Save an editable .canvas copy to move between desktop and browser.</p></details><p id="home-version" class="muted"></p>`;
document.querySelector('.workspace').append(homeRoot);
const homeRail=button('',()=>homeToggle(true),{class:'rail-tool','aria-label':'Studio home'});
homeRail.innerHTML='<svg viewBox="0 0 24 24"><path d="m3 10 9-7 9 7v11H3ZM9 21v-8h6v8"/></svg><span>Home</span>';
document.querySelector('.studio-rail').prepend(homeRail);
// Remember the current workspace before rail capture handlers close it.
document.addEventListener('click',event=>{if(homeRail.contains(event.target)&&!homeOpen)homeReturn=document.body.classList.contains('research-open')?'research':document.body.classList.contains('film-open')?'video':document.body.classList.contains('mockup-open')?'mockups':document.body.classList.contains('ai-open')?'generate':'design';},true);
function homeToggle(open){
 homeOpen=open;
 if(open){window.researchToggle?.(false);aiToggle(false);filmToggle(false);if(document.body.classList.contains('mockup-open'))$('mockup-close').click();document.querySelectorAll('.studio-rail .rail-tool.active').forEach(button=>button.classList.remove('active'));}
 document.body.classList.toggle('home-open',open);homeRoot.hidden=!open;homeRail.classList.toggle('active',open);
 if(open)homeRefresh();else fit();
}
document.querySelector('.studio-rail').addEventListener('click',event=>{if(homeOpen&&!homeRail.contains(event.target))homeToggle(false);},true);
function homeRefresh(){
 if(!state)return;
 if(homeDocument!==state.document_id){homeDocument=state.document_id;homeReturn='design';}
 $('home-project-title').textContent=state.title;
 $('home-project-status').textContent=`${state.pages.length} ${state.pages.length===1?'page':'pages'} · ${drafts.size?'Unapplied edits - apply them in the editor before exporting.':pending?'Save an editable copy to keep your changes.':state.recovery?'Recovery saved on this device. Download an editable copy to keep.':'Ready for your next idea.'}`;
 $('home-version').textContent=`Canvas Studio ${state.version} · on this computer`;
 document.title=`Canvas Studio ${state.version} · Creative workspace`;
}
async function homeTask(id){
 homeToggle(false);
 if(id==='product'){
  studioCategory='All';$('template-search').value='';document.querySelector('[data-panel="designs"]').classList.remove('active');studioPanel('designs');tab('edit');
 }else if(id==='campaign')studioOpenInspector('campaign');
 else if(id==='video')filmToggle(true);
 else if(id==='research')window.researchToggle?.(true);
 else if(id==='image'){await aiToggle(true);$('ai-image-tab').click();}
}
$('home-continue').onclick=()=>{const resume=homeReturn;homeToggle(false);if(resume==='research')researchToggle(true);else if(resume==='video')filmToggle(true);else if(resume==='generate')aiToggle(true);else if(resume==='mockups')document.querySelector('[aria-label="Product mockups"]').click();else{const name=!$('design-panel').hidden?'designs':!$('pages-panel').hidden?'pages':null;if(name)document.querySelector(`[data-panel="${name}"]`).classList.add('active');}};
$('home-open-file').onclick=()=>$('open').click();
window.studioHomeInitial=()=>{if(!state||!homeFirst)return;homeFirst=false;if(state.blank)homeToggle(true);};
window.studioHomeRefresh=()=>{homeRefresh();};
window.studioHomeOpened=()=>homeToggle(false);
api('/api/workflows').then(r=>r.json()).then(catalog=>{
 $('home-desktop-tools').textContent=catalog.desktop_extra;
 if(!catalog.generation_available){aiRail.hidden=true;}else{$('home-desktop-tools').after(button('Generate an image…',()=>homeTask('image')));}
 $('home-tasks').replaceChildren(...catalog.tasks.map((task,index)=>{
  const card=elem('article',{class:'home-task home-task-'+task.id});
  const art=elem('div',{class:'home-task-art','aria-hidden':'true'});art.append(elem('span',{},String(index+1).padStart(2,'0')));
  card.append(art,elem('h2',{},task.title),elem('p',{},task.description),button(task.title,()=>homeTask(task.id),{class:'primary'}),elem('small',{},task.next));return card;
 }));
}).catch(error=>{$('home-tasks').replaceChildren(elem('p',{},'Starting points could not load. Use the sidebar or open a design.'),button('Try again',()=>location.reload()));notice(error.message,true);});
if(state){window.studioHomeInitial();homeRefresh();}
