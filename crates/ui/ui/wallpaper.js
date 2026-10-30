const invoke=window.__TAURI__?.core?.invoke;
const video=document.getElementById('video'),scene=document.getElementById('scene');
let config=null,current=null,lastStatus='',player=null,pendingPlayer=null,changing=false,generation=0,syncing=false,appliedOptions='';
function report(status,detail=null){if(status!==lastStatus){lastStatus=status;invoke('wallpaper_report',{status,detail:detail?String(detail):null}).catch(()=>{})}}
function failed(error){console.error('Wallpaper playback failed',error);generation++;changing=false;current=null;pendingPlayer?.dispose();pendingPlayer=null;player?.dispose();player=null;video.pause();report('error',error)}
function options(){return {...(config?.settings?.[config.active]||{fit:'cover',mute:true,volume:50,paused:false}),performance:config?.performance}}
async function update(next){
 config=next;const item=next.items.find(x=>x.id===next.active),source=item?item.kind+':'+item.id:null;
 if(source!==current){
  const version=++generation;current=source;changing=true;appliedOptions='';pendingPlayer?.dispose();pendingPlayer=null;
  if(!item){video.pause();video.removeAttribute('src');video.load();player?.dispose();player=null;scene.replaceChildren();scene.style.display='none';video.style.display='none';changing=false;report('stopped');return}
  report('loading');
  if(item.kind==='scene'){
   try{const loading=new window.AtelierScenePlayer(document.createElement('div'));pendingPlayer=loading;loading.onError=error=>{if(version===generation)failed(error)};await loading.load(item.id);if(version!==generation){loading.dispose();return}player?.dispose();video.pause();video.removeAttribute('src');video.load();scene.replaceChildren(loading.canvas);scene.className='scene native';scene.style.display='block';video.style.display='none';player=loading;pendingPlayer=null}
   catch(error){if(version!==generation)return;failed(error);return}
  }else if(item.kind==='video'||item.kind==='preset-video'){
   player?.dispose();player=null;scene.replaceChildren();scene.style.display='none';video.style.display='block';video.src=item.kind==='video'?`http://wallpaper-media.localhost/${item.id}`:`wallpaper-presets/${encodeURIComponent(item.asset)}`;video.load();
  }else{player?.dispose();player=null;video.pause();video.style.display='none';scene.className='scene '+item.id.replace('preset-','');scene.style.display='block'}
  changing=false;
 }
 const settings=options();video.style.objectFit=settings.fit==='stretch'?'fill':settings.fit;
 await sync();
}
async function sync(){
 if(!config?.active||syncing||changing)return;syncing=true;
 try{
  let runtime;try{runtime=await invoke('wallpaper_playback_state')}catch{runtime={paused:options().paused,muted:false,reason:options().paused?'paused':null}}const reason=runtime.reason,settings={...options(),mute:options().mute||runtime.muted};video.muted=settings.mute;video.volume=settings.volume/100;
  if(!current)return;scene.classList.toggle('paused',Boolean(reason));
  if(player&&!player.ready)return;
  if(player){const key=JSON.stringify([settings,runtime.paused]);if(key!==appliedOptions){player.setOptions(settings,runtime.paused);appliedOptions=key}report(reason||'playing')}
  else if(current.startsWith('video:')||current.startsWith('preset-video:')){
   if(reason){video.pause();report(reason)}else{try{await video.play();report('playing')}catch{report('error')}}
  }else report(reason||'playing');
 }catch(error){failed(error)}finally{syncing=false}
}
video.addEventListener('error',()=>{if(current?.startsWith('video:')||current?.startsWith('preset-video:'))report('error',video.error?.message||'The video could not be decoded')});
window.__TAURI__?.event?.listen('wallpaper-changed',event=>update(event.payload));
window.__TAURI__?.event?.listen('wallpaper-scene-updated',event=>{if(config?.active===event.payload){current=null;update(config)}});
invoke('wallpaper_snapshot').then(snapshot=>update(snapshot.config)).catch(()=>report('error'));
setInterval(sync,1000);
let repairing=false;setInterval(async()=>{if(!config?.active||repairing)return;repairing=true;try{await invoke('wallpaper_repair')}catch(error){console.error('Wallpaper desktop refresh failed',error)}finally{repairing=false}},10000);
if(window.__ATELIER_RUNTIME_TEST__)import('./wallpaper-runtime-test.js').then(test=>test.run(()=>({player,current,lastStatus,config})));
