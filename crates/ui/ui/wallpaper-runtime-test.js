export async function run(state){
 const invoke=window.__TAURI__.core.invoke,start=performance.now();
 try{
  while(performance.now()-start<30000){const s=state();if(s.lastStatus==='error')throw Error('Controller reported playback error');if(s.player?.scene&&s.player.layers.length===s.player.scene.layers.length&&s.lastStatus==='playing')break;await new Promise(resolve=>setTimeout(resolve,100))}
  const s=state();if(!s.player?.scene)throw Error('Scene did not start');
  s.player.draw();const canvas=s.player.canvas,gl=s.player.gl,pixels=new Uint8Array(canvas.width*canvas.height*4);gl.readPixels(0,0,canvas.width,canvas.height,gl.RGBA,gl.UNSIGNED_BYTE,pixels);
  let colored=0;for(let i=0;i<pixels.length;i+=4)if(pixels[i]+pixels[i+1]+pixels[i+2]>12)colored++;
  const png=canvas.toDataURL('image/png');s.player.setOptions(s.player.options,true);await new Promise(resolve=>setTimeout(resolve,200));
  const retained=new Uint8Array(pixels.length);gl.readPixels(0,0,canvas.width,canvas.height,gl.RGBA,gl.UNSIGNED_BYTE,retained);let retainedColored=0;for(let i=0;i<retained.length;i+=4)if(retained[i]+retained[i+1]+retained[i+2]>12)retainedColored++;
  const error=gl.getError();await invoke('scene_test_report',{report:{ok:colored>canvas.width*canvas.height*.1&&retainedColored>canvas.width*canvas.height*.1&&error===0,status:s.lastStatus,source:s.current,canvas:{width:canvas.width,height:canvas.height},colored,retainedColored,glError:error,paused:s.player.paused,time:s.player.time,sceneDisplay:getComputedStyle(document.getElementById('scene')).display,png}});
 }catch(error){await invoke('scene_test_report',{report:{ok:false,error:String(error),stack:error.stack}})}
}
