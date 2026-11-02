'use strict';
const $=id=>document.getElementById(id);
const fragment=location.hash.slice(1);
if(fragment){sessionStorage.setItem('canvas-token',fragment);history.replaceState(null,'',location.pathname);}
const token=sessionStorage.getItem('canvas-token')||'';
let state=null,page=0,selected=null,previewURL=null,pending=false,working=false,previewSequence=0,range=[0,0],drag=null;
let queue=Promise.resolve();
const drafts=new Map();
function restoreDraft(form,key){form.dataset.draftKey=key;const draft=drafts.get(key);if(draft)for(const input of form.querySelectorAll('input,textarea,select')){const value=draft[input.id||input.name];if(value!==undefined){if(input.type==='checkbox')input.checked=value;else input.value=value;}}}
function updateDraftNotice(){$('discard-drafts').hidden=!drafts.size;}
document.addEventListener('input',event=>{const form=event.target.closest('form[data-draft-key]');if(!form)return;const values={};for(const input of form.querySelectorAll('input,textarea,select'))values[input.id||input.name]=input.type==='checkbox'?input.checked:input.value;drafts.set(form.dataset.draftKey,values);updateDraftNotice();});

const paragraphLabels={space_before:'Before (pt)',space_after:'After (pt)',first_line_indent:'Indent (pt)',inset_left:'Left inset',inset_right:'Right inset',inset_top:'Top inset',inset_bottom:'Bottom inset'};
const geometryLabels={x:'X',y:'Y',width:'Width',height:'Height',rotation:'Rotation °'};
function notice(message,error=false){$('message').textContent=message;$('message').classList.toggle('error',error);$('message').hidden=false;clearTimeout(notice.timer);if(!error)notice.timer=setTimeout(()=>$('message').hidden=true,6000);}
async function api(path,options={}){
 const response=await fetch(path,{...options,headers:{'X-Canvas-Token':token,...(state?{'X-Canvas-Revision':String(state.revision)}:{}),...options.headers}});
 if(!response.ok){let error;try{error=await response.json();}catch{error={error:response.statusText};}throw Error(error.error||`Request failed (${response.status})`);}
 return response;
}
function serial(task){queue=queue.then(async()=>{working=true;try{return await task();}catch(error){notice(error.message,true);}finally{working=false;}});return queue;}
async function command(value,draftKey){return serial(async()=>{await api('/api/command',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(value)});if(draftKey)drafts.delete(draftKey);if(['template','new'].includes(value.action))drafts.clear();updateDraftNotice();pending=true;await refresh();});}
function elem(tag,attributes={},text){const e=document.createElement(tag);for(const [key,value]of Object.entries(attributes)){if(key==='class')e.className=value;else e.setAttribute(key,String(value));}if(text!==undefined)e.textContent=text;return e;}
function button(text,action,attributes={}){const b=elem('button',{type:'button',...attributes},text);b.addEventListener('click',action);return b;}
function current(){return state?.elements.find(e=>e.id===selected);}
function tab(name){for(const e of document.querySelectorAll('[data-tab]')){e.classList.toggle('active',e.dataset.tab===name);e.setAttribute('aria-selected',String(e.dataset.tab===name));}for(const n of ['fields','layers','edit'])$('tab-'+n).hidden=n!==name;}
function select(id){document.body.classList.add('inspector-open');selected=id;renderSelection();renderLayers();renderTargets();tab('edit');}
async function refresh(){
 const response=await api(`/api/state?page=${page}`);state=await response.json();page=state.page;if(!state.elements.some(e=>e.id===selected))selected=null;
 $('document-title').value=state.title;$('page-title').value=state.pages[page].title;$('page-count').textContent=state.pages.length;
 $('session-state').textContent=`${state.recovery?'Recovery saved on this device':'Session only'}${pending?' · Save a copy when ready':''}`;
 $('dimensions').textContent=`${state.width_mm.toFixed(1)} × ${state.height_mm.toFixed(1)} mm · page ${page+1} of ${state.pages.length}`;
 $('undo').disabled=!state.undo;$('redo').disabled=!state.redo;$('up-page').disabled=page===0;$('down-page').disabled=page===state.pages.length-1;$('delete-page').disabled=state.pages.length===1;
 renderPages();renderFields();renderLayers();renderSelection();renderTargets();fit();await renderPreview();if(window.studioReady)window.studioReady();
}
function renderPages(){const root=$('pages');root.replaceChildren();state.pages.forEach((p,index)=>{const b=button('',()=>serial(async()=>{page=index;selected=null;await refresh();}),{class:'page-card'+(index===page?' active':''),'aria-label':`Page ${index+1}: ${p.title}`});b.append(elem('span',{class:'page-index'},index+1),elem('span',{class:'page-label'},p.title));root.append(b);});root.querySelector('.active')?.scrollIntoView({block:'nearest',inline:'nearest'});if(window.studioEnhancePages)window.studioEnhancePages();}
function renderFields(){const root=$('fields');root.replaceChildren();if(!state.fields.length){root.append(elem('p',{class:'muted'},'This design has no linked fields. Select a text or photo layer to edit it directly. Create shared fields in the desktop Production panel.'));return;}
 for(const field of state.fields){const form=elem('form',{class:'field-row'});const label=elem('label',{},field.key+(field.required?' · required':''));if(field.image){form.append(label,elem('p',{class:'muted'},field.value||'No photo selected'),button('Choose photo…',()=>choosePhoto({key:field.key})));}else{const input=elem('input',{'aria-label':field.key,value:field.value,name:'value'});label.append(input);form.append(label,elem('button',{type:'submit'},'Apply across collection'));form.addEventListener('submit',event=>{event.preventDefault();command({action:'field',key:field.key,value:input.value},'field:'+field.key);});}restoreDraft(form,'field:'+field.key);root.append(form);}
}
function renderLayers(){const root=$('layers');root.replaceChildren();for(const layer of [...state.elements].reverse()){const row=elem('div',{class:'layer-row'+(layer.id===selected?' active':'')});row.append(button(`${layer.type==='text'?'T':layer.type==='image'?'▧':'◇'} ${layer.name}`,()=>select(layer.id)),button(layer.locked?'Unlock':'Lock',()=>command({action:'lock',id:layer.id,value:!layer.locked}),{'aria-label':`${layer.locked?'Unlock':'Lock'} ${layer.name}`}));root.append(row);}}
function renderSelection(){const layer=current();$('selection-empty').hidden=!!layer;$('text-form').hidden=!layer||layer.type!=='text';$('image-editor').hidden=!layer||layer.type!=='image';$('geometry-form').hidden=!layer;if(!layer)return;
 $('geometry-form').querySelectorAll('input,button').forEach(e=>e.disabled=layer.locked);$('replace-photo').disabled=layer.locked;
 for(const key of Object.keys(geometryLabels))$('geometry-'+key).value=Number(layer[key].toFixed(3));
 restoreDraft($('geometry-form'),'geometry:'+layer.id);
 if(layer.type!=='text')return;
 $('text-content').value=layer.content;$('font').replaceChildren(...state.fonts.map(f=>elem('option',{value:f.id},f.name)));$('font').value=layer.family;
 $('font-size').value=Number(layer.size.toFixed(3));$('ink').value=layer.color;$('leading').value=Number(layer.leading.toFixed(3));$('tracking').value=Number(layer.tracking.toFixed(3));
 for(const key of ['bold','italic','underline'])$(key).checked=layer[key];for(const key of Object.keys(paragraphLabels))$('paragraph-'+key).value=Number(layer.paragraph[key].toFixed(3));
 $('linked-notice').hidden=!layer.linked_style;
 $('text-form').querySelectorAll('input,textarea,select,button').forEach(e=>e.disabled=layer.locked);
 if(layer.linked_style){$('text-form').querySelectorAll('input,select').forEach(e=>e.disabled=true);}
 restoreDraft($('text-form'),'text:'+layer.id);
 // Imported font faces are selected as their actual weight/style, as on desktop.
 if(layer.family.startsWith('imported:')){$('bold').disabled=true;$('italic').disabled=true;}
}
function renderTargets(){const root=$('hit-targets');root.replaceChildren();if(!state)return;
 for(const layer of state.elements){if(!layer.visible)continue;const b=button('',()=>select(layer.id),{class:'layer-hit'+(layer.id===selected?' selected':''),'aria-label':`Select ${layer.name}`});b.style.left=`${100*layer.x/state.width}%`;b.style.top=`${100*layer.y/state.height}%`;b.style.width=`${100*layer.width/state.width}%`;b.style.height=`${100*layer.height/state.height}%`;b.style.transform=`rotate(${layer.rotation}deg)`;b.disabled=layer.locked;
 b.addEventListener('pointerdown',event=>{if(event.button!==0||layer.locked)return;event.preventDefault();selected=layer.id;renderSelection();renderLayers();tab('edit');b.classList.add('selected');b.setPointerCapture(event.pointerId);drag={id:layer.id,x:event.clientX,y:event.clientY,startX:layer.x,startY:layer.y,width:layer.width,height:layer.height,rotation:layer.rotation,scale:$('paper').clientWidth/state.width,node:b};});
 b.addEventListener('pointermove',event=>{if(!drag||drag.node!==b)return;const x=drag.startX+(event.clientX-drag.x)/drag.scale;const y=drag.startY+(event.clientY-drag.y)/drag.scale;b.style.left=`${100*x/state.width}%`;b.style.top=`${100*y/state.height}%`;});
 b.addEventListener('pointerup',event=>{if(!drag||drag.node!==b)return;const d=drag;drag=null;const dx=(event.clientX-d.x)/d.scale,dy=(event.clientY-d.y)/d.scale;if(Math.hypot(dx,dy)>1)command({action:'geometry',id:d.id,x:d.startX+dx,y:d.startY+dy,width:d.width,height:d.height,rotation:d.rotation});});
 b.addEventListener('pointercancel',()=>{drag=null;renderTargets();});root.append(b);}
 $('empty-hint').hidden=state.elements.length>0;
}
async function renderPreview(){const sequence=++previewSequence;try{const response=await api(`/api/preview.svg?page=${page}`);const blob=await response.blob();if(sequence!==previewSequence)return;const url=URL.createObjectURL(blob),old=previewURL;previewURL=url;$('preview').src=url;if(old)URL.revokeObjectURL(old);}catch(error){if(sequence===previewSequence){$('preview').removeAttribute('src');notice(`Preview: ${error.message}`,true);}}}
function fit(){if(!state)return;const area=$('paper-scroll');let scale=$('zoom').value==='fit'?Math.min((area.clientWidth-54)/state.width,(area.clientHeight-50)/state.height):Number($('zoom').value);scale=Math.max(.08,scale);$('paper').style.width=`${state.width*scale}px`;$('paper').style.height=`${state.height*scale}px`;}
async function upload(path,file){return serial(async()=>{await api(path,{method:'POST',body:file});if(path==='/api/open'){drafts.clear();updateDraftNotice();}pending=true;await refresh();if(path.startsWith('/api/add-image'))select(state.elements.at(-1).id);});}
let photoTarget=null;
function choosePhoto(target){photoTarget=target;$('photo-file').value='';$('photo-file').click();}
async function download(path,name){return serial(async()=>{if(drafts.size){notice('Apply your form edits before downloading, or discard the unapplied edits.',true);return;}const response=await api(path);const revision=Number(response.headers.get('X-Canvas-Revision'));const url=URL.createObjectURL(await response.blob());const a=elem('a',{href:url,download:name});document.body.append(a);a.click();a.remove();setTimeout(()=>URL.revokeObjectURL(url),10000);if(name.endsWith('.canvas')&&revision===state.revision){pending=false;$('session-state').textContent=`Local engine · ${state.version} · Editable copy downloaded`;}notice(`Downloaded ${name}.`);});}
for(const [key,label]of Object.entries(paragraphLabels)){const l=elem('label',{},label);l.append(elem('input',{id:'paragraph-'+key,type:'number',min:0,max:300,step:'any'}));$('paragraph-controls').append(l);}
for(const [key,label]of Object.entries(geometryLabels)){const l=elem('label',{},label);l.append(elem('input',{id:'geometry-'+key,type:'number',step:'any'}));$('geometry-controls').append(l);}
for(const b of document.querySelectorAll('[data-tab]'))b.addEventListener('click',()=>tab(b.dataset.tab));
$('document-title').addEventListener('change',event=>command({action:'title',value:event.target.value}));
$('page-title').addEventListener('change',event=>command({action:'page_title',id:state.pages[page].id,value:event.target.value}));
$('add-photo').onclick=()=>choosePhoto({add:true,page});
$('open').onclick=()=>{if((pending||drafts.size)&&!confirm('Download your current document first if you need an independent copy. Open another design?'))return;$('open-file').value='';$('open-file').click();};
$('open-file').onchange=event=>{const file=event.target.files[0];if(file)upload('/api/open',file);};
$('font-file').onchange=event=>{const file=event.target.files[0];if(file)upload('/api/font',file);};
$('import-font').onclick=()=>{$('font-file').value='';$('font-file').click();};
$('photo-file').onchange=event=>{const file=event.target.files[0];if(file&&photoTarget){const target=photoTarget;upload(target.add?`/api/add-image?page=${target.page}`:target.key?`/api/photo?key=${encodeURIComponent(target.key)}`:`/api/image?id=${target.id}`,file);}};
$('replace-photo').onclick=()=>{const layer=current();if(layer)choosePhoto(layer.binding?{key:layer.binding}:{id:layer.id});};
$('save').onclick=()=>download('/api/download.canvas','design.canvas');$('pdf').onclick=()=>download('/api/download.pdf','design.pdf');$('png').onclick=()=>download(`/api/download.png?page=${page}`,'page.png');
$('undo').onclick=()=>command({action:'undo'});$('redo').onclick=()=>command({action:'redo'});
$('discard-drafts').onclick=()=>{drafts.clear();updateDraftNotice();serial(()=>refresh());};
$('refresh').onclick=()=>serial(()=>refresh());$('guide').onclick=()=>$('help').showModal();$('close-help').onclick=()=>$('help').close();
$('preflight').onclick=()=>serial(async()=>{const report=await(await api('/api/preflight')).json();notice(report.issues.length?report.issues.map(i=>`Page ${i.page_index+1} · ${i.severity}: ${i.message}`).join('\n'):'Print checks passed. Review a physical proof before production.',report.issues.some(i=>i.severity==='Error'));});
$('add-page').onclick=()=>command({action:'add_page'});$('add-text').onclick=()=>serial(async()=>{await api('/api/command',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({action:'add_text',page})});pending=true;await refresh();select(state.elements.at(-1).id);});
$('delete-page').onclick=()=>{if(confirm('Delete this page? You can undo this action.'))command({action:'delete_page',id:state.pages[page].id});};
$('up-page').onclick=()=>{const target=page-1;command({action:'move_page',id:state.pages[page].id,to:target}).then(()=>{page=target;serial(()=>refresh());});};
$('down-page').onclick=()=>{const target=page+1;command({action:'move_page',id:state.pages[page].id,to:target}).then(()=>{page=target;serial(()=>refresh());});};
for(const b of document.querySelectorAll('[data-template]'))b.onclick=()=>{if((pending||drafts.size)&&!confirm('Replace the current session document? Download a copy first. Undo is available.'))return;page=0;command(b.dataset.template==='new'?{action:'new'}:{action:'template',template:b.dataset.template});};
$('text-form').onsubmit=event=>{event.preventDefault();if(!selected)return;const value={action:'text',id:selected,content:$('text-content').value,family:$('font').value,size:Number($('font-size').value),color:$('ink').value,bold:$('bold').checked,italic:$('italic').checked,underline:$('underline').checked,leading:Number($('leading').value),tracking:Number($('tracking').value)};for(const key of Object.keys(paragraphLabels))value[key]=Number($('paragraph-'+key).value);command(value,'text:'+selected);};
$('geometry-form').onsubmit=event=>{event.preventDefault();const value={action:'geometry',id:selected};for(const key of Object.keys(geometryLabels))value[key]=Number($('geometry-'+key).value);command(value,'geometry:'+selected);};
$('duplicate').onclick=()=>command({action:'duplicate',id:selected});$('delete').onclick=()=>command({action:'delete',id:selected});
$('text-content').addEventListener('select',()=>{range=[$('text-content').selectionStart,$('text-content').selectionEnd];});
for(const b of document.querySelectorAll('[data-emphasis]'))b.onclick=()=>{const text=$('text-content').value;if(range[0]===range[1]){notice('Select characters in the text area first.');return;}const target=selected;serial(async()=>{await api('/api/command',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({action:'text',id:target,content:text})});const next=await(await api(`/api/state?page=${page}`)).json();state=next;await api('/api/command',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({action:'emphasis',id:target,start:[...text.slice(0,range[0])].length,end:[...text.slice(0,range[1])].length,property:b.dataset.emphasis,value:true})});const draft=drafts.get('text:'+target);if(draft){draft['text-content']=text;drafts.set('text:'+target,draft);}pending=true;await refresh();});};
$('zoom').onchange=fit;new ResizeObserver(fit).observe($('paper-scroll'));
window.addEventListener('beforeunload',event=>{if(pending||drafts.size){event.preventDefault();event.returnValue='';}});
document.addEventListener('keydown',event=>{if(!(event.ctrlKey||event.metaKey))return;const key=event.key.toLowerCase();if(key==='s'){event.preventDefault();$('save').click();}else if(key==='z'&&!['INPUT','TEXTAREA'].includes(document.activeElement?.tagName)){event.preventDefault();command({action:event.shiftKey?'redo':'undo'});}});
refresh().catch(error=>notice(error.message,true));
