'use strict';
const invoke=window.__TAURI__.core.invoke, panel=document.getElementById('languages');
async function refresh(){
 try{const languages=await invoke('dock_languages');panel.replaceChildren();for(const language of languages){const button=document.createElement('button');button.type='button';button.role='menuitem';button.textContent=language.name;button.onclick=async()=>{try{await invoke('dock_select_language',{id:language.id})}catch(error){panel.textContent=String(error)}};panel.append(button)}if(!languages.length)panel.textContent='No keyboard layouts';}
 catch(error){panel.textContent=String(error)}
}
window.addEventListener('focus',refresh);refresh();
document.addEventListener('keydown',event=>{if(event.key==='Escape')window.__TAURI__.window.getCurrentWindow().hide()});
