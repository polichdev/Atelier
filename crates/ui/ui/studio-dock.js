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
  function renderApps(){
    const list=$('dd-apps');list.replaceChildren();
    if(!items.length){const p=document.createElement('p');p.style.color='var(--muted)';p.textContent='Enable the dock to import pinned apps, or open an app to pin it.';list.append(p);return;}
    const pinned=items.filter(i=>i.pinned);
    for(const item of items){
      const row=document.createElement('div');row.className='dd-app';
      const icon=document.createElement('img');icon.src=item.icon;icon.alt='';if(!item.icon)icon.hidden=true;
      const name=document.createElement('span');name.className='dd-app-name';name.textContent=item.name;name.title=item.name;
      const state=document.createElement('small');state.textContent=item.active?'Active':item.running?'Running':'Pinned';
      row.append(icon,name,state);
      const button=(text,label,action)=>{const b=document.createElement('button');b.type='button';b.textContent=text;b.setAttribute('aria-label',label);b.addEventListener('click',async()=>{b.disabled=true;try{await action();await refresh()}catch(e){toast(String(e),true)}finally{b.disabled=false}});row.append(b);return b};
      if(item.pinned){const at=pinned.findIndex(i=>i.id===item.id);for(const [text,delta]of [['↑',-1],['↓',1]]){const b=button(text,`Move ${item.name} ${delta<0?'earlier':'later'}`,async()=>{const ids=pinned.map(i=>i.id);const target=at+delta;[ids[at],ids[target]]=[ids[target],ids[at]];await invoke('dock_reorder',{ids})});b.disabled=at+delta<0||at+delta>=pinned.length;}
        const picker=document.createElement('input');picker.type='file';picker.accept='image/png,image/jpeg,image/webp';picker.hidden=true;picker.setAttribute('aria-label','Choose icon for '+item.name);row.append(picker);
        const choose=document.createElement('button');choose.type='button';choose.textContent='Icon';choose.setAttribute('aria-label','Change icon for '+item.name);choose.addEventListener('click',()=>picker.click());row.append(choose);
        picker.addEventListener('change',async()=>{const file=picker.files[0];if(!file)return;try{if(file.size>5*1024*1024)throw Error('Choose an image smaller than 5 MB');const bitmap=await createImageBitmap(file);const canvas=document.createElement('canvas');canvas.width=canvas.height=128;const ctx=canvas.getContext('2d');const ratio=Math.min(128/bitmap.width,128/bitmap.height);const w=bitmap.width*ratio,h=bitmap.height*ratio;ctx.drawImage(bitmap,(128-w)/2,(128-h)/2,w,h);bitmap.close();await invoke('dock_set_icon',{id:item.id,icon:canvas.toDataURL('image/png')});await refresh()}catch(e){toast(String(e),true)}});
        if(saved.pinned.some(p=>p.path===item.id&&p.icon))button('Reset icon','Reset icon for '+item.name,()=>invoke('dock_set_icon',{id:item.id,icon:null}));
      }
      button(item.pinned?'Unpin':'Pin',(item.pinned?'Unpin ':'Pin ')+item.name,()=>invoke('dock_pin',{id:item.id,pinned:!item.pinned}));list.append(row);
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
