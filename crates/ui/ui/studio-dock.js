(()=>{
  'use strict';
  const $=id=>document.getElementById(id),invoke=window.__TAURI__?.core?.invoke;
  const root=$('desktop-dock-settings');
  const fields=[...root.querySelectorAll('[id^="dd-"]')].filter(e=>['INPUT','SELECT'].includes(e.tagName));
  let saved=null,items=[],busy=false,queued=false,timer=null,refreshing=false,revision=0;
  function outputs(){for(const input of fields.filter(e=>e.type==='range')){const name=input.id.slice(3),suffix=['opacity','inactive_opacity','magnification'].includes(name)?'%':name==='hide_delay'?' ms':' px';input.nextElementSibling.textContent=input.value+suffix;}}
  function paint(c){saved=c;for(const field of fields){const value=c[field.id.slice(3)];if(field.type==='checkbox')field.checked=!!value;else field.value=value;}outputs();$('dd-status').textContent=c.enabled?'Desktop dock is on':'Desktop dock is off';}
  function read(){const c={...saved};for(const field of fields){const name=field.id.slice(3);c[name]=field.type==='checkbox'?field.checked:['range','number'].includes(field.type)||name==='monitor'?Number(field.value):field.value;}c.hide_windows_taskbar=c.enabled;return c;}
  async function refresh(){
    if(!invoke||refreshing||busy||timer)return;refreshing=true;const startRevision=revision;
    try{const data=await invoke('get_dock_snapshot');if(startRevision!==revision||busy||timer)return;const selected=data.config.monitor;$('dd-monitor').replaceChildren(...data.monitors.map((name,i)=>{const o=document.createElement('option');o.value=i;const match=name.match(/^\\\\\.\\DISPLAY(\d+)$/i);o.textContent=match?'Display '+match[1]:name;return o}));$('dd-monitor').value=selected;paint(data.config);items=data.items;renderApps();}
    catch(e){$('dd-status').textContent=String(e)}finally{refreshing=false}
  }
  function eyeSvg(hidden){return hidden?'<svg viewBox="0 0 24 24" width="17" height="17" fill="none" stroke="currentColor" stroke-width="1.8" aria-hidden="true"><path d="M3 3l18 18M10.6 10.6a2 2 0 0 0 2.8 2.8M9.8 5.3A11.4 11.4 0 0 1 12 5c5 0 8.5 4.3 9 7-.3 1.2-1.1 2.5-2.2 3.6M6.1 6.1C4.3 7.4 3.3 9.3 3 12c.5 2.7 4 7 9 7 1.2 0 2.4-.3 3.4-.8"/></svg>':'<svg viewBox="0 0 24 24" width="17" height="17" fill="none" stroke="currentColor" stroke-width="1.8" aria-hidden="true"><path d="M3 12c.5-2.7 4-7 9-7s8.5 4.3 9 7c-.5 2.7-4 7-9 7s-8.5-4.3-9-7Z"/><circle cx="12" cy="12" r="3"/></svg>'}
  function renderApps(){
    const list=$('dd-apps');list.replaceChildren();
    const pinned=items.filter(i=>i.pinned);
    for(const [key,label,icon] of [['show_start','Windows Start','<path d="M3 3h8v8H3zm10 0h8v8h-8zM3 13h8v8H3zm10 0h8v8h-8z" fill="currentColor" stroke="none"/>'],['show_search','Windows Search','<circle cx="10.5" cy="10.5" r="6.5"/><path d="m15.5 15.5 5 5"/>'],['show_tray','Hidden tray icons','<path d="m6 14 6-6 6 6"/>'],['show_language','Input language','<path d="M4 19l4-14 4 14M5 15h6m2-7h8m-4-3v3m-2 3c1 4 3 6 6 8m-1-8c-1 4-3 6-6 8"/>'],['show_network','Network','<rect x="3" y="4" width="18" height="13" rx="1"/><path d="M8 21h8m-4-4v4"/>'],['show_volume','Volume','<path d="M3 9h4l5-4v14l-5-4H3zm12-1a6 6 0 0 1 0 8"/>'],['show_clock','Clock and notifications','<circle cx="12" cy="12" r="9"/><path d="M12 7v5l3 2"/>'],['show_studio','Atelier','<rect x="3" y="3" width="18" height="18" rx="5"/><path d="m7 17 5-10 5 10m-8-3h6"/>']]){const row=document.createElement('div');row.className='dd-app';const image=document.createElement('span');image.innerHTML='<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">'+icon+'</svg>';const name=document.createElement('span');name.className='dd-app-name';name.textContent=label;const state=document.createElement('small');state.textContent=saved[key]?'Visible':'Hidden';const eye=document.createElement('button');eye.type='button';eye.setAttribute('aria-label',(saved[key]?'Hide ':'Show ')+label);eye.innerHTML=eyeSvg(!saved[key]);eye.setAttribute('aria-pressed',String(!!saved[key]));eye.onclick=()=>{const input=$('dd-'+key);input.checked=!input.checked;revision++;save()};row.append(image,name,state,eye);list.append(row)}
    for(const item of items){
      const row=document.createElement('div');row.className='dd-app';
      const icon=document.createElement('img');icon.src=item.icon;icon.alt='';if(!item.icon)icon.hidden=true;
      const name=document.createElement('span');name.className='dd-app-name';name.textContent=item.name;name.title=item.name;
      const state=document.createElement('small');state.textContent=item.hidden?'Hidden':item.active?'Active':item.running?'Running':item.pinned?'Pinned':'Unavailable';
      row.append(icon,name,state);
      const button=(text,label,action)=>{const b=document.createElement('button');b.type='button';b.textContent=text;b.setAttribute('aria-label',label);b.addEventListener('click',async()=>{b.disabled=true;try{await action();await refresh()}catch(e){toast(String(e),true)}finally{b.disabled=false}});row.append(b);return b};
      const visibility=button('',(item.hidden?'Show ':'Hide ')+item.name,()=>invoke('dock_set_hidden',{id:item.id,hidden:!item.hidden}));visibility.innerHTML=eyeSvg(item.hidden);visibility.setAttribute('aria-pressed',String(!item.hidden));
      if(item.pinned){const at=pinned.findIndex(i=>i.id===item.id);for(const [text,delta]of [['↑',-1],['↓',1]]){const b=button(text,`Move ${item.name} ${delta<0?'earlier':'later'}`,async()=>{const ids=pinned.map(i=>i.id);const target=at+delta;[ids[at],ids[target]]=[ids[target],ids[at]];await invoke('dock_reorder',{ids})});b.disabled=at+delta<0||at+delta>=pinned.length;}
        const picker=document.createElement('input');picker.type='file';picker.accept='image/png,image/jpeg,image/webp';picker.hidden=true;picker.setAttribute('aria-label','Choose icon for '+item.name);row.append(picker);
        const choose=document.createElement('button');choose.type='button';choose.textContent='Icon';choose.setAttribute('aria-label','Change icon for '+item.name);choose.addEventListener('click',()=>picker.click());row.append(choose);
        picker.addEventListener('change',async()=>{const file=picker.files[0];if(!file)return;try{if(file.size>5*1024*1024)throw Error('Choose an image smaller than 5 MB');const bitmap=await createImageBitmap(file);const canvas=document.createElement('canvas');canvas.width=canvas.height=256;const ctx=canvas.getContext('2d');const ratio=Math.min(256/bitmap.width,256/bitmap.height);const w=bitmap.width*ratio,h=bitmap.height*ratio;ctx.drawImage(bitmap,(256-w)/2,(256-h)/2,w,h);bitmap.close();await invoke('dock_set_icon',{id:item.id,icon:canvas.toDataURL('image/png')});await refresh()}catch(e){toast(String(e),true)}});
        if(saved.pinned.some(p=>p.path===item.id&&p.icon))button('Reset icon','Reset icon for '+item.name,()=>invoke('dock_set_icon',{id:item.id,icon:null}));
      }
      if(item.pinned||item.running)button(item.pinned?'Unpin':'Pin',(item.pinned?'Unpin ':'Pin ')+item.name,()=>invoke('dock_pin',{id:item.id,pinned:!item.pinned}));list.append(row);
    }
  }
  async function save(){
    clearTimeout(timer);timer=null;if(!invoke||!saved)return;
    if(busy){queued=true;return;}busy=true;const next=read();$('dd-status').textContent='Applying…';
    try{const c=await invoke('set_dock_config',{config:next});const changed=JSON.stringify(read())!==JSON.stringify(next);saved=c;if(!queued&&!changed)paint(c);else if(changed)queued=true;}
    catch(e){paint(saved);toast('Could not update dock: '+e,true)}
    finally{busy=false;if(queued){queued=false;save()}else refresh()}
  }
  for(const field of fields){field.addEventListener('input',()=>{revision++;outputs();if(field.type==='range'){clearTimeout(timer);timer=setTimeout(save,180)}});field.addEventListener('change',()=>{revision++;save()});}
  function preset(values){if(!saved)return;revision++;const previous=saved;paint({...read(),...values,enabled:true});saved=previous;save()}
  $('dd-preset-mac').addEventListener('click',()=>preset({position:'bottom',alignment:'center',icon_size:44,spacing:6,padding:10,edge_gap:10,radius:18,opacity:72,tint:'#232532',magnification:140,icon_style:'original',inactive_opacity:90,active_background:false,indicator:'dot',indicator_color:'#f3f0ea',show_labels:true,show_running:true,show_clock:false}));
  $('dd-preset-compact').addEventListener('click',()=>preset({icon_size:24,spacing:2,padding:5,edge_gap:5,radius:12,magnification:125,show_clock:false}));
  $('dd-preset-sidebar').addEventListener('click',()=>preset({position:'left',alignment:'center',icon_size:36,spacing:5,padding:8,edge_gap:8,radius:16,magnification:125,show_clock:false}));
  if(!invoke){fields.forEach(e=>e.disabled=true);root.querySelectorAll('button').forEach(e=>e.disabled=true);$('dd-status').textContent='Launch Atelier to use the desktop dock';return}
  window.__TAURI__?.event?.listen('dock-config-changed',()=>{if(!busy&&!timer)refresh()});
  refresh();setInterval(()=>{if(document.getElementById('taskbar').classList.contains('active')&&$('desktop-dock-details').open)refresh()},4000);
})();
