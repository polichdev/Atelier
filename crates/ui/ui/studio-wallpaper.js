(()=>{
 const invoke=window.__TAURI__?.core?.invoke,$=id=>document.getElementById(id),results=document.querySelector('.wallpaper-results'),empty=$('wallpaper-empty'),status=$('wallpaper-notice'),search=$('wallpaper-search');
 let snapshot=null,tab='library',selected=null,busy=false,localWorkshopItems=[];
 const PAGE_SIZE=18,counts={library:PAGE_SIZE,workshop:PAGE_SIZE},grids={library:$('wallpaper-grid-library'),workshop:$('wallpaper-grid-workshop')},tiles={library:new Map(),workshop:new Map()};
 let scanPromise=null,lastScan=0,workshopLoaded=false,detailKey=null;
 const videoObserver=typeof IntersectionObserver==='function'?new IntersectionObserver(entries=>{for(const entry of entries)if(entry.isIntersecting){const video=entry.target;videoObserver.unobserve(video);if(video.dataset.src){video.src=video.dataset.src;delete video.dataset.src;video.load()}}},{root:results,rootMargin:'250px'}):null;
 const defaults={fit:'cover',volume:50,mute:true,paused:false,pause_fullscreen:true,pause_battery:false};
 let noticeTimer;function message(value,error=false){clearTimeout(noticeTimer);status.textContent=value;status.classList.toggle('error',error);status.hidden=!value;noticeTimer=setTimeout(()=>{status.hidden=true},error?7000:4000)}
 async function act(fn){if(busy)return;busy=true;try{await fn();await load()}catch(e){message(String(e),true)}finally{busy=false}}
 function openWorkshopBrowse(){const q=search.value.trim();act(()=>invoke('workshop_open_browse',{query:q||null}))}
 function preview(node,item){
  node.replaceChildren();const detail=node.id==='wallpaper-detail-preview';node.className='wallpaper-preview'+(detail?' wallpaper-detail-preview':'');node.onmouseenter=null;node.onmouseleave=null;
  let videoSource=null;
  if(item.kind==='video')videoSource=`http://wallpaper-media.localhost/${item.id}`;
  else if(item.kind==='workshop'&&item.steam.video_available)videoSource=`http://workshop-media.localhost/${item.steam.id}/video`;
  if(videoSource){
   const poster=!detail&&item.kind==='workshop'?item.steam.preview:null;
   const video=document.createElement('video');video.muted=true;video.loop=true;video.playsInline=true;video.preload='metadata';
   if(poster)video.poster=poster;
   if(detail){video.src=videoSource}else if(poster){video.preload='none';video.dataset.src=videoSource}else if(videoObserver){video.dataset.src=videoSource;videoObserver.observe(video)}else{video.src=videoSource}
   node.append(video);node.onmouseenter=()=>{if(video.dataset.src){video.src=video.dataset.src;delete video.dataset.src;videoObserver?.unobserve(video)}video.play().catch(()=>{})};node.onmouseleave=()=>{video.pause();if(Number.isFinite(video.duration))video.currentTime=0};return}
  let source=null,fallback=null;
  if(item.kind==='scene')source=`http://wallpaper-scene.localhost/${item.id}/${item.asset}`;
  else if(item.kind==='workshop'){const w=item.steam;fallback=w.preview||null;source=w.scene_available||w.local_type==='scene'?`http://workshop-media.localhost/${w.id}/cover`:fallback}
  if(source){const img=document.createElement('img');img.src=source;img.alt='';img.loading=detail?'eager':'lazy';img.decoding='async';img.referrerPolicy='no-referrer';if(fallback&&source!==fallback){img.style.backgroundImage='url('+JSON.stringify(fallback)+')';img.style.backgroundSize='cover';img.style.backgroundPosition='center';img.onerror=()=>{img.onerror=null;img.src=fallback}}node.append(img)}
 }
 function setDetail(open){const panel=$('wallpaper-detail');$('wallpaper-layout').classList.toggle('has-selection',open);panel.setAttribute('aria-hidden',String(!open));panel.inert=!open;if(open)$('wallpaper-detail-content').hidden=false}
 function select(id){selected=id;draw()}
 function draw(){if(!snapshot)return;const settingsTab=tab==='settings',workshop=tab==='workshop';$('wallpaper-performance').hidden=!settingsTab;document.querySelector('.wallpaper-results').hidden=settingsTab;document.querySelector('.wallpaper-toolbar').hidden=settingsTab;$('wallpaper-import').closest('label').hidden=settingsTab;
  if(settingsTab){$('workshop-actions').hidden=true;setDetail(false);window.AtelierSceneSettings.show(null,{},config=>{snapshot.config=config},message);window.AtelierWallpaperSettings.show(snapshot.config.performance,config=>{snapshot.config=config},message);return}
  $('workshop-actions').hidden=!workshop;search.placeholder=workshop?'Search Workshop wallpapers':'Search your wallpapers';
  const workshopList=localWorkshopItems.map(w=>({id:'steam:'+w.id,title:w.title,description:w.description,category:w.kind==='video'?'Video':w.kind==='scene'?'Scene':w.kind==='web'?'Web':'Workshop',kind:'workshop',steam:w}));
  const all=workshop?workshopList:snapshot.config.items.filter(i=>i.kind==='video'||i.kind==='scene').slice().reverse();
  const term=search.value.trim().toLocaleLowerCase(),items=all.filter(i=>i.title.toLocaleLowerCase().includes(term)),grid=grids[tab],cache=tiles[tab];
  grids.library.hidden=workshop;grids.workshop.hidden=!workshop;empty.hidden=items.length>0;
  if(!items.length){empty.replaceChildren();const text=document.createElement('p');text.textContent=term?'No wallpapers match your search.':workshop&&!workshopLoaded?'Scanning Steam wallpapers…':workshop?'Subscribe to wallpapers in Steam, then refresh your downloads here.':'Your library is ready for its first wallpaper. Import an MP4 or WebM video to get started.';empty.append(text);if(workshop&&workshopLoaded&&!term){const button=document.createElement('button');button.type='button';button.className='wallpaper-primary';button.textContent='Browse Workshop';button.onclick=openWorkshopBrowse;empty.append(button)}}
  const visible=items.slice(0,counts[tab]),wanted=new Set(visible.map(item=>item.id)),available=new Set(all.map(item=>item.id));
  for(const [id,entry] of cache){if(!available.has(id)){const video=entry.tile.querySelector('video');if(video)videoObserver?.unobserve(video);entry.tile.remove();cache.delete(id)}else if(!wanted.has(id)&&entry.tile.parentNode===grid)entry.tile.remove()}
  for(let index=0;index<visible.length;index++){const item=visible[index],key=JSON.stringify([item.title,item.kind,item.asset,item.steam?.preview,item.steam?.video_available,item.steam?.scene_available]);let entry=cache.get(item.id);
   if(!entry||entry.key!==key){if(entry){const video=entry.tile.querySelector('video');if(video)videoObserver?.unobserve(video);entry.tile.remove()}const tile=document.createElement('button');tile.type='button';tile.className='wallpaper-tile';tile.setAttribute('aria-label',item.title);tile.title=item.title;tile.onclick=()=>select(item.id);const image=document.createElement('div');preview(image,item);const body=document.createElement('div');body.className='wallpaper-tile-body';const title=document.createElement('strong');title.textContent=item.title;body.append(title);tile.append(image,body);entry={tile,key};cache.set(item.id,entry)}
   entry.tile.classList.toggle('selected',selected===item.id);entry.tile.setAttribute('aria-pressed',String(selected===item.id));if(grid.children[index]!==entry.tile)grid.insertBefore(entry.tile,grid.children[index]||null);
  }
  $('wallpaper-more').hidden=visible.length>=items.length;
  const item=all.find(i=>i.id===selected);setDetail(!!item);window.AtelierSceneSettings.show(item,snapshot.config.settings?.[item?.id]||defaults,config=>{snapshot.config=config},message);if(!item)return;const isSteam=item.kind==='workshop';const saved=!isSteam&&snapshot.config.items.some(i=>i.id===item.id),active=!isSteam&&snapshot.config.active===item.id,options=snapshot.config.settings?.[item.id]||defaults;
  const nextDetailKey=JSON.stringify([item.id,item.kind,item.asset,item.steam?.video_available,item.steam?.scene_available,item.steam?.preview]);if(detailKey!==nextDetailKey){preview($('wallpaper-detail-preview'),item);detailKey=nextDetailKey}$('wallpaper-detail-title').textContent=item.title;$('wallpaper-detail-meta').textContent=isSteam?`Steam Workshop · ${item.steam.kind||item.steam.local_type||'Wallpaper'}`:`${active?(snapshot.playback==='error'?'Playback failed · ':'Active · '):''}${item.kind==='scene'?'2D scene':item.category.toUpperCase()+' video'}`;
  if(isSteam){
    const w=item.steam;
    const isDownloaded=Boolean(w.is_installed||w.video_available||w.scene_available||w.web_available||w.local_type);
    const canImport=Boolean(w.video_available||w.scene_available||w.local_type==='scene');
    $('workshop-open').hidden=!canImport;
    $('workshop-open').textContent=w.video_available?'Import video to library':'Import scene to library';
    $('workshop-client').hidden=false;
    $('workshop-page').hidden=false;
    $('workshop-detail').hidden=false;
    $('wallpaper-apply').hidden=!canImport;
    if(canImport){
      $('wallpaper-apply').disabled=false;
      $('wallpaper-apply').textContent='Apply to Desktop';
    }
    $('wallpaper-stop').hidden=true;
    $('wallpaper-remove').hidden=true;
    $('wallpaper-options').hidden=true;
    let detailText='';
    if(w.video_available){
      detailText='Downloaded video detected. Click “Apply to Desktop” to play immediately, or “Import video to library” to add it.';
    }else if(w.kind==='scene'||w.local_type==='scene'||w.scene_available){
      detailText='2D scene playback is available. Advanced shaders, particles and SceneScript may differ from Wallpaper Engine.';
    }else if(w.kind==='web'||w.web_available){
      detailText='Web/HTML5 wallpaper detected.';
    }else if(!isDownloaded){
      detailText='Subscribe in Steam and refresh downloads to access this wallpaper.';
    }else{
      detailText=`Installed format: ${w.kind||w.local_type||'Unknown'}. Atelier currently supports MP4 and WebM video items.`;
    }
    $('workshop-detail').textContent=detailText;
    return;
  }
  $('workshop-open').hidden=true;
  $('workshop-client').hidden=true;
  $('workshop-page').hidden=true;
  $('workshop-detail').hidden=item.kind!=='scene';
  if(item.kind==='scene')$('workshop-detail').textContent='Scene playback. Some custom shaders, particle systems and scripts are not supported yet.';
  $('wallpaper-apply').hidden=!saved;
  $('wallpaper-apply').disabled=active&&snapshot.playback!=='error';
  $('wallpaper-apply').textContent=active?(snapshot.playback==='error'?'Retry':snapshot.playback==='loading'?'Starting…':'Applied'):'Apply';
  $('wallpaper-stop').hidden=!active;
  $('wallpaper-remove').hidden=!saved;
  $('wallpaper-options').hidden=!saved;
  $('wallpaper-fit').value=options.fit;$('wallpaper-volume').value=options.volume;$('wallpaper-mute').checked=options.mute;$('wallpaper-paused').checked=options.paused;$('wallpaper-fullscreen').checked=options.pause_fullscreen;$('wallpaper-battery').checked=options.pause_battery;const audio=options.audio||{pulse:false,glow:false,pulse_strength:40,glow_strength:40};$('wallpaper-audio-pulse').checked=audio.pulse;$('wallpaper-audio-glow').checked=audio.glow;$('wallpaper-audio-pulse-strength').value=audio.pulse_strength;$('wallpaper-audio-glow-strength').value=audio.glow_strength;
 }
 function importWorkshop(id){const item=localWorkshopItems.find(w=>w.id===id);return invoke(item?.kind==='scene'||item?.local_type==='scene'?'workshop_import_scene':'workshop_import_video',{id})}
 async function scanWorkshop(force=false){if(!invoke)return;if(scanPromise)return scanPromise;if(!force&&workshopLoaded&&Date.now()-lastScan<15000)return true;scanPromise=(async()=>{try{localWorkshopItems=await invoke('workshop_list_local');lastScan=Date.now();workshopLoaded=true;return true}catch(e){console.warn('Workshop scan error:',e);workshopLoaded=true;message('Could not scan Steam wallpapers.',true);return false}finally{scanPromise=null}})();return scanPromise}
 async function load(){snapshot=await invoke('wallpaper_snapshot');if(snapshot.playback==='error'&&snapshot.error)message(snapshot.error,true);draw()}
 async function saveOptions(id){await invoke('wallpaper_options',{id,fit:$('wallpaper-fit').value,volume:Number($('wallpaper-volume').value),mute:$('wallpaper-mute').checked,paused:$('wallpaper-paused').checked,pauseFullscreen:$('wallpaper-fullscreen').checked,pauseBattery:$('wallpaper-battery').checked})}
 function paintTabs(){document.querySelectorAll('[data-wallpaper-tab]').forEach(button=>{const active=button.dataset.wallpaperTab===tab;button.classList.toggle('active',active);button.setAttribute('aria-selected',String(active));button.tabIndex=active?0:-1})}
 async function switchTab(next){if(tab===next)return;for(const video of results.querySelectorAll('video'))video.pause();tab=next;selected=null;search.value='';results.scrollTop=0;paintTabs();draw();if(next==='workshop'){await scanWorkshop();if(tab==='workshop')draw()}}
 for(const button of document.querySelectorAll('[data-wallpaper-tab]')){button.onclick=()=>switchTab(button.dataset.wallpaperTab);button.onkeydown=event=>{if(!['ArrowLeft','ArrowRight','Home','End'].includes(event.key))return;event.preventDefault();const tabs=['library','workshop','settings'],next=event.key==='Home'?tabs[0]:event.key==='End'?tabs[2]:tabs[(tabs.indexOf(tab)+(event.key==='ArrowRight'?1:2))%3];switchTab(next);document.querySelector('[data-wallpaper-tab="'+next+'"]').focus()}}
 search.oninput=()=>{counts[tab]=PAGE_SIZE;results.scrollTop=0;draw()};$('wallpaper-more-button').onclick=()=>{counts[tab]+=PAGE_SIZE;draw()};results.addEventListener('scroll',()=>{if(!$('wallpaper-more').hidden&&results.scrollTop+results.clientHeight>=results.scrollHeight-400){counts[tab]+=PAGE_SIZE;draw()}},{passive:true});$('wallpaper-detail-close').onclick=()=>{selected=null;draw()};
 document.addEventListener('keydown',event=>{if(event.key==='Escape'&&selected){selected=null;draw()}});
 $('workshop-browse').onclick=openWorkshopBrowse;
 $('workshop-refresh').onclick=()=>act(async()=>{message('Scanning Steam libraries…');if(await scanWorkshop(true))message(`Found ${localWorkshopItems.length} Steam wallpapers`)});
 $('workshop-open').onclick=()=>{const all=localWorkshopItems.map(w=>({id:'steam:'+w.id,steam:w}));const item=all.find(i=>i.id===selected);if(!item?.steam?.id)return;const id=item.steam.id;act(async()=>{const imported=await importWorkshop(id);tab='library';selected=imported.id;paintTabs();message(`“${imported.title}” imported into your library`)})};
 $('workshop-client').onclick=()=>{const all=localWorkshopItems.map(w=>({id:'steam:'+w.id,steam:w}));const item=all.find(i=>i.id===selected);if(item?.steam?.id)act(()=>invoke('workshop_open_client',{id:item.steam.id}))};
 $('workshop-page').onclick=()=>{const all=localWorkshopItems.map(w=>({id:'steam:'+w.id,steam:w}));const item=all.find(i=>i.id===selected);if(item?.steam?.id)act(()=>invoke('workshop_open_page',{id:item.steam.id}))};
 let audioSaveTimers={},audioSaveQueue=Promise.resolve();
 function saveAudioOptions(){const id=selected;if(!id||id.startsWith('steam:'))return;const options={pulse:$('wallpaper-audio-pulse').checked,glow:$('wallpaper-audio-glow').checked,pulse_strength:Number($('wallpaper-audio-pulse-strength').value),glow_strength:Number($('wallpaper-audio-glow-strength').value)};clearTimeout(audioSaveTimers[id]);audioSaveTimers[id]=setTimeout(()=>{delete audioSaveTimers[id];audioSaveQueue=audioSaveQueue.catch(()=>{}).then(()=>invoke('wallpaper_audio_options',{id,options})).then(config=>{snapshot.config=config}).catch(error=>message(String(error),true))},100)}
 for(const id of ['wallpaper-audio-pulse','wallpaper-audio-glow'])$(id).onchange=saveAudioOptions;
 for(const id of ['wallpaper-audio-pulse-strength','wallpaper-audio-glow-strength'])$(id).oninput=saveAudioOptions;
 for(const id of ['wallpaper-fit','wallpaper-volume','wallpaper-mute','wallpaper-paused','wallpaper-fullscreen','wallpaper-battery'])$(id).onchange=()=>{const id=selected;if(id)act(()=>saveOptions(id))};
 $('wallpaper-apply').onclick=()=>{const id=selected;if(id){if(id.startsWith('steam:')){const steamId=id.replace('steam:','');act(async()=>{message('Importing and applying wallpaper to desktop…');const imported=await importWorkshop(steamId);await invoke('wallpaper_apply',{id:imported.id});tab='library';selected=imported.id;paintTabs();message(`“${imported.title}” applied to desktop!`);draw()});return}act(async()=>{message('Starting wallpaper…');await invoke('wallpaper_apply',{id})})}};
 $('wallpaper-stop').onclick=()=>act(async()=>{await invoke('wallpaper_apply',{id:null});message('Wallpaper removed from desktop')});
 $('wallpaper-remove').onclick=()=>{const id=selected,item=snapshot?.config.items.find(i=>i.id===id);if(id&&item)act(async()=>{if(!window.confirm(`Remove “${item.title}” from your library?`))return;await invoke('wallpaper_remove',{id});selected=null;message('Removed from library')})};
 $('wallpaper-import').onchange=async e=>{const input=e.target,file=input.files[0];if(!file||busy)return;busy=true;input.disabled=true;try{const id=await invoke('wallpaper_import_begin',{name:file.name,size:file.size}),chunkSize=256*1024;for(let offset=0;offset<file.size;offset+=chunkSize){const data=await new Promise((resolve,reject)=>{const reader=new FileReader();reader.onerror=()=>reject(Error('Could not read video'));reader.onload=()=>resolve(String(reader.result).split(',')[1]);reader.readAsDataURL(file.slice(offset,offset+chunkSize))});await invoke('wallpaper_import_chunk',{id,offset,data});message(`Importing ${Math.round(Math.min(file.size,offset+chunkSize)*100/file.size)}%`)}const item=await invoke('wallpaper_import_finish',{id});message('Video added to your library');tab='library';paintTabs();selected=item.id;await load()}catch(error){message(String(error),true)}finally{busy=false;input.disabled=false;input.value=''}};
 window.__TAURI__?.event?.listen('wallpaper-status',event=>{if(snapshot){snapshot.playback=event.payload;if(snapshot.config.active===selected){$('wallpaper-apply').disabled=event.payload!=='error';$('wallpaper-apply').textContent=event.payload==='error'?'Retry':event.payload==='loading'?'Starting…':'Applied'}}if(event.payload==='error')message('The wallpaper could not play. Try importing it again or choose another wallpaper.',true)});
 let refreshing=false;async function refreshVisibleWorkshop(){if(tab!=='workshop'||busy||refreshing||document.hidden||!document.getElementById('wallpaper').classList.contains('active'))return;refreshing=true;try{const old=JSON.stringify(localWorkshopItems);await scanWorkshop();if(old!==JSON.stringify(localWorkshopItems))draw()}finally{refreshing=false}}
 window.addEventListener('focus',refreshVisibleWorkshop);setInterval(refreshVisibleWorkshop,10000);
 window.__TAURI__?.event?.listen('wallpaper-audio-error',event=>{if(event.payload)message('Audio reactivity: '+String(event.payload),true)});
 window.__TAURI__?.event?.listen('wallpaper-error',event=>{if(event.payload)message(String(event.payload),true)});
 if(invoke)load().catch(e=>message(String(e),true));else message('Launch Atelier to manage live wallpapers.');
})();
