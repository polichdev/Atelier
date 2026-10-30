const test=require('node:test'),assert=require('node:assert/strict'),fs=require('node:fs'),vm=require('node:vm'),path=require('node:path');
const turn=()=>new Promise(setImmediate);
function fixture(){
 class Element{
  constructor(tag='div'){this.tag=tag;this.children=[];this.parentNode=null;this.dataset={};this.style={};this.attributes={};this.hidden=false;this.value='';this.classList={toggle(){},contains(){return true}};this.clientHeight=600;this.scrollHeight=2000;this.scrollTop=0}
  setAttribute(k,v){this.attributes[k]=v} append(...nodes){for(const n of nodes)this.insertBefore(n,null)} insertBefore(n,b){n.remove();n.parentNode=this;const i=b?this.children.indexOf(b):-1;if(i<0)this.children.push(n);else this.children.splice(i,0,n)} remove(){if(this.parentNode){this.parentNode.children.splice(this.parentNode.children.indexOf(this),1);this.parentNode=null}} replaceChildren(...nodes){for(const n of this.children)n.parentNode=null;this.children=[];this.append(...nodes)}
  querySelector(tag){return this.querySelectorAll(tag)[0]||null} querySelectorAll(tag){return this.children.flatMap(c=>[...(c.tag===tag?[c]:[]),...c.querySelectorAll(tag)])} closest(){return importLabel} addEventListener(k,fn){this['on'+k]=fn} pause(){} load(){} play(){return Promise.resolve()}
 }
 const ids=new Map(),get=id=>{if(!ids.has(id))ids.set(id,new Element());return ids.get(id)},results=new Element(),toolbar=new Element(),importLabel=new Element();
 const tabs=['library','workshop','settings'].map(t=>{const x=new Element('button');x.dataset.wallpaperTab=t;return x});
 const items=Array.from({length:40},(_,i)=>({id:'v'+i,title:'Wallpaper '+i,kind:'video',category:'video'}));let scans=0;
 const invoke=async cmd=>{if(cmd==='wallpaper_snapshot')return {config:{items,settings:{},performance:{}},playback:'stopped'};if(cmd==='workshop_list_local'){scans++;return [{id:'123',title:'Steam item',kind:'video',video_available:true,preview:'data:image/png;base64,AA'}]}return {}};
 const document={getElementById:get,createElement:t=>new Element(t),querySelector:q=>q==='.wallpaper-results'?results:toolbar,querySelectorAll:q=>q==='[data-wallpaper-tab]'?tabs:[],addEventListener(){},hidden:false};
 class Observer{constructor(fn){this.fn=fn;Observer.instance=this;this.observed=[]}observe(v){this.observed.push(v)}unobserve(v){this.observed=this.observed.filter(x=>x!==v)}}
 const window={__TAURI__:{core:{invoke},event:{listen(){}}},AtelierSceneSettings:{show(){}},AtelierWallpaperSettings:{show(){}},addEventListener(){}};
 const context=vm.createContext({window,document,IntersectionObserver:Observer,console,setTimeout,clearTimeout,setInterval(){},Date,Set,Map,Number});
 vm.runInContext(fs.readFileSync(path.join(__dirname,'../ui/studio-wallpaper.js'),'utf8'),context);
 return{get,tabs,results,items,Observer,scans:()=>scans};
}
test('pages the library and retains existing previews when selecting and loading more',async()=>{
 const f=fixture();await turn();const grid=f.get('wallpaper-grid-library');assert.equal(grid.children.length,18);const first=grid.children[0],video=first.querySelector('video');assert.equal(video.src,undefined);assert.ok(f.Observer.instance.observed.includes(video));first.onclick();assert.equal(grid.children[0],first);f.get('wallpaper-more-button').onclick();assert.equal(grid.children.length,36);assert.equal(grid.children[0],first);
});
test('keeps each tab’s cards and avoids repeat Steam scans on quick switches',async()=>{
 const f=fixture();await turn();const library=f.get('wallpaper-grid-library').children[0];await f.tabs[1].onclick();const grid=f.get('wallpaper-grid-workshop'),steam=grid.children[0];assert.equal(steam.querySelector('video').poster,'data:image/png;base64,AA');await f.tabs[0].onclick();await f.tabs[1].onclick();assert.equal(f.scans(),1);assert.equal(grid.children[0],steam);await f.tabs[0].onclick();assert.equal(f.get('wallpaper-grid-library').children[0],library);
});
test('search filters the entire library and resets the page limit',async()=>{
 const f=fixture();await turn();f.get('wallpaper-more-button').onclick();const search=f.get('wallpaper-search');search.value='Wallpaper 0';search.oninput();const grid=f.get('wallpaper-grid-library');assert.equal(grid.children.length,1);search.value='';search.oninput();assert.equal(grid.children.length,18);
});
