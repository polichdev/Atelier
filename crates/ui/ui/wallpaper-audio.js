window.createWallpaperAudioEffects=({video,scene,glow,invoke})=>{
 let config=null,active=false,frame=null,generation=0,pending=false,nextPoll=0,target=0,level=0,lastFrame=0,reported=false;
 function reset(){for(const node of [video,scene]){node.style.transform='';node.style.filter=''}if(glow)glow.style.opacity='0';target=0;level=0;lastFrame=0}
 function tick(now){frame=null;if(!active)return;if(!pending&&now>=nextPoll){pending=true;nextPoll=now+80;const version=generation;invoke('audio_spectrum').then(data=>{if(version!==generation)return;const bass=Number(data.bass)||0,mid=Number(data.mid)||0;target=Math.min(1,Math.max(0,bass*2.5+mid*.5));reported=false}).catch(error=>{if(version!==generation)return;target=0;nextPoll=performance.now()+2000;if(!reported){reported=true;invoke('wallpaper_audio_error',{message:String(error)}).catch(()=>{})}}).finally(()=>{pending=false})}
  const dt=Math.min(100,Math.max(1,now-(lastFrame||now)));lastFrame=now;const rate=target>level?0.22:0.08;level+=(target-level)*(1-Math.pow(1-rate,dt/16.7));const scale=1+(config.pulse?level*Math.max(0,Math.min(100,Number(config.pulse_strength)||0))*.0006:0),light=config.glow?level*Math.max(0,Math.min(100,Number(config.glow_strength)||0))/100:0;
  for(const node of [video,scene]){node.style.transform=scale>1?`scale(${scale.toFixed(4)})`:'';node.style.filter=light>0?`brightness(${(1+light*.2).toFixed(3)})`:''}if(glow)glow.style.opacity=(light*.55).toFixed(3);frame=requestAnimationFrame(tick)
 }
 function configure(options,paused){config=options||null;const next=Boolean(!paused&&config&&(config.pulse||config.glow));if(next===active)return;active=next;generation++;if(!active){if(frame!==null)cancelAnimationFrame(frame);frame=null;reset()}else{nextPoll=0;lastFrame=0;frame=requestAnimationFrame(tick)}}
 return{configure};
};
