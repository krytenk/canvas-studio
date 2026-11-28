'use strict';
let researchOpen=false,researchLoaded=false,researchEntries=[],researchSource=null,researchFileId=null,researchCue=null,researchDoc='',researchSequence=0,researchUploadDoc='';
const researchRoot=elem('section',{id:'research-studio',hidden:true,'aria-label':'Personal research studio'});
researchRoot.innerHTML=`<div class="research-heading"><div><span class="eyebrow">YOUR SOURCES · YOUR PROJECT</span><h1>Research</h1><p>Bring Reference Desk transcripts, captions, audio and video into your project.</p><p class="muted">Imported sources travel with your editable project. Original files stay in Reference Desk.</p></div><button id="research-close">Back to design</button></div><div class="research-actions"><button id="research-launch">Open Reference Desk</button><button id="research-refresh">Refresh saved files</button><button id="research-upload" class="primary">Import transcript / SRT…</button><button id="research-upload-media">Import audio / video…</button></div><div class="research-layout"><aside class="research-library"><label>Find a file or source<input type="search" id="research-search" placeholder="Search saved filenames and project sources"></label><h2>Reference Desk files</h2><p id="research-scope" class="muted">Saved files on this computer</p><div id="research-files"></div><h2>Sources in this project</h2><div id="research-project-sources"></div></aside><section class="research-reading"><div id="research-reader"><h2>Read and use</h2><p>Preview a saved transcript, or select a source already in this project.</p></div><section class="research-notes"><h2 id="research-notes-title">Page notes</h2><pre id="research-notes"></pre></section></section></div>`;
document.querySelector('.workspace').append(researchRoot);
const researchRail=button('',()=>researchToggle(!researchOpen),{class:'rail-tool','aria-label':'Personal research'});
researchRail.innerHTML='<svg viewBox="0 0 24 24"><path d="M3 4h8l1 2 1-2h8v16h-8l-1 1-1-1H3ZM12 6v14M6 8h3M6 12h3M15 8h3M15 12h3"/></svg><span>Research</span>';
document.querySelector('[aria-label="Image and video generation"]').before(researchRail);
window.researchToggle=function(open){
 researchOpen=open;if(open){homeToggle(false);aiToggle(false);filmToggle(false);if(document.body.classList.contains('mockup-open'))$('mockup-close').click();}
 document.body.classList.toggle('research-open',open);researchRoot.hidden=!open;researchRail.classList.toggle('active',open);
 if(open){researchRefresh();if(!researchLoaded)researchLoadLibrary();}else fit();
};
document.querySelector('.studio-rail').addEventListener('click',event=>{if(researchOpen&&!researchRail.contains(event.target))researchToggle(false);},true);
$('research-close').onclick=()=>researchToggle(false);
$('research-search').oninput=()=>{researchRenderLists();};
async function researchLoadLibrary(){
 try{const data=await(await api('/api/research/library')).json();researchEntries=data.entries;researchLoaded=true;$('research-launch').disabled=!data.can_launch;$('research-scope').textContent=data.scope;researchRenderLists();}
 catch(error){$('research-files').replaceChildren(elem('p',{},'Saved files could not load. Import a file, or try Refresh saved files.'));notice(error.message,true);}
}
$('research-refresh').onclick=researchLoadLibrary;
function researchRefresh(){
 if(!state)return;
 if(researchDoc!==state.document_id){researchDoc=state.document_id;researchSource=null;researchFileId=null;researchCue=null;researchSequence++;}
 if(researchSource&&!researchFileId&&!state.research.some(s=>s.id===researchSource.id)){researchSource=null;researchCue=null;}
 if(!researchOpen)return;
 researchRenderLists();researchRenderReader();
 $('research-notes-title').textContent='Notes for page '+(page+1);
 $('research-notes').textContent=state.notes||'Choose a passage, then add it to page notes.';
}
window.researchRefresh=researchRefresh;
function researchRenderLists(){
 const q=$('research-search').value.toLowerCase(),files=$('research-files');files.replaceChildren();
 const entries=researchEntries.filter(e=>e.name.toLowerCase().includes(q));
 if(!entries.length)files.append(elem('p',{class:'muted'},researchEntries.length?'No matching saved files.':'No saved files yet. Open Reference Desk, or import a file above.'));
 for(const entry of entries){const row=elem('article',{class:'research-file'});row.append(elem('strong',{},entry.name),elem('small',{},`${entry.kind.toUpperCase()} · ${(entry.bytes/1024).toFixed(1)} KB`),button(['txt','srt'].includes(entry.kind)?'Preview transcript':'Import to video library',()=>['txt','srt'].includes(entry.kind)?researchPreview(entry.id):researchImportMedia(entry.id)));files.append(row);}
 const saved=$('research-project-sources');saved.replaceChildren();
 const sources=(state?.research||[]).filter(s=>`${s.title} ${s.filename}`.toLowerCase().includes(q));
 if(!sources.length)saved.append(elem('p',{class:'muted'},state?.research.length?'No matching project sources.':'Import a source to keep a portable research copy here.'));
 for(const source of sources)saved.append(button(source.title+' · '+source.kind.toUpperCase(),()=>researchRead(source.id),{class:researchSource?.id===source.id&&!researchFileId?'active':''}));
}
async function researchPreview(id){
 const doc=state.document_id,sequence=++researchSequence;
 try{const source=await(await api('/api/research/preview?id='+encodeURIComponent(id))).json();if(sequence!==researchSequence||state.document_id!==doc)return;researchSource=source;researchFileId=id;researchCue=source.cues.length?0:null;researchRenderReader();}
 catch(error){notice(error.message,true);}
}
async function researchRead(id){
 const doc=state.document_id,sequence=++researchSequence;
 try{const source=await(await api('/api/research/source?id='+encodeURIComponent(id))).json();if(sequence!==researchSequence||state.document_id!==doc)return;researchSource=source;researchFileId=null;researchCue=source.cues.length?0:null;researchRenderLists();researchRenderReader();}
 catch(error){notice(error.message,true);}
}
function researchRenderReader(){
 const root=$('research-reader');root.replaceChildren();const source=researchSource;
 if(!source){root.append(elem('h2',{},'Read and use'),elem('p',{},'Preview a saved transcript, or select a source already in this project.'));return;}
 root.append(elem('h2',{},source.title),elem('p',{class:'muted'},source.filename+(source.method?' · '+source.method:'')));
 if(source.source_url)root.append(elem('a',{href:source.source_url,target:'_blank',rel:'noopener noreferrer'},'Open original source'));
 const actions=elem('div',{class:'research-actions'});
 if(researchFileId){root.append(elem('p',{},'Preview only. Copy this transcript into the project to use passages.'));actions.append(button('Copy source into project',researchSave,{class:'primary'}));}
 else{
  if(source.text)actions.append(button('Add passage to page notes',()=>researchAction('research_note'),{class:'primary'}));
  if(source.kind==='srt')actions.append(button('Add captions to video',()=>researchAction('research_captions')));
  if(source.media_asset)actions.append(button('Open video library',()=>{researchToggle(false);filmToggle(true);}));
  actions.append(button('Remove project copy',()=>researchAction('research_remove'),{class:'quiet-danger'}));
 }
 root.append(actions);
 if(source.kind==='srt')root.append(elem('p',{class:'muted'},'Video captions support up to 200 clips within ten minutes. Review their timing against the recording.'));
 const content=elem('div',{class:'research-transcript','aria-label':'Transcript passages'});
 if(!source.cues.length)content.append(elem('pre',{},source.text||'Audio/video is available in the project’s video library.'));
 else source.cues.forEach((cue,index)=>{
  const time=`${String(Math.floor(cue.start_ms/60000)).padStart(2,'0')}:${String(Math.floor(cue.start_ms/1000)%60).padStart(2,'0')}`;
  content.append(button(time+'  '+cue.text,()=>{researchCue=index;researchRenderReader();$('research-reader').querySelectorAll('.research-passage')[index]?.focus();},{class:'research-passage'+(researchCue===index?' active':''),'aria-pressed':researchCue===index?'true':'false'}));
 });
 root.append(content);
}
function researchPost(path,value){return api(path,{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(value)}).then(r=>r.json());}
function researchSave(){
 const source=researchSource,id=researchFileId,doc=state.document_id;
 return serial(async()=>{if(doc!==state.document_id||!source||!id)throw Error('Preview the file again in the current project.');const result=await researchPost('/api/research/import',{id,fingerprint:source.fingerprint});pending=true;await refresh();await researchRead(result.id);notice('Source copied into this project. Original file kept.');});
}
function researchAction(action){
 const source=researchSource,cue=researchCue,doc=state.document_id;
 return serial(async()=>{if(!source||doc!==state.document_id)throw Error('Select a source in the current project.');const value={action,id:source.id,page};if(action==='research_note'&&cue!==null)value.cue=cue;
  await researchPost('/api/command',value);if(action==='research_remove'){researchSource=null;researchCue=null;}pending=true;await refresh();
  if(action==='research_captions'){researchToggle(false);filmToggle(true);notice('Captions added. Review their timing in Video studio.');}
 });
}
async function researchReadyMedia(id){
 pending=true;await refresh();const asset=state.research.find(s=>s.id===id)?.media_asset;
 if(asset)try{await filmPost('proxy',{asset});}catch(error){notice('Media imported. Preview files need attention: '+error.message,true);}
 researchToggle(false);filmToggle(true);
}
function researchImportMedia(id){const doc=state.document_id;return serial(async()=>{if(doc!==state.document_id)throw Error('Choose the file again in this project.');notice('Copying media into the project…');const result=await researchPost('/api/research/media',{id});await researchReadyMedia(result.id);});}
$('research-launch').onclick=()=>serial(async()=>{await researchPost('/api/research/launch',{});notice('Reference Desk opened. Save your files there, then use Refresh saved files here.');});
const researchFile=elem('input',{type:'file',accept:'.txt,.srt',hidden:true}),researchMedia=elem('input',{type:'file',accept:'.mp3,.wav,.m4a,.flac,.ogg,.mp4,.mov,.webm,.mkv,.m4v',hidden:true});document.body.append(researchFile,researchMedia);
for(const [id,input,media]of [['research-upload',researchFile,false],['research-upload-media',researchMedia,true]]){
 $(id).onclick=()=>{researchUploadDoc=state.document_id;input.value='';input.click();};
 input.onchange=()=>{const file=input.files[0],doc=researchUploadDoc;if(!file)return;serial(async()=>{if(doc!==state.document_id)throw Error('Choose the file again in the current project.');if(file.size>(media?128*1024*1024:1000000))throw Error(media?'Keep media files below 128 MiB.':'Keep transcripts below 1 MB.');notice('Importing '+file.name+'…');const result=await(await api('/api/research/'+(media?'upload-media':'upload')+'?name='+encodeURIComponent(file.name),{method:'POST',body:file})).json();if(media)await researchReadyMedia(result.id);else{pending=true;await refresh();await researchRead(result.id);}});};
}
if(state)researchRefresh();
