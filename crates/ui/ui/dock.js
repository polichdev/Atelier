'use strict';
const invoke=window.__TAURI__?.core?.invoke;
const $=id=>document.getElementById(id);
let cfg,items=[],refreshing=false,collapsed=false,fullscreenHidden=false,fullscreenRevision=0,hideTimer,dragId,menuItem=null,errorTimer,lastSignature='',sizeTimer;
let visibleSize={width:320,height:110};
const dockAudio=window.createDockAudio?.({element:$('dock'),invoke});
document.addEventListener('contextmenu',event=>event.preventDefault());
function error(e){$('error').textContent=String(e);$('error').hidden=false;clearTimeout(errorTimer);errorTimer=setTimeout(()=>$('error').hidden=true,5000)}
function config(c){
  cfg=c;dockAudio?.configure(c.audio,Boolean(c.enabled&&!collapsed&&!fullscreenHidden),c.position);document.body.className=[c.position,c.icon_style,c.indicator,collapsed?'collapsed':''].join(' ');
  const style=document.documentElement.style;
  for(const [name,value]of Object.entries({'--icon':c.icon_size+'px','--gap':c.spacing+'px','--padding':c.padding+'px','--radius':c.radius+'px','--alpha':c.opacity/100,'--inactive':c.inactive_opacity/100,'--indicator':c.indicator_color,'--zoom':c.magnification/100,'--room':Math.max(32,Math.ceil(c.icon_size*(c.magnification/100-1))+28)+'px','--active-fill':c.active_background?'rgba(255,255,255,.12)':'transparent'}))style.setProperty(name,value);
  style.setProperty('--tint',[1,3,5].map(i=>parseInt(c.tint.slice(i,i+2),16)).join(','));
  $('clock').hidden=!c.show_clock;for(const id of ['tray','language','network','volume'])$(id).hidden=!c['show_'+id];$('utilities').hidden=!c.show_clock&&!c.show_tray&&!c.show_language&&!c.show_network&&!c.show_volume;$('start').hidden=c.show_start===false;$('search').hidden=c.show_search===false;$('studio').hidden=c.show_studio===false;$('divider').hidden=$('utilities').hidden&&c.show_studio===false;
  if(!c.show_labels)$('label').hidden=true;
  if(!c.auto_hide&&collapsed){collapsed=false;document.body.classList.remove('collapsed');dockAudio?.configure(c.audio,Boolean(c.enabled&&!fullscreenHidden),c.position)}
  scheduleMeasure();
}
async function refresh(){
  if(!invoke||refreshing||dragId||menuItem)return;refreshing=true;const revision=fullscreenRevision;
  try{const snapshot=await invoke('get_dock_snapshot');if(revision===fullscreenRevision)fullscreenHidden=Boolean(snapshot.fullscreen_hidden);config(snapshot.config);items=snapshot.items;render();}
  catch(e){error(e)}finally{refreshing=false}
}
function render(){
  const visible=items.filter(i=>!i.hidden);const signature=JSON.stringify(visible.map(i=>[i.id,i.name,i.pinned,i.active,i.running,i.icon]));
  if(signature===lastSignature)return;lastSignature=signature;
  const current=new Map([...$('apps').children].map(e=>[e.dataset.id,e]));
  for(const [index,item] of visible.entries()){
    let button=current.get(item.id);if(!button){button=document.createElement('button');button.className='app';button.dataset.id=item.id;
      const image=document.createElement('img');image.alt='';image.draggable=false;
      image.addEventListener('error',()=>{image.hidden=true;button.querySelector('.fallback').hidden=false});
      const fallback=document.createElement('span');fallback.className='fallback';
      const icon=document.createElement('span');icon.className='icon';icon.append(image,fallback);
      const indicator=document.createElement('span');indicator.className='indicator';button.append(icon,indicator);
      button.addEventListener('click',()=>activate(button.dataset.id,false));
      button.addEventListener('pointerenter',()=>showLabel(button));button.addEventListener('pointerleave',()=>$('label').hidden=true);
      button.addEventListener('focus',()=>showLabel(button));button.addEventListener('blur',()=>$('label').hidden=true);
      button.addEventListener('contextmenu',e=>{e.preventDefault();openMenu(button.dataset.id)});
      button.addEventListener('dragstart',e=>{if(!button.draggable){e.preventDefault();return}dragId=button.dataset.id;button.classList.add('dragging');e.dataTransfer.setData('text/plain',dragId)});
      button.addEventListener('dragend',()=>{dragId=null;button.classList.remove('dragging');document.querySelectorAll('.drop-target').forEach(e=>e.classList.remove('drop-target'))});
      button.addEventListener('dragover',e=>{if(dragId&&button.draggable){e.preventDefault();button.classList.add('drop-target')}});
      button.addEventListener('dragleave',()=>button.classList.remove('drop-target'));
      button.addEventListener('drop',async e=>{e.preventDefault();button.classList.remove('drop-target');const ids=items.filter(i=>i.pinned).map(i=>i.id);const from=ids.indexOf(dragId),to=ids.indexOf(button.dataset.id);if(from<0||to<0||from===to)return;ids.splice(to,0,ids.splice(from,1)[0]);try{await invoke('dock_reorder',{ids});await refresh()}catch(e){error(e)}});
    }
    button.draggable=item.pinned;button.setAttribute('aria-label',item.name+(item.active?', active':item.running?', running':''));
    button.classList.toggle('active',item.active);button.classList.toggle('running',item.running);
    const image=button.querySelector('img');if(image.getAttribute('src')!==item.icon)image.src=item.icon;
    image.hidden=!item.icon;
    const fallback=button.querySelector('.fallback');fallback.hidden=!!item.icon;fallback.textContent=item.name[0]||'·';
    if($('apps').children[index]!==button)$('apps').insertBefore(button,$('apps').children[index]||null);current.delete(item.id);
  }
  for(const button of current.values())button.remove();scheduleMeasure();
}
async function activate(id,newInstance){
  const button=[...$('apps').children].find(b=>b.dataset.id===id),item=items.find(i=>i.id===id);if(cfg?.launch_animation!==false&&(newInstance||!item?.running)&&button){button.classList.remove('launching');void button.offsetWidth;button.classList.add('launching');clearTimeout(button.launchTimer);button.launchTimer=setTimeout(()=>button.classList.remove('launching'),550)}
  try{await invoke('dock_activate',{id,newInstance});setTimeout(refresh,250)}catch(e){error(e)}

}
function showLabel(button){
  if(!cfg?.show_labels||menuItem)return;const item=items.find(i=>i.id===button.dataset.id);if(!item)return;
  const label=$('label');label.textContent=item.name+(item.windows>1?' · '+item.windows+' windows':'');label.hidden=false;
  const r=button.getBoundingClientRect(),l=label.getBoundingClientRect();
  let x=r.x+(r.width-l.width)/2,y=r.y-l.height-8;
  if(cfg.position==='top')y=r.bottom+8;
  if(cfg.position==='left'){x=r.right+8;y=r.y+(r.height-l.height)/2}
  if(cfg.position==='right'){x=r.x-l.width-8;y=r.y+(r.height-l.height)/2}
  label.style.left=Math.max(2,Math.min(innerWidth-l.width-2,x))+'px';label.style.top=Math.max(2,Math.min(innerHeight-l.height-2,y))+'px';
}
function openMenu(id){
  const item=items.find(i=>i.id===id);if(!item)return;menuItem=item;clearTimeout(hideTimer);$('label').hidden=true;
  const menu=$('menu');menu.replaceChildren();const name=document.createElement('div');name.className='name';name.textContent=item.name;menu.append(name);
  const action=(label,callback)=>{const b=document.createElement('button');b.role='menuitem';b.textContent=label;b.addEventListener('click',async()=>{closeMenu();try{await callback()}catch(e){error(e)}});menu.append(b)};
  action(item.running?'Show window':'Open',()=>activate(id,false));action('Open new instance',()=>activate(id,true));
  action(item.pinned?'Unpin from dock':'Keep in dock',async()=>{await invoke('dock_pin',{id,pinned:!item.pinned});await refresh()});
  action('Dock settings',()=>invoke('dock_open_studio'));menu.hidden=false;
}
function closeMenu(){menuItem=null;$('menu').hidden=true}
document.addEventListener('keydown',e=>{if(e.key==='Escape')closeMenu()});
document.addEventListener('pointerdown',e=>{if(!$('menu').contains(e.target))closeMenu()});
function scheduleMeasure(){clearTimeout(sizeTimer);sizeTimer=setTimeout(measure,60)}
async function measure(){
  if(!cfg||!invoke||menuItem)return;
  const vertical=['left','right'].includes(cfg.position),room=parseFloat(getComputedStyle(document.documentElement).getPropertyValue('--room'));
  const count=items.filter(i=>!i.hidden).length;const appLength=count*cfg.icon_size+Math.max(0,count-1)*cfg.spacing+8;
  const utility=(cfg.show_studio!==false?cfg.icon_size*.65+cfg.spacing:0)+(cfg.show_clock?40:0)+(cfg.show_start!==false?cfg.icon_size+cfg.spacing:0)+(cfg.show_search!==false?cfg.icon_size+cfg.spacing:0)+(cfg.show_tray?28:0)+(cfg.show_language?34:0)+(cfg.show_network?28:0)+(cfg.show_volume?28:0)+cfg.spacing*2+10;
  const length=appLength+utility+cfg.padding*2+28;
  const thickness=cfg.icon_size+7+cfg.padding*2+room+14;
  visibleSize=vertical?{width:thickness,height:length}:{width:length,height:thickness};
  try{await invoke('dock_resize',{...visibleSize,collapsed})}catch(e){error(e)}
}
async function setCollapsed(value){
  if(!cfg?.auto_hide||menuItem||collapsed===value)return;collapsed=value;document.body.classList.toggle('collapsed',value);dockAudio?.configure(cfg.audio,Boolean(cfg.enabled&&!value&&!fullscreenHidden),cfg.position);$('label').hidden=true;
  try{await invoke('dock_resize',{...visibleSize,collapsed:value})}catch(e){error(e)}
}
document.documentElement.addEventListener('pointerenter',()=>{clearTimeout(hideTimer);setCollapsed(false)});
document.documentElement.addEventListener('pointerleave',()=>{closeMenu();clearTimeout(hideTimer);if(cfg?.auto_hide)hideTimer=setTimeout(()=>setCollapsed(true),cfg.hide_delay)});
$('apps').addEventListener('wheel',event=>{if(cfg&&!['left','right'].includes(cfg.position)&&$('apps').scrollWidth>$('apps').clientWidth){$('apps').scrollLeft+=event.deltaY||event.deltaX;event.preventDefault()}},{passive:false});
for(const action of ['start','search','tray','language','network','volume','clock'])$(action).addEventListener('click',()=>invoke('dock_open_system',{action}).catch(error));
$('studio').addEventListener('click',()=>invoke('dock_open_studio').catch(error));
setInterval(()=>{$('clock').textContent=new Date().toLocaleTimeString([],{hour:'2-digit',minute:'2-digit'})},1000);
async function refreshLanguage(){if(!invoke||!cfg?.show_language)return;try{$('language').textContent=await invoke('dock_input_language')}catch{}}
setInterval(refreshLanguage,500);$('language').addEventListener('pointerenter',refreshLanguage);window.addEventListener('focus',refreshLanguage);
window.__TAURI__?.event?.listen('dock-config-changed',()=>{lastSignature='';refresh()});
window.__TAURI__?.event?.listen('dock-fullscreen-changed',event=>{fullscreenRevision++;fullscreenHidden=Boolean(event.payload);if(cfg)dockAudio?.configure(cfg.audio,Boolean(cfg.enabled&&!collapsed&&!fullscreenHidden),cfg.position)});
refresh();setInterval(refresh,1600);
