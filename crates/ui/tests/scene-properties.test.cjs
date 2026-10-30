const test=require('node:test'),assert=require('node:assert/strict'),fs=require('node:fs'),vm=require('node:vm'),path=require('node:path');
const context=vm.createContext({window:{},requestAnimationFrame:()=>1,cancelAnimationFrame:()=>{}});
for(const name of ['scene-properties.js','scene-player.js'])vm.runInContext(fs.readFileSync(path.join(__dirname,'../ui',name),'utf8'),context);
const P=context.window.AtelierSceneProperties;context.AtelierSceneProperties=P;
test('author bindings switch conditional character variants and retain defaults',()=>{
 const properties=P.values([{key:'size',default:'0'},{key:'animations',default:true}],{size:'1',animations:false});
 assert.equal(P.resolve({user:{name:'size',condition:'0'},value:true},properties),false);
 assert.equal(P.resolve({user:{name:'size',condition:'1'},value:false},properties),true);
 assert.equal(P.visible([{user:'animations',value:true}],properties),false);
 assert.equal(P.resolve({user:'missing',value:.4},properties),.4);
 const m=P.matrix([{origin:'100 50',scale:'2 2',angles:'0 0 0'},{origin:'3 4',scale:'1 1',angles:'0 0 0',attachment:[1,0,0,1,5,10]}],properties);
 assert.deepEqual(Array.from(m),[2,0,0,2,116,78]);
});
test('display conditions use a bounded grammar and never execute script',()=>{
 assert.equal(P.condition('enabled.value && size.value == "1"',{enabled:true,size:'1'}),true);
 assert.equal(P.condition('enabled.value && size.value == "1"',{enabled:false,size:'1'}),false);
 assert.equal(P.condition('!enabled.value || (strength.value >= 0.5)',{enabled:true,strength:.2}),false);
 P.condition('window.compromised = true',{});assert.equal(context.window.compromised,undefined);
});
test('animation speed scales elapsed time and zero freezes the timeline',()=>{
 const tick=context.window.AtelierScenePlayer.prototype.tick;
 const player={disposed:false,paused:false,last:100,time:4,options:{scene:{speed:2}},draw(){},tick(){}};
 tick.call(player,150);assert.equal(player.time,4.1);
 player.options.scene.speed=0;tick.call(player,200);assert.equal(player.time,4.1);
 player.options.scene.speed=.5;tick.call(player,250);assert.equal(player.time,4.125);
});
