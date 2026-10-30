(()=>{
 const invoke=window.__TAURI__?.core?.invoke,P=window.AtelierSceneProperties,root=document.getElementById('wallpaper-scene-settings');
 const entries=new Map();let current=null,notify=()=>{},saved=()=>{};
 const colorToHex=value=>'#'+P.vector(value,[1,1,1]).map(n=>Math.round(Math.max(0,Math.min(1,n))*255).toString(16).padStart(2,'0')).join('');
 const hexToColor=value=>[1,3,5].map(i=>parseInt(value.slice(i,i+2),16)/255).join(' ');
 function element(tag,className,text){const node=document.createElement(tag);if(className)node.className=className;if(text!==undefined)node.textContent=text;return node}
 function schedule(entry){entry.dirty=true;if(!entry.timer)entry.timer=setTimeout(()=>{entry.timer=null;flush(entry)},80)}
 async function flush(entry){
  if(entry.saving)return;entry.saving=true;
  try{while(entry.dirty){entry.dirty=false;const options=structuredClone(entry.options);try{const config=await invoke('wallpaper_scene_options',{id:entry.id,options});saved(config)}catch(error){notify(String(error),true)}}}
  finally{entry.saving=false}
 }
 function propertyValue(entry,property){return Object.prototype.hasOwnProperty.call(entry.options.properties,property.key)?entry.options.properties[property.key]:property.default}
 function updateConditions(entry){const values=P.values(entry.schema,entry.options.properties);for(const row of entry.rows)row.node.hidden=!P.condition(row.property.condition,values)}
 function range(label,value,min,max,step,format,onChange){
  const row=element('label','scene-range'),head=element('span','scene-range-head'),output=element('output');head.append(element('span','',label),output);
  const input=element('input');input.type='range';input.min=min;input.max=max;input.step=step;input.value=value;input.setAttribute('aria-label',label);
  const update=()=>{output.value=format(Number(input.value))};update();input.oninput=()=>{update();onChange(Number(input.value))};row.append(head,input);return row;
 }
 function render(entry){
  root.replaceChildren();entry.rows=[];
  const heading=element('div','scene-settings-heading');heading.append(element('div','card-title','Scene settings'));const reset=element('button','wallpaper-secondary','Reset');reset.type='button';reset.onclick=()=>{entry.options=P.defaults();render(entry);schedule(entry)};heading.append(reset);root.append(heading);
  const fields=element('div','scene-settings-fields');
  for(const [key,label,min,max,step,format]of [['speed','Animation speed',0,3,.05,v=>v.toFixed(2)+'×'],['hue','Hue',-180,180,1,v=>v+'°'],['saturation','Saturation',0,2,.01,v=>Math.round(v*100)+'%'],['brightness','Brightness',0,2,.01,v=>Math.round(v*100)+'%'],['contrast','Contrast',0,2,.01,v=>Math.round(v*100)+'%']]){
   fields.append(range(label,entry.options[key],min,max,step,format,value=>{entry.options[key]=value;schedule(entry)}));
  }
  const tint=element('label','scene-property-row');tint.append(element('span','','Tint'));const picker=element('input');picker.type='color';picker.value=colorToHex(entry.options.tint);picker.setAttribute('aria-label','Tint');picker.oninput=()=>{entry.options.tint=hexToColor(picker.value);schedule(entry)};tint.append(picker);fields.append(tint);root.append(fields);
  const author=element('div','scene-author-settings');author.append(element('div','card-title','Wallpaper properties'));
  if(entry.loading){author.append(element('p','scene-settings-note','Loading wallpaper properties…'))}
  else if(entry.error){author.append(element('p','scene-settings-note',entry.error))}
  else if(entry.legacy){author.append(element('p','scene-settings-note','The original downloaded Workshop scene is needed to restore its author settings.'))}
  else if(!entry.schema.length){author.append(element('p','scene-settings-note','The author has not provided additional properties for this scene.'))}
  for(const property of entry.schema){
   if(property.key==='schemecolor')continue;
   if(property.type==='group'){author.append(element('div','scene-property-group',property.label));continue}
   let row,input;const value=propertyValue(entry,property),change=value=>{entry.options.properties[property.key]=value;updateConditions(entry);schedule(entry)};
   if(property.type==='slider'){row=range(property.label,value,property.min,property.max,property.step,v=>Number(v.toFixed(3)).toString(),change);input=row.querySelector('input')}
   else{row=element('label','scene-property-row');row.append(element('span','',property.label));
    if(property.type==='combo'){input=element('select');for(const option of property.options){const node=element('option','',option.label);node.value=option.value;input.append(node)}input.value=value;input.onchange=()=>change(input.value)}
    else if(property.type==='bool'){input=element('input');input.type='checkbox';input.className='switch';input.checked=Boolean(value);input.onchange=()=>change(input.checked)}
    else if(property.type==='color'){input=element('input');input.type='color';input.value=colorToHex(value);input.oninput=()=>change(hexToColor(input.value))}
    else{input=element('input');input.type='text';input.value=typeof value==='string'?value:''}
    input.setAttribute('aria-label',property.label);row.append(input);
   }
   input.disabled=!property.supported;
   const wrapper=element('div','scene-property');wrapper.append(row);if(!property.supported){wrapper.classList.add('unsupported');wrapper.append(element('p','scene-settings-note',property.reason))}
   author.append(wrapper);entry.rows.push({node:wrapper,property});
  }
  root.append(author);updateConditions(entry);
 }
 function show(item,options,onSaved,onError){
  saved=onSaved;notify=onError;const id=item?.kind==='scene'?item.id:null;root.hidden=!id;
  if(!id){current=null;return}
  let entry=entries.get(id);
  if(!entry){entry={id,options:{...P.defaults(),...options.scene,properties:{...options.scene?.properties}},schema:[],rows:[],loading:true};entries.set(id,entry);
   invoke('wallpaper_scene_info',{id}).then(info=>{entry.loading=false;entry.schema=info.properties;entry.legacy=info.legacy;if(current===id)render(entry)}).catch(error=>{entry.loading=false;entry.error=String(error);if(current===id)render(entry)});
  }else if(!entry.dirty&&!entry.saving){entry.options={...P.defaults(),...options.scene,properties:{...options.scene?.properties}}}
  if(current!==id){current=id;render(entry)}
 }
 window.AtelierSceneSettings={show};
})();
