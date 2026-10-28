(async()=>{
  const invoke=window.__TAURI__.core.invoke,checks=[];
  const check=(condition,name)=>{if(!condition)throw Error(name);checks.push(name)};
  try{
    let snapshot=await invoke('get_dock_snapshot');
    const base={...snapshot.config,enabled:true,hide_windows_taskbar:false,auto_hide:false};
    const originalPins=snapshot.config.pinned.map(p=>p.path);
    await invoke('set_dock_config',{config:{...base,enabled:false}});
    let config=await invoke('set_dock_config',{config:base});
    for(const position of ['left','top','right','bottom']){
      config=await invoke('set_dock_config',{config:{...config,position,icon_size:24,alignment:'center'}});
      await invoke('dock_resize',{width:position==='left'||position==='right'?90:360,height:position==='left'||position==='right'?360:90,collapsed:false});
      const p=await invoke('dock_native_test_probe',{report:null});
      const area=p.area;
      check(p.visible,'dock is visible at '+position);
      check(p.position.x>=area.position.x&&p.position.y>=area.position.y,'dock starts inside display at '+position);
      check(p.position.x+p.size.width<=area.position.x+area.size.width+1&&p.position.y+p.size.height<=area.position.y+area.size.height+1,'dock fits display at '+position);
    }
    config=await invoke('set_dock_config',{config:{...config,auto_hide:true}});
    await invoke('dock_resize',{width:360,height:90,collapsed:true});
    let p=await invoke('dock_native_test_probe',{report:null});check(p.size.height===4,'auto-hide creates a four-pixel reveal strip');
    await invoke('dock_resize',{width:360,height:90,collapsed:false});
    p=await invoke('dock_native_test_probe',{report:null});check(p.size.height>4,'dock expands from reveal strip');
    snapshot=await invoke('get_dock_snapshot');check(snapshot.items.some(i=>i.running),'real running programs are discovered');check(snapshot.items.some(i=>i.icon.startsWith('data:image/png;base64,')),'real icons have a PNG alpha channel');
    p=await invoke('dock_native_test_probe',{report:null});
    if(p.fixture){const fixtureItem=snapshot.items.find(i=>i.id===p.fixture)||snapshot.items.find(i=>i.name==='Atelier launch fixture');await invoke('dock_activate',{id:fixtureItem.id,newInstance:false});await new Promise(r=>setTimeout(r,150));snapshot=await invoke('get_dock_snapshot');check(snapshot.items.some(i=>i.id===fixtureItem.id&&i.active),'dock activates an actual native test window');}
    const launcher=snapshot.items.find(i=>i.name==='Atelier launch fixture');
    if(launcher){const before=launcher.windows;await invoke('dock_activate',{id:launcher.id,newInstance:true});for(let i=0;i<20;i++){await new Promise(r=>setTimeout(r,100));snapshot=await invoke('get_dock_snapshot');if(snapshot.items.some(a=>a.id===launcher.id&&a.windows>before))break}check(snapshot.items.some(a=>a.id===launcher.id&&a.windows>before),'dock launches a pinned shortcut with its arguments');}
    const candidates=snapshot.items.filter(i=>!i.pinned&&i.running);
    if(candidates.length){
      const id=candidates[0].id;await invoke('dock_pin',{id,pinned:true});snapshot=await invoke('get_dock_snapshot');check(snapshot.items.some(i=>i.id===id&&i.pinned),'running program can be pinned');
      const ids=snapshot.config.pinned.map(p=>p.path).reverse();await invoke('dock_reorder',{ids});snapshot=await invoke('get_dock_snapshot');check(JSON.stringify(snapshot.config.pinned.map(p=>p.path))===JSON.stringify(ids),'native pinned order is persisted');
      await invoke('dock_pin',{id,pinned:false});snapshot=await invoke('get_dock_snapshot');check(!snapshot.config.pinned.some(p=>p.path===id),'program can be unpinned');
    }
    await invoke('reset_desktop_dock');p=await invoke('dock_native_test_probe',{report:null});check(!p.visible&&!p.config.enabled&&!p.config.hide_windows_taskbar,'reset hides custom dock and disables Windows replacement');
    await invoke('dock_native_test_probe',{report:{ok:true,checks,originalPins,appCount:snapshot.items.length}});
  }catch(error){await invoke('dock_native_test_probe',{report:{ok:false,checks,error:String(error)}})}
})();
