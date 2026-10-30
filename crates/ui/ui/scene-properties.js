(()=>{
 const own=(o,k)=>Object.prototype.hasOwnProperty.call(o||{},k);
 function resolve(value,properties={}){
  if(!value||typeof value!=='object'||Array.isArray(value))return value;
  const user=value.user,key=typeof user==='string'?user:user?.name;
  if(key&&own(properties,key)){
   const selected=properties[key];
   if(typeof user==='object'&&own(user,'condition'))return String(selected)===String(user.condition);
   return selected;
  }
  return own(value,'value')?value.value:value;
 }
 function vector(value,fallback,properties){value=resolve(value,properties);const a=typeof value==='string'?value.trim().split(/\s+/).map(Number):Array.isArray(value)?value:[];return fallback.map((v,i)=>Number.isFinite(Number(a[i]))?Number(a[i]):v)}
 function number(value,fallback,properties){const n=Number(resolve(value,properties));return value!=null&&Number.isFinite(n)?n:fallback}
 const multiply=(p,m)=>[p[0]*m[0]+p[2]*m[1],p[1]*m[0]+p[3]*m[1],p[0]*m[2]+p[2]*m[3],p[1]*m[2]+p[3]*m[3],p[0]*m[4]+p[2]*m[5]+p[4],p[1]*m[4]+p[3]*m[5]+p[5]];
 function matrix(transforms,properties){let m=[1,0,0,1,0,0];for(const t of transforms){const [x,y]=vector(t.origin,[0,0],properties),[sx,sy]=vector(t.scale,[1,1],properties),angle=vector(t.angles,[0,0,0],properties)[2],c=Math.cos(angle),s=Math.sin(angle);m=multiply(multiply(m,t.attachment||[1,0,0,1,0,0]),[c*sx,s*sx,-s*sy,c*sy,x,y])}return m}
 function values(schema,overrides={}){const result=Object.create(null);for(const p of schema||[])result[p.key]=own(overrides,p.key)?overrides[p.key]:p.default;return result}
 const visible=(chain,properties)=>(chain||[]).every(v=>resolve(v,properties)!==false);
 function condition(expression,properties){
  if(!expression)return true;
  const tokens=[];let rest=expression;
  while(rest.trim()){const match=/^\s*(===|!==|==|!=|>=|<=|&&|\|\||[!()<>]|-?\d+(?:\.\d+)?|"[^"\\]*"|'[^'\\]*'|[A-Za-z_][A-Za-z_0-9]*(?:\.value)?)/.exec(rest);if(!match)return true;tokens.push(match[1]);rest=rest.slice(match[0].length);if(tokens.length>80)return true}
  let at=0;
  function atom(){const t=tokens[at++];if(t==='!')return !atom();if(t==='('){const v=or();if(tokens[at++]!==')')throw Error();return v}if(t===undefined)throw Error();if(t==='true'||t==='false')return t==='true';if(/^['"]/.test(t))return t.slice(1,-1);if(/^-?\d/.test(t))return Number(t);const key=t.replace(/\.value$/,'');return own(properties,key)?properties[key]:undefined}
  function compare(){let a=atom();const op=tokens[at];if(['===','!==','==','!=','>','<','>=','<='].includes(op)){at++;const b=atom();if(op==='=='||op==='===')a=String(a)===String(b);else if(op==='!='||op==='!==')a=String(a)!==String(b);else a=op==='>'?a>b:op==='<'?a<b:op==='>='?a>=b:a<=b}return a}
  function and(){let a=compare();while(tokens[at]==='&&'){at++;const b=compare();a=Boolean(a)&&Boolean(b)}return a}
  function or(){let a=and();while(tokens[at]==='||'){at++;const b=and();a=Boolean(a)||Boolean(b)}return a}
  try{const result=or();return at===tokens.length?Boolean(result):true}catch{return true}
 }
 const defaults=()=>({speed:1,hue:0,saturation:1,brightness:1,contrast:1,tint:'1 1 1',properties:{}});
 window.AtelierSceneProperties={resolve,vector,number,matrix,values,visible,condition,defaults};
})();
