const test=require('node:test'),assert=require('node:assert/strict'),fs=require('node:fs'),vm=require('node:vm'),path=require('node:path');
const turn=()=>new Promise(setImmediate);
function fixture(){
 class Element{constructor(){this.style={};this.children=[];this.classList={toggle(){}}}append(child){child.remove();child.parent=this;this.children.push(child)}replaceChildren(...children){for(const child of this.children)child.parent=null;this.children=[];for(const child of children)this.append(child)}remove(){if(this.parent){this.parent.children=this.parent.children.filter(c=>c!==this);this.parent=null}}}
 const scene=new Element(),video=new Element(),loopFrame=new Element(),handlers={},timers=[];let captures=0,playCalls=0;Object.assign(loopFrame,{getContext:()=>({drawImage(){captures++}})});Object.assign(video,{paused:true,readyState:4,videoWidth:1920,videoHeight:1080,duration:9,currentTime:0,seeking:false,pause(){this.paused=true},load(){},removeAttribute(){},addEventListener(name,handler){handlers[name]=handler},play(){playCalls++;this.paused=false;return Promise.resolve()}});
 let snapshotResolve,runtime={paused:false,muted:false,reason:null};const snapshot=new Promise(resolve=>snapshotResolve=resolve),reports=[];
 class Player{static instances=[];constructor(host){this.canvas=new Element();host.append(this.canvas);Player.instances.push(this)}load(){return new Promise((resolve,reject)=>{this.complete=()=>{this.ready=true;resolve()};this.fail=reject})}dispose(){this.disposed=true;this.canvas.remove()}setOptions(options,paused){this.options=options;this.paused=paused}}
 const bridge={invoke(command,args){if(command==='wallpaper_snapshot')return snapshot;if(command==='wallpaper_report')reports.push(args);return Promise.resolve(runtime)}};
 const context=vm.createContext({window:{AtelierScenePlayer:Player,__TAURI__:{core:bridge}},document:{getElementById:id=>id==='video'?video:id==='loop-frame'?loopFrame:scene,createElement:()=>new Element()},setInterval(){},setTimeout(fn,delay){timers.push({fn,delay})},requestAnimationFrame(fn){fn()},console:{error(){}}});
 vm.runInContext(fs.readFileSync(path.join(__dirname,'../ui/wallpaper.js'),'utf8'),context);
 const config=id=>({active:id,items:[{id,kind:'scene'}],settings:{[id]:{fit:'cover',volume:50,mute:true,paused:false}}});
 return{scene,video,loopFrame,handlers,Player,reports,captures:()=>captures,playCalls:()=>playCalls,setRuntime(value){runtime=value},finishFade(){const timer=timers.find(t=>t.delay===250);assert.ok(timer);timer.fn()},start:id=>snapshotResolve({config:config(id)}),startVideo(id){snapshotResolve({config:{active:id,items:[{id,kind:'video'}],settings:{[id]:{fit:'cover',volume:50,mute:true,paused:false}}}})},sync:()=>vm.runInContext('sync()',context),update(id){context.nextConfig=config(id);return vm.runInContext('update(nextConfig)',context)}};
}
test('retains the playing canvas until its replacement is fully loaded',async()=>{
 const f=fixture();f.start('first');await turn();const first=f.Player.instances[0];first.complete();await turn();assert.equal(f.scene.children[0],first.canvas);
 const pending=f.update('second');await turn();const second=f.Player.instances[1];assert.equal(first.disposed,undefined);assert.equal(f.scene.children[0],first.canvas);assert.equal(f.reports.at(-1).status,'loading');
 second.complete();await pending;assert.equal(first.disposed,true);assert.equal(f.scene.children[0],second.canvas);assert.equal(f.reports.at(-1).status,'playing');
});
test('a cancelled load cannot replace the newer wallpaper',async()=>{
 const f=fixture();f.start('first');await turn();f.Player.instances[0].complete();await turn();
 const pending=f.update('cancelled');await turn();const cancelled=f.Player.instances[1];const latest=f.update('latest');await turn();const replacement=f.Player.instances[2];
 cancelled.complete();await pending;assert.equal(cancelled.disposed,true);assert.equal(f.scene.children[0],f.Player.instances[0].canvas);
 replacement.complete();await latest;assert.equal(f.scene.children[0],replacement.canvas);
});
test('decode failures report their reason and permit a retry',async()=>{
 const f=fixture();f.start('broken');await turn();f.Player.instances[0].fail(Error('Texture decoding failed'));await turn();assert.equal(f.reports.at(-1).status,'error');assert.match(f.reports.at(-1).detail,/Texture decoding failed/);
 const retry=f.update('broken');await turn();f.Player.instances[1].complete();await retry;assert.equal(f.reports.at(-1).status,'playing');
});
test('holds the last video frame while seeking and fades it into the first frame',async()=>{
 const f=fixture();f.startVideo('video');await turn();assert.equal(f.playCalls(),1);await f.sync();assert.equal(f.playCalls(),1);
 f.video.currentTime=8.5;f.handlers.timeupdate();assert.equal(f.captures(),1);f.video.currentTime=0;f.video.seeking=true;f.handlers.seeking();assert.equal(f.loopFrame.style.display,'block');
 f.video.seeking=false;f.handlers.seeked();assert.equal(f.loopFrame.style.display,'block');assert.equal(f.loopFrame.style.opacity,'0');f.finishFade();assert.equal(f.loopFrame.style.display,'none');
});
test('ignores an interrupted video play without reporting a fatal error',async()=>{
 const f=fixture();f.video.play=()=>Promise.reject(Object.assign(Error('interrupted'),{name:'AbortError'}));f.startVideo('video');await turn();assert.notEqual(f.reports.at(-1)?.status,'error');
});
test('pauses video when a maximized app is detected and resumes when it closes',async()=>{
 const f=fixture();f.startVideo('video');await turn();assert.equal(f.video.paused,false);f.setRuntime({paused:true,muted:false,reason:'paused-maximized'});await f.sync();assert.equal(f.video.paused,true);assert.equal(f.reports.at(-1).status,'paused-maximized');f.setRuntime({paused:false,muted:false,reason:null});await f.sync();assert.equal(f.video.paused,false);assert.equal(f.reports.at(-1).status,'playing');
});
