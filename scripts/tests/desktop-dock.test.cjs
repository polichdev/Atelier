const {test,before,after}=require('node:test');
const assert=require('node:assert/strict');
const {chromium}=require('playwright');
const {pathToFileURL}=require('node:url');
const path=require('node:path');
const fs=require('node:fs');
let browser;
before(async()=>{browser=await chromium.launch({headless:true,channel:process.env.ATELIER_TEST_BROWSER||undefined})});
after(async()=>{await browser?.close()});
async function fixture(file){
  const page=await browser.newPage({viewport:{width:1100,height:800}});
  const errors=[];page.on('pageerror',e=>errors.push(e.message));
  await page.addInitScript(()=>{
    const c={enabled:true,position:'bottom',alignment:'center',monitor:0,icon_size:44,spacing:6,padding:10,edge_gap:10,radius:18,opacity:72,tint:'#232532',magnification:140,icon_style:'original',inactive_opacity:85,active_background:true,indicator:'dot',indicator_color:'#f3f0ea',show_labels:true,show_running:true,show_clock:false,show_start:true,show_search:true,show_studio:true,launch_animation:true,maximize_on_open:true,hidden_apps:[],auto_hide:false,hide_delay:700,hide_windows_taskbar:false,pinned:[{path:'alpha.exe',name:'Alpha'},{path:'beta.exe',name:'Beta'}],imported:true};
    const icon=color=>'data:image/svg+xml,'+encodeURIComponent(`<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64"><rect x="4" y="4" width="56" height="56" rx="13" fill="${color}"/><circle cx="32" cy="32" r="15" fill="white" fill-opacity=".6"/></svg>`);
    window.__dockFixture={config:c,items:[{id:'alpha.exe',name:'Alpha',icon:icon('#428fe0'),pinned:true,running:true,active:true,windows:2},{id:'beta.exe',name:'Beta',icon:icon('#a774d6'),pinned:true,running:false,active:false,windows:0},{id:'gamma.exe',name:'Gamma',icon:'',pinned:false,running:true,active:false,windows:1}],calls:[],listeners:[],delay:0};
    const f=window.__dockFixture;
    f.native={style:'Default',floating_dock:false,floating_margin:0,corner_radius:0,vertical_gap:0,hide_top_border:false,appearance:{position:-1,icon_size:0,button_width:0,button_radius:-1,inactive_opacity:100,running_indicator:0,hover_scale:100}};
    window.__setDock=async values=>{Object.assign(c,values);for(const cb of f.listeners)await cb({payload:c})};
    window.__TAURI__={event:{listen:async(name,cb)=>{f.listeners.push(cb);return()=>{}}},core:{invoke:async(cmd,args={})=>{
      f.calls.push({cmd,args:structuredClone(args)});
      if(cmd==='get_dock_snapshot')return structuredClone({config:c,items:f.items,monitors:['Display 1','Display 2']});
      if(cmd==='set_dock_config'){if(f.delay)await new Promise(r=>setTimeout(r,f.delay));Object.assign(c,args.config);return structuredClone(c)}
      if(cmd==='dock_pin'){const item=f.items.find(i=>i.id===args.id);item.pinned=args.pinned;c.pinned=f.items.filter(i=>i.pinned).map(i=>({path:i.id,name:i.name}));return}
      if(cmd==='dock_reorder'){c.pinned=args.ids.map(id=>c.pinned.find(p=>p.path===id));f.items=[...args.ids.map(id=>f.items.find(i=>i.id===id)),...f.items.filter(i=>!i.pinned)];return}
      if(cmd==='dock_set_hidden'){const item=f.items.find(i=>i.id===args.id);item.hidden=args.hidden;return}
      if(cmd==='dock_open_system')return;
      if(cmd==='dock_set_icon'){const pin=c.pinned.find(p=>p.path===args.id);pin.icon=args.icon;const item=f.items.find(i=>i.id===args.id);item.icon=args.icon||icon('#428fe0');return}
      if(cmd==='reset_desktop_dock'){c.enabled=false;return structuredClone(c)}
      if(cmd==='get_taskbar_config')return structuredClone(f.native);
      if(cmd==='set_taskbar_config'){f.native=args.config;return structuredClone(f.native)}
      if(cmd==='repair_taskbar_icons'){f.native.appearance.icon_size=0;f.native.appearance.hover_scale=100;return structuredClone(f.native)}
      if(cmd==='get_taskbar_preview_info')return{apps:[],clock:'12:00',language:'ENG'};
      return null;
    }}};
  });
  await page.goto(pathToFileURL(path.resolve('crates/ui/ui',file)).href);
  if(file==='index.html')await page.locator('#desktop-dock-details summary').click();
  return{page,errors};
}
test('desktop dock renders real states, all orientations, actions and pinned ordering',async()=>{
  const{page,errors}=await fixture('dock.html');
  await page.waitForSelector('.app');
  assert.equal(await page.locator('.app').count(),3);
  assert.equal(await page.locator('.app img:not([hidden])').count(),2);
  assert.equal(await page.locator('.app .fallback:not([hidden])').count(),1);
  assert.equal(await page.locator('.app.active.running').count(),1);
  await page.getByRole('button',{name:'Alpha, active'}).click();
  assert.equal(await page.evaluate(()=>__dockFixture.calls.some(c=>c.cmd==='dock_activate'&&c.args.id==='alpha.exe')),true);
  await page.getByRole('button',{name:'Gamma, running'}).click({button:'right'});
  await page.getByRole('menuitem',{name:'Keep in dock',exact:true}).click();
  await page.waitForFunction(()=>__dockFixture.config.pinned.length===3);
  for(const position of ['left','top','right','bottom']){
    await page.evaluate(p=>__setDock({position:p,icon_size:20,icon_style:'monochrome',indicator:'line',inactive_opacity:40}),position);
    await page.waitForFunction(p=>document.body.classList.contains(p),position);
    assert.equal(await page.locator('.app img').first().evaluate(e=>Math.round(e.getBoundingClientRect().width)),20);
  }
  await page.locator('.app[data-id="alpha.exe"]').dragTo(page.locator('.app[data-id="beta.exe"]'));
  await page.waitForFunction(()=>__dockFixture.config.pinned[0].path==='beta.exe');
  await page.evaluate(()=>__setDock({position:'bottom',icon_size:44,icon_style:'original',indicator:'dot'}));
  await page.waitForTimeout(200);
  fs.mkdirSync('target/dock-tests',{recursive:true});await page.screenshot({path:'target/dock-tests/desktop-dock.png'});
  assert.deepEqual(errors,[]);await page.close();
});
test('auto-hide has a native resize request and reveals on entry',async()=>{
  const{page,errors}=await fixture('dock.html');await page.waitForSelector('.app');
  await page.evaluate(()=>__setDock({auto_hide:true,hide_delay:200}));
  await page.mouse.move(100,100);await page.mouse.move(-20,-20);
  await page.waitForFunction(()=>document.body.classList.contains('collapsed'));
  assert.equal(await page.evaluate(()=>__dockFixture.calls.some(c=>c.cmd==='dock_resize'&&c.args.collapsed)),true);
  await page.mouse.move(100,100);
  await page.waitForFunction(()=>!document.body.classList.contains('collapsed'));
  assert.deepEqual(errors,[]);await page.close();
});
test('studio saves settings, presets, per-app controls and Windows reset disables the custom dock',async()=>{
  const{page,errors}=await fixture('index.html');await page.waitForFunction(()=>document.getElementById('dd-status').textContent==='Desktop dock is on');
  await page.locator('#dd-position').selectOption('right');
  await page.waitForFunction(()=>__dockFixture.config.position==='right');
  await page.locator('#dd-icon_style').selectOption('rounded');
  await page.waitForFunction(()=>__dockFixture.config.icon_style==='rounded');
  await page.getByRole('button',{name:'Compact',exact:true}).click();
  await page.waitForFunction(()=>__dockFixture.config.icon_size===24);
  await page.getByRole('button',{name:'Move Alpha later',exact:true}).click();
  await page.waitForFunction(()=>__dockFixture.config.pinned[0].path==='beta.exe');
  await page.locator('[data-tab="taskbar"]').click();
  await page.locator('[data-mat="Default"]').click();
  await page.waitForFunction(()=>!__dockFixture.config.enabled);
  assert.equal(await page.evaluate(()=>__dockFixture.calls.some(c=>c.cmd==='reset_defaults')),true);
  await page.locator('.scroll-area').evaluate(e=>e.scrollTop=0);assert.deepEqual(errors,[]);await page.screenshot({path:'target/dock-tests/studio.png',fullPage:true});await page.close();
});
test('rapid edits survive a slow native save',async()=>{
  const{page,errors}=await fixture('index.html');await page.waitForFunction(()=>document.getElementById('dd-status').textContent==='Desktop dock is on');
  await page.evaluate(()=>__dockFixture.delay=300);
  await page.locator('#dd-position').selectOption('left');
  await page.locator('#dd-icon_size').evaluate(e=>{e.value='28';e.dispatchEvent(new Event('input',{bubbles:true}));e.dispatchEvent(new Event('change',{bubbles:true}))});
  await page.waitForFunction(()=>__dockFixture.config.position==='left'&&__dockFixture.config.icon_size===28);
  await page.waitForTimeout(700);
  assert.equal(await page.locator('#dd-icon_size').inputValue(),'28');assert.deepEqual(errors,[]);await page.close();
});
test('custom app icon is rasterized locally and can be reset',async()=>{
  const{page,errors}=await fixture('index.html');await page.waitForFunction(()=>document.getElementById('dd-status').textContent==='Desktop dock is on');
  const tmp=await browser.newPage();await tmp.setContent('<body style="margin:0;background:red"></body>');const png=await tmp.screenshot({clip:{x:0,y:0,width:64,height:64}});await tmp.close();
  await page.locator('input[type=file]').first().setInputFiles({name:'custom.png',mimeType:'image/png',buffer:png});
  await page.waitForFunction(()=>__dockFixture.config.pinned[0].icon?.startsWith('data:image/png;base64,'));
  await page.getByRole('button',{name:'Reset icon for Alpha',exact:true}).click();
  await page.waitForFunction(()=>!__dockFixture.config.pinned[0].icon);
  assert.deepEqual(errors,[]);await page.close();
});
test('many apps remain scrollable without pushing Studio off the dock',async()=>{
  const{page,errors}=await fixture('dock.html');await page.setViewportSize({width:320,height:420});await page.waitForSelector('.app');
  await page.evaluate(()=>{const sample=__dockFixture.items[0];__dockFixture.items=Array.from({length:64},(_,i)=>({...sample,id:'app-'+i,name:'App '+i}));return __setDock({position:'left',icon_size:32})});
  await page.waitForFunction(()=>document.querySelectorAll('.app').length===64);
  const bounds=await page.locator('#studio').boundingBox();assert.ok(bounds.y>=0&&bounds.y+bounds.height<=420);
  await page.locator('.app[data-id="app-63"]').scrollIntoViewIfNeeded();await page.locator('.app[data-id="app-63"]').click();
  assert.equal(await page.evaluate(()=>__dockFixture.calls.some(c=>c.cmd==='dock_activate'&&c.args.id==='app-63')),true);
  assert.deepEqual(errors,[]);await page.close();
});
test('Taskbar controls and collapsible Desktop Dock share a tab',async()=>{
  const{page,errors}=await fixture('index.html');await page.locator('[data-tab="taskbar"]').click();
  assert.equal(await page.locator('[data-tab="desktop-dock"]').count(),0);
  assert.equal(await page.locator('#desktop-dock-settings').isVisible(),true);
  await page.locator('#desktop-dock-details summary').click();
  assert.equal(await page.locator('#dd-position').isVisible(),true);
  assert.equal(await page.locator('#dd-monitor').isVisible(),false);
  await page.locator('#desktop-dock-details summary').click();
  assert.equal(await page.locator('#dd-monitor').isVisible(),true);
  assert.equal(await page.locator('#native-taskbar-appearance').isVisible(),true);
  const before=await page.evaluate(()=>__dockFixture.calls.filter(c=>c.cmd==='set_dock_config').length);
  await page.locator('#native-compact').click();await page.waitForFunction(()=>__dockFixture.native.appearance.icon_size===20&&__dockFixture.native.appearance.button_width===32);
  await page.locator('#native-reset').click();await page.waitForFunction(()=>__dockFixture.native.appearance.icon_size===0&&__dockFixture.native.appearance.button_radius===-1);
  for(const position of ['0','1','2','3']){
    await page.locator('#native-position').selectOption(position);
    await page.waitForFunction(p=>__dockFixture.native.appearance.position===Number(p),position);
  }
  await page.locator('#native-compact').click();
  await page.waitForFunction(()=>__dockFixture.native.appearance.icon_size===20);
  await page.getByRole('button',{name:'Restore sharp icons',exact:true}).click();
  await page.waitForFunction(()=>__dockFixture.native.appearance.icon_size===0);
  assert.equal(await page.evaluate(()=>__dockFixture.native.appearance.position),3);
  assert.equal(await page.evaluate(()=>__dockFixture.calls.filter(c=>c.cmd==='set_dock_config').length),before);
  assert.deepEqual(errors,[]);await page.close();
});
test('enabling and disabling the dock requests matching Windows taskbar visibility',async()=>{
  const{page,errors}=await fixture('index.html');
  await page.waitForFunction(()=>document.getElementById('dd-status').textContent==='Desktop dock is on');
  await page.locator('#dd-enabled').uncheck();
  await page.waitForFunction(()=>!__dockFixture.config.enabled&&!__dockFixture.config.hide_windows_taskbar);
  await page.locator('#dd-enabled').check();
  await page.waitForFunction(()=>__dockFixture.config.enabled&&__dockFixture.config.hide_windows_taskbar);
  assert.deepEqual(errors,[]);await page.close();
});

test('visibility buttons and launch animation settings update independently',async()=>{
  const{page,errors}=await fixture('index.html');await page.waitForFunction(()=>document.getElementById('dd-status').textContent==='Desktop dock is on');
  await page.getByRole('button',{name:'Hide Alpha',exact:true}).click();
  await page.waitForFunction(()=>__dockFixture.items.find(i=>i.id==='alpha.exe').hidden===true);
  await page.getByRole('button',{name:'Show Alpha',exact:true}).click();
  await page.waitForFunction(()=>__dockFixture.items.find(i=>i.id==='alpha.exe').hidden===false);
  await page.locator('#dd-launch_animation').uncheck();
  await page.waitForFunction(()=>__dockFixture.config.launch_animation===false);
  assert.deepEqual(errors,[]);await page.close();
});
