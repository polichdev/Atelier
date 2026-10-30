const test=require('node:test'),assert=require('node:assert/strict'),fs=require('node:fs'),vm=require('node:vm'),path=require('node:path');
const turn=()=>new Promise(setImmediate);
function fixture(){
 class Element{constructor(){this.style={};this.children=[];this.classList={toggle(){}}}append(child){child.remove();child.parent=this;this.children.push(child)}replaceChildren(...children){for(const child of this.children)child.parent=null;this.children=[];for(const child of children)this.append(child)}remove(){if(this.parent){this.parent.children=this.parent.children.filter(c=>c!==this);this.parent=null}}}
 const scene=new Element(),video=new Element();Object.assign(video,{pause(){},load(){},removeAttribute(){},addEventListener(){},play:()=>Promise.resolve()});
 let snapshotResolve;const snapshot=new Promise(resolve=>snapshotResolve=resolve),reports=[];
 class Player{static instances=[];constructor(host){this.canvas=new Element();host.append(this.canvas);Player.instances.push(this)}load(){return new Promise((resolve,reject)=>{this.complete=()=>{this.ready=true;resolve()};this.fail=reject})}dispose(){this.disposed=true;this.canvas.remove()}setOptions(options,paused){this.options=options;this.paused=paused}}
 const bridge={invoke(command,args){if(command==='wallpaper_snapshot')return snapshot;if(command==='wallpaper_report')reports.push(args);return Promise.resolve({paused:false,muted:false,reason:null})}};
 const context=vm.createContext({window:{AtelierScenePlayer:Player,__TAURI__:{core:bridge}},document:{getElementById:id=>id==='video'?video:scene,createElement:()=>new Element()},setInterval(){},console:{error(){}}});
 vm.runInContext(fs.readFileSync(path.join(__dirname,'../ui/wallpaper.js'),'utf8'),context);
 const config=id=>({active:id,items:[{id,kind:'scene'}],settings:{[id]:{fit:'cover',volume:50,mute:true,paused:false}}});
 return{scene,Player,reports,start:id=>snapshotResolve({config:config(id)}),update(id){context.nextConfig=config(id);return vm.runInContext('update(nextConfig)',context)}};
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
