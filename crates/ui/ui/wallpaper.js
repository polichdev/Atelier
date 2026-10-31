const invoke=window.__TAURI__?.core?.invoke;
const video=document.getElementById('video'),scene=document.getElementById('scene');
const loopFrame=document.getElementById('loop-frame');
let config=null,current=null,lastStatus='',player=null,pendingPlayer=null,changing=false,generation=0,syncing=false,appliedOptions='';
let loopFrameReady=false,loopFrameVersion=0,loopFrameFading=false;
function clearLoopFrame(){loopFrameVersion++;loopFrameReady=false;loopFrameFading=false;if(loopFrame){loopFrame.style.display='none';loopFrame.style.opacity='1'}}
function captureLoopFrame(){
 if(!loopFrame||!current?.startsWith('video:')&&!current?.startsWith('preset-video:')||!Number.isFinite(video.duration)||video.duration<=0||video.currentTime<Math.max(0,video.duration-1)||video.readyState<2||!video.videoWidth||!video.videoHeight)return;
 try{const scale=Math.min(1,1920/video.videoWidth,1080/video.videoHeight),width=Math.max(1,Math.round(video.videoWidth*scale)),height=Math.max(1,Math.round(video.videoHeight*scale));if(loopFrame.width!==width||loopFrame.height!==height){loopFrame.width=width;loopFrame.height=height}const context=loopFrame.getContext('2d');if(!context)return;context.drawImage(video,0,0,width,height);loopFrameReady=true}catch{loopFrameReady=false}
}
function releaseLoopFrame(){
 if(!loopFrame||loopFrame.style.display==='none')return;const version=loopFrameVersion;
 const release=()=>{if(version===loopFrameVersion&&video.readyState>=2&&!video.seeking&&!loopFrameFading){loopFrameFading=true;loopFrameReady=false;loopFrame.style.opacity='0';setTimeout(()=>{if(version===loopFrameVersion){loopFrame.style.display='none';loopFrameFading=false}},250)}};
 if(video.seeking||video.readyState<2)return;
 if(typeof video.requestVideoFrameCallback==='function')video.requestVideoFrameCallback(release);else if(typeof requestAnimationFrame==='function')requestAnimationFrame(release);
}
if(typeof video.requestVideoFrameCallback==='function'){
 const capture=()=>{captureLoopFrame();video.requestVideoFrameCallback(capture)};
 video.requestVideoFrameCallback(capture);
}else video.addEventListener('timeupdate',captureLoopFrame);
video.addEventListener('seeking',()=>{if(loopFrame&&loopFrameReady&&video.currentTime<1){loopFrameVersion++;loopFrameFading=false;loopFrame.style.opacity='1';loopFrame.style.display='block'}});
video.addEventListener('seeked',releaseLoopFrame);
video.addEventListener('playing',releaseLoopFrame);
const audioEffects=window.createWallpaperAudioEffects?.({video,scene,glow:document.getElementById('audio-glow'),invoke});
function report(status,detail=null){if(status!==lastStatus){lastStatus=status;invoke('wallpaper_report',{status,detail:detail?String(detail):null}).catch(()=>{})}}
function failed(error){audioEffects?.configure(null,true);console.error('Wallpaper playback failed',error);generation++;changing=false;current=null;pendingPlayer?.dispose();pendingPlayer=null;player?.dispose();player=null;video.pause();report('error',error)}
function options(){return {...(config?.settings?.[config.active]||{fit:'cover',mute:true,volume:50,paused:false}),performance:config?.performance}}
async function update(next){
 config=next;const item=next.items.find(x=>x.id===next.active),source=item?item.kind+':'+item.id:null;
 if(source!==current){
  const version=++generation;current=source;changing=true;appliedOptions='';clearLoopFrame();pendingPlayer?.dispose();pendingPlayer=null;
  if(!item){audioEffects?.configure(null,true);video.pause();video.removeAttribute('src');video.load();player?.dispose();player=null;scene.replaceChildren();scene.style.display='none';video.style.display='none';changing=false;report('stopped');return}
  audioEffects?.configure(null,true);report('loading');
  if(item.kind==='scene'){
   try{const loading=new window.AtelierScenePlayer(document.createElement('div'));pendingPlayer=loading;loading.onError=error=>{if(version===generation)failed(error)};await loading.load(item.id);if(version!==generation){loading.dispose();return}player?.dispose();video.pause();video.removeAttribute('src');video.load();scene.replaceChildren(loading.canvas);scene.className='scene native';scene.style.display='block';video.style.display='none';player=loading;pendingPlayer=null}
   catch(error){if(version!==generation)return;failed(error);return}
  }else if(item.kind==='video'||item.kind==='preset-video'){
   player?.dispose();player=null;scene.replaceChildren();scene.style.display='none';video.style.display='block';video.src=item.kind==='video'?`http://wallpaper-media.localhost/${item.id}`:`wallpaper-presets/${encodeURIComponent(item.asset)}`;video.load();
  }else{player?.dispose();player=null;video.pause();video.style.display='none';scene.className='scene '+item.id.replace('preset-','');scene.style.display='block'}
  changing=false;
 }
 const settings=options();video.style.objectFit=settings.fit==='stretch'?'fill':settings.fit;if(loopFrame)loopFrame.style.objectFit=video.style.objectFit;
 await sync();
}
async function sync(){
 if(!config?.active||syncing||changing)return;syncing=true;
 try{
  let runtime;try{runtime=await invoke('wallpaper_playback_state')}catch{runtime={paused:options().paused,muted:false,reason:options().paused?'paused':null}}const reason=runtime.reason,settings={...options(),mute:options().mute||runtime.muted};audioEffects?.configure(settings.audio,Boolean(runtime.paused||reason));video.muted=settings.mute;video.volume=settings.volume/100;
  if(!current)return;scene.classList.toggle('paused',Boolean(reason));
  if(player&&!player.ready)return;
  if(player){const key=JSON.stringify([settings,runtime.paused]);if(key!==appliedOptions){player.setOptions(settings,runtime.paused);appliedOptions=key}report(reason||'playing')}
  else if(current.startsWith('video:')||current.startsWith('preset-video:')){
   if(reason){if(!video.paused)video.pause();report(reason)}else if(video.paused){const version=generation;try{await video.play();if(version===generation)report('playing')}catch(error){if(version===generation&&error?.name!=='AbortError')report('error',error)}}else report('playing');
  }else report(reason||'playing');
 }catch(error){failed(error)}finally{syncing=false}
}
video.addEventListener('error',()=>{if(current?.startsWith('video:')||current?.startsWith('preset-video:'))report('error',video.error?.message||'The video could not be decoded')});
window.__TAURI__?.event?.listen('wallpaper-changed',event=>update(event.payload));
window.__TAURI__?.event?.listen('wallpaper-scene-updated',event=>{if(config?.active===event.payload){current=null;update(config)}});
invoke('wallpaper_snapshot').then(snapshot=>update(snapshot.config)).catch(()=>report('error'));
setInterval(sync,1000);
let repairing=false;setInterval(async()=>{if(!config?.active||repairing)return;repairing=true;try{await invoke('wallpaper_repair')}catch(error){console.error('Wallpaper desktop refresh failed',error)}finally{repairing=false}},1500);
if(window.__ATELIER_RUNTIME_TEST__)import('./wallpaper-runtime-test.js').then(test=>test.run(()=>({player,current,lastStatus,config})));
