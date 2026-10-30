class AtelierScenePlayer {
  constructor(host) {
    this.canvas = document.createElement('canvas'); host.append(this.canvas);
    this.gl = this.canvas.getContext('webgl2', {alpha:false, antialias:false, premultipliedAlpha:false,preserveDrawingBuffer:true});
    if (!this.gl) throw Error('Scene playback requires WebGL 2');
    this.ready=false;this.canvas.addEventListener('webglcontextlost',event=>{if(!this.disposed){event.preventDefault();this.onError?.(Error('The graphics context was lost. Apply the wallpaper again.'))}});
    this.options = {fit:'cover',mute:true,volume:50}; this.paused=true;this.time=0;this.last=0;this.frame=0;this.disposed=false;
    this.properties=Object.create(null);this.composition=null;this.compositionOther=null;this.texturePixels=0;this.targets=null;this.textures=new Map();this.resources=[];this.audio=[];this.layers=[];
    const gl=this.gl;
    this.program=this.createProgram(`#version 300 es
      in vec2 a_position;in vec2 a_uv;out vec2 v_uv;
      void main(){gl_Position=vec4(a_position,0.,1.);v_uv=a_uv;}`,`#version 300 es
      precision highp float;in vec2 v_uv;out vec4 outColor;
      uniform sampler2D u_image,u_mask;uniform int u_kind;
      uniform vec2 u_region;uniform float u_time,u_alpha;uniform vec3 u_color,u_low,u_high;
      uniform bool u_grading,u_aa,u_composite;uniform vec4 u_filter;uniform vec2 u_center;uniform vec4 u_grade;uniform vec3 u_tint,u_effectcolor;uniform float u_effectalpha;
      uniform vec4 u_params,u_pulse;uniform vec2 u_bounds;uniform bool u_masked;
      void main(){
        vec2 uv=v_uv;float mask=u_masked?texture(u_mask,u_composite?vec2(uv.x,1.-uv.y):uv).r:1.;
        float speed=u_params.x,scale=u_params.y,strength=u_params.z;
        vec2 direction=vec2(cos(u_params.w),sin(u_params.w));
        if(u_kind==1){uv+=sin(u_time*speed+dot(uv,direction)*scale)*vec2(direction.y,-direction.x)*strength*strength*mask;}
        if(u_kind==2){vec2 wave=vec2(sin(uv.y*scale+u_time*speed),cos(uv.x*scale-u_time*speed*.8));uv+=wave*strength*.025*mask;}
        if(u_kind==3){uv+=vec2(sin(u_time*speed*2.),cos(u_time*speed*1.7))*strength*.012*mask;}
        if(u_kind==5){uv+=vec2(sin(uv.y*scale+u_time*speed),cos(uv.x*scale+u_time*speed))*.01*strength*mask;}
        vec4 sampleColor=texture(u_image,clamp(uv*u_region,.5/vec2(textureSize(u_image,0)),u_region-.5/vec2(textureSize(u_image,0))));
        if(u_kind==4){float pulse=pow(max(0.,smoothstep(u_bounds.x,max(u_bounds.x+.0001,u_bounds.y),sin(u_time*speed+u_params.w)*.5+.5)*u_pulse.x),max(.01,u_pulse.y));
          vec4 changed=sampleColor;if(u_pulse.z>.5)changed.a*=clamp(pulse,0.,1.);
          if(u_pulse.w>.5)changed.rgb*=mix(u_low,u_high,clamp(pulse,0.,1.));
          sampleColor=mix(sampleColor,changed,mask);
        }
        if(u_kind==8||u_kind==9){
          vec2 px=1./vec2(textureSize(u_image,0));vec3 blur=vec3(0.);float weights=0.;
          for(int y=-2;y<=2;y++){for(int x=-2;x<=2;x++){float w=exp(-float(x*x+y*y)*.5);vec3 color=texture(u_image,uv+vec2(float(x),float(y))*px*u_filter.x).rgb;if(u_kind==9)color=max(vec3(0.),color-vec3(u_filter.z));blur+=color*w;weights+=w;}}
          blur/=weights;if(u_kind==8)sampleColor.rgb=mix(sampleColor.rgb,blur,clamp(u_filter.w*mask,0.,1.));else sampleColor.rgb+=blur*u_filter.y*mask;
        }
        if(u_kind==10){
          vec2 center=vec2(u_center.x,1.-u_center.y),step=(center-uv)*u_filter.x/16.;vec2 p=uv;vec3 rays=vec3(0.);float decay=1.;
          for(int i=0;i<16;i++){p+=step;vec3 color=max(texture(u_image,clamp(p,0.,1.)).rgb-vec3(u_filter.z),0.);float m=u_masked?texture(u_mask,vec2(p.x,1.-p.y)).r:1.;rays+=color*m*decay;decay*=.96;}
          sampleColor.rgb+=rays*u_filter.y/16.;
        }
        if(u_kind==11){
          vec2 p=uv*vec2(115.,38.);p.x+=p.y*.12;float column=floor(p.x),random=fract(sin(column*127.1)*43758.5453);p.y+=u_time*(7.+random*6.)*max(.1,speed);float drop=smoothstep(.04,0.,abs(fract(p.x)-.5))*smoothstep(.35,.95,fract(p.y+random));
          sampleColor.rgb+=vec3(.35,.42,.46)*drop*strength;
        }
        if(u_aa){
          vec2 pixel=1./vec2(textureSize(u_image,0));vec3 lumaWeights=vec3(.299,.587,.114);
          vec3 nw=texture(u_image,uv+vec2(-1.,-1.)*pixel).rgb,ne=texture(u_image,uv+vec2(1.,-1.)*pixel).rgb;
          vec3 sw=texture(u_image,uv+vec2(-1.,1.)*pixel).rgb,se=texture(u_image,uv+vec2(1.,1.)*pixel).rgb;
          float a=dot(nw,lumaWeights),b=dot(ne,lumaWeights),c=dot(sw,lumaWeights),d=dot(se,lumaWeights),center=dot(sampleColor.rgb,lumaWeights);
          float low=min(center,min(min(a,b),min(c,d))),high=max(center,max(max(a,b),max(c,d)));
          vec2 dir=vec2(-((a+b)-(c+d)),(a+c)-(b+d));float reduce=max((a+b+c+d)*.03125,.0078125);
          dir=clamp(dir/(min(abs(dir.x),abs(dir.y))+reduce),vec2(-8.),vec2(8.))*pixel;
          vec3 rgbA=.5*(texture(u_image,uv+dir*(-1./6.)).rgb+texture(u_image,uv+dir*(1./6.)).rgb);
          vec3 rgbB=rgbA*.5+.25*(texture(u_image,uv+dir*(-.5)).rgb+texture(u_image,uv+dir*.5).rgb);
          float lumaB=dot(rgbB,lumaWeights);sampleColor.rgb=(lumaB<low||lumaB>high)?rgbA:rgbB;
        }
        if(u_kind==6)sampleColor.rgb=mix(sampleColor.rgb,sampleColor.rgb*u_effectcolor,mask);
        if(u_kind==7)sampleColor.a*=mix(1.,u_effectalpha,mask);
        if(u_grading){vec3 axis=normalize(vec3(1.));float c=cos(u_grade.x),s=sin(u_grade.x);vec3 rgb=sampleColor.rgb*c+cross(axis,sampleColor.rgb)*s+axis*dot(axis,sampleColor.rgb)*(1.-c);float luma=dot(rgb,vec3(.2126,.7152,.0722));rgb=mix(vec3(luma),rgb,u_grade.y);rgb=(rgb-.5)*u_grade.w+.5;sampleColor.rgb=clamp(rgb*u_grade.z*u_tint,0.,1.);}
        outColor=vec4(sampleColor.rgb*u_color,sampleColor.a*u_alpha);
      }`);
    this.buffer=gl.createBuffer();gl.bindBuffer(gl.ARRAY_BUFFER,this.buffer);
    const stride=16;
    for(const [name,offset]of [['a_position',0],['a_uv',8]]){const location=gl.getAttribLocation(this.program,name);gl.enableVertexAttribArray(location);gl.vertexAttribPointer(location,2,gl.FLOAT,false,stride,offset)}
    this.uniforms={};for(const name of ['image','mask','kind','time','alpha','color','low','high','params','pulse','bounds','masked','region','grading','aa','grade','tint','effectcolor','effectalpha','composite','filter','center'])this.uniforms[name]=gl.getUniformLocation(this.program,'u_'+name);
    this.resize=()=>this.draw();window.addEventListener('resize',this.resize);
  }
  createProgram(vertex,fragment){const gl=this.gl,program=gl.createProgram();for(const [kind,source]of [[gl.VERTEX_SHADER,vertex],[gl.FRAGMENT_SHADER,fragment]]){const shader=gl.createShader(kind);gl.shaderSource(shader,source);gl.compileShader(shader);if(!gl.getShaderParameter(shader,gl.COMPILE_STATUS)){const error=gl.getShaderInfoLog(shader);gl.deleteShader(shader);throw Error(error)}gl.attachShader(program,shader);gl.deleteShader(shader)}gl.linkProgram(program);if(!gl.getProgramParameter(program,gl.LINK_STATUS))throw Error(gl.getProgramInfoLog(program));gl.useProgram(program);return program}
  async texture(src,base,isMask=false){const key=(isMask?'mask:':'')+src;if(this.textures.has(key))return this.textures.get(key);const response=await fetch(base+src,{cache:'no-store'});if(!response.ok)throw Error('Could not load scene texture');let image=await createImageBitmap(await response.blob());if(isMask){const scale=Math.min(1,1920/image.width,1080/image.height);if(scale<1){const resized=await createImageBitmap(image,{resizeWidth:Math.max(1,Math.round(image.width*scale)),resizeHeight:Math.max(1,Math.round(image.height*scale)),resizeQuality:'high'});image.close();image=resized}}if(this.disposed){image.close();throw Error('Scene was closed')}
    const gl=this.gl;if(image.width>gl.getParameter(gl.MAX_TEXTURE_SIZE)||image.height>gl.getParameter(gl.MAX_TEXTURE_SIZE)){image.close();throw Error('Scene texture exceeds GPU limits')}
    if(this.texturePixels+image.width*image.height>64*1024*1024){image.close();throw Error('Scene texture memory exceeds GPU budget')}this.texturePixels+=image.width*image.height;const texture=gl.createTexture();gl.bindTexture(gl.TEXTURE_2D,texture);gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_MIN_FILTER,gl.LINEAR);gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_MAG_FILTER,gl.LINEAR);gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_WRAP_S,gl.CLAMP_TO_EDGE);gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_WRAP_T,gl.CLAMP_TO_EDGE);gl.pixelStorei(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL,false);gl.texImage2D(gl.TEXTURE_2D,0,gl.RGBA,gl.RGBA,gl.UNSIGNED_BYTE,image);const result={texture,width:image.width,height:image.height};image.close();this.resources.push(texture);this.textures.set(key,result);return result;
  }
  target(width,height){const gl=this.gl,texture=gl.createTexture();gl.bindTexture(gl.TEXTURE_2D,texture);gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_MIN_FILTER,gl.LINEAR);gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_MAG_FILTER,gl.LINEAR);gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_WRAP_S,gl.CLAMP_TO_EDGE);gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_WRAP_T,gl.CLAMP_TO_EDGE);gl.texImage2D(gl.TEXTURE_2D,0,gl.RGBA,width,height,0,gl.RGBA,gl.UNSIGNED_BYTE,null);const framebuffer=gl.createFramebuffer();gl.bindFramebuffer(gl.FRAMEBUFFER,framebuffer);gl.framebufferTexture2D(gl.FRAMEBUFFER,gl.COLOR_ATTACHMENT0,gl.TEXTURE_2D,texture,0);if(gl.checkFramebufferStatus(gl.FRAMEBUFFER)!==gl.FRAMEBUFFER_COMPLETE)throw Error('Could not allocate scene render target');this.resources.push(texture);return{texture,framebuffer,width,height}}
  async load(id){const base=`http://wallpaper-scene.localhost/${id}/`,response=await fetch(base+'scene.json',{cache:'no-store'});if(!response.ok)throw Error('Scene manifest is missing');this.scene=await response.json();if(this.disposed)return;
    for(const definition of this.scene.layers){if(this.disposed)return;const image=definition.kind==='composite'?null:await this.texture(definition.src,base),effects=[];for(const effect of definition.effects){effects.push({...effect,maskTexture:effect.mask?await this.texture(effect.mask,base,true):null})}
      let effectWidth=0,effectHeight=0;if(effects.length&&image){const factor=Math.min(1,1920/image.width,1080/image.height);effectWidth=Math.max(1,Math.round(image.width*factor));effectHeight=Math.max(1,Math.round(image.height*factor));if(!this.targets)this.targets=[this.target(1920,1080),this.target(1920,1080)]}
      this.layers.push({...definition,image,effects,effectWidth,effectHeight});
    }
    for(const sound of this.scene.sounds){const audio=new Audio(base+sound.src);audio.loop=true;this.audio.push({audio,definition:sound})}
    this.ready=true;this.setOptions(this.options,this.paused);
  }
  setOptions(options,paused){this.options={...options,scene:{...AtelierSceneProperties.defaults(),...options.scene}};this.properties=AtelierSceneProperties.values(this.scene?.properties,this.options.scene.properties);this.paused=paused;
    for(const layer of this.layers){if(layer.transforms?.length)layer.matrix=AtelierSceneProperties.matrix(layer.transforms,this.properties);layer.activeEffects=layer.effects.filter(e=>AtelierSceneProperties.resolve(e.visible,this.properties)!==false)}
    for(const {audio,definition}of this.audio){audio.muted=options.mute;audio.volume=Math.min(1,Math.max(0,options.volume/100*AtelierSceneProperties.number(definition.volume_binding??definition.volume,1,this.properties)));if(paused||!AtelierSceneProperties.visible(definition.visibility,this.properties))audio.pause();else audio.play().catch(()=>{})}
    cancelAnimationFrame(this.frame);this.last=0;this.draw();if(!paused)this.frame=requestAnimationFrame(now=>this.tick(now))}
  tick(now){if(this.disposed||this.paused)return;if(!this.last)this.last=now;const delta=(now-this.last)/1000;if(delta>=1/(this.options.performance?.fps||30)-.0001){this.time+=Math.min(delta,.1)*(this.options.scene?.speed??1);this.last=now;this.draw()}this.frame=requestAnimationFrame(next=>this.tick(next))}
  pass(texture,vertices,effect=null,alpha=1,color=[1,1,1],region=[1,1],grading=false,antialiasing=false){const gl=this.gl,u=this.uniforms,values=effect?.values||{},combos=effect?.combos||{},number=(key,fallback)=>AtelierSceneProperties.number(values[key],fallback,this.properties),vector=(v,fallback)=>AtelierSceneProperties.vector(v,fallback,this.properties);
    gl.useProgram(this.program);gl.activeTexture(gl.TEXTURE0);gl.bindTexture(gl.TEXTURE_2D,texture);gl.uniform1i(u.image,0);gl.activeTexture(gl.TEXTURE1);gl.bindTexture(gl.TEXTURE_2D,effect?.maskTexture?.texture||texture);gl.uniform1i(u.mask,1);gl.uniform1i(u.masked,Boolean(effect?.maskTexture));gl.uniform1i(u.kind,{waterwaves:1,waterripple:2,shake:3,pulse:4,waterflow:5,tint:6,opacity:7,blur:8,bloom:9,godrays:10,rain:11}[effect?.kind]||0);gl.uniform2fv(u.region,region);gl.uniform1i(u.grading,grading);gl.uniform1i(u.aa,antialiasing);gl.uniform1i(u.composite,Boolean(effect?.composite));gl.uniform2fv(u.center,vector(values.center,[.5,.5]));gl.uniform4f(u.filter,effect?.kind==='godrays'?number('raylength',.3):number('radius',number('Aperture',1)*3),effect?.kind==='godrays'?number('rayintensity',1):number('strength',.2),number('raythreshold',number('threshold',.5)),number('Opacity',number('opacity',1)));const grade=this.options.scene||AtelierSceneProperties.defaults();gl.uniform4f(u.grade,grade.hue*Math.PI/180,grade.saturation,grade.brightness,grade.contrast);gl.uniform3fv(u.tint,vector(grade.tint,[1,1,1]));gl.uniform3fv(u.effectcolor,vector(values.tint??values.Tint??values.color,[1,1,1]));gl.uniform1f(u.effectalpha,number('alpha',number('opacity',1)));gl.uniform1f(u.time,this.time);gl.uniform1f(u.alpha,alpha);gl.uniform3fv(u.color,color);gl.uniform3fv(u.low,vector(values.tintlow,[1,1,1]));gl.uniform3fv(u.high,vector(values.tinthigh,[1,1,1]));gl.uniform2fv(u.bounds,vector(values.bounds,[0,1]));gl.uniform4f(u.params,number('speed',number('animationspeed',3)),number('scale',50),number('strength',number('ripplestrength',.1)),number('direction',number('phase',0)));gl.uniform4f(u.pulse,number('amount',1),number('power',1),combos.PULSEALPHA||0,combos.PULSECOLOR===0?0:1);gl.bindBuffer(gl.ARRAY_BUFFER,this.buffer);gl.bufferData(gl.ARRAY_BUFFER,new Float32Array(vertices),gl.DYNAMIC_DRAW);gl.drawArrays(vertices.length===16?gl.TRIANGLE_STRIP:gl.TRIANGLES,0,vertices.length/4);
  }
  draw(){if(this.disposed||!this.scene)return;const gl=this.gl,canvas=this.canvas,ratio=Math.min(devicePixelRatio||1,2)*(this.options.performance?.resolution||1),width=Math.max(1,Math.round(innerWidth*ratio)),height=Math.max(1,Math.round(innerHeight*ratio));if(canvas.width!==width||canvas.height!==height){canvas.width=width;canvas.height=height}const scene=this.scene;
    const grade=this.options.scene||AtelierSceneProperties.defaults(),grading=grade.hue!==0||grade.saturation!==1||grade.brightness!==1||grade.contrast!==1||grade.tint!=='1 1 1',antialiasing=Boolean(this.options.performance?.antialiasing),hasComposites=this.options.performance?.effects!==false&&this.layers.some(l=>l.kind==='composite'),postprocess=grading||antialiasing||hasComposites;
    if(postprocess&&(!this.composition||this.composition.width!==width||this.composition.height!==height)){
      for(const target of [this.composition,this.compositionOther])if(target){gl.deleteFramebuffer(target.framebuffer);gl.deleteTexture(target.texture);this.resources=this.resources.filter(t=>t!==target.texture)}
      this.composition=this.target(width,height);this.compositionOther=hasComposites?this.target(width,height):null;
    }
    if(hasComposites&&!this.compositionOther)this.compositionOther=this.target(width,height);
    let sceneTarget=this.composition,framebuffer=postprocess?sceneTarget.framebuffer:null;
    gl.bindFramebuffer(gl.FRAMEBUFFER,framebuffer);gl.viewport(0,0,width,height);gl.clearColor(...scene.background,1);gl.clear(gl.COLOR_BUFFER_BIT);
    let sx=width/scene.width,sy=height/scene.height;if(this.options.fit!=='stretch')sx=sy=this.options.fit==='contain'?Math.min(sx,sy):Math.max(sx,sy);
    const camera=this.scene.camera||{},shake=AtelierSceneProperties.resolve(camera.enabled,this.properties),amplitude=shake?AtelierSceneProperties.number(camera.amplitude,.5,this.properties)*10:0,cameraSpeed=AtelierSceneProperties.number(camera.speed,1,this.properties),offsetX=Math.sin(this.time*cameraSpeed)*amplitude,offsetY=Math.cos(this.time*cameraSpeed*.73)*amplitude;
    for(const layer of this.layers){if(!AtelierSceneProperties.visible(layer.visibility,this.properties))continue;const effects=this.options.performance?.effects===false?[]:layer.activeEffects||layer.effects;if(layer.kind==='composite'){
        for(const effect of effects){const target=sceneTarget===this.composition?this.compositionOther:this.composition;gl.bindFramebuffer(gl.FRAMEBUFFER,target.framebuffer);gl.viewport(0,0,width,height);gl.disable(gl.BLEND);this.pass(sceneTarget.texture,[-1,-1,0,0,1,-1,1,0,-1,1,0,1,1,1,1,1],effect);sceneTarget=target;framebuffer=target.framebuffer;}
        continue;
      }
      let texture=layer.image.texture,region=[1,1];gl.disable(gl.BLEND);
      for(let i=0;i<effects.length;i++){const target=this.targets[i%2];gl.bindFramebuffer(gl.FRAMEBUFFER,target.framebuffer);gl.viewport(0,0,layer.effectWidth,layer.effectHeight);this.pass(texture,[-1,-1,0,0,1,-1,1,0,-1,1,0,1,1,1,1,1],effects[i],1,[1,1,1],region);texture=target.texture;region=[layer.effectWidth/target.width,layer.effectHeight/target.height]}
      gl.bindFramebuffer(gl.FRAMEBUFFER,framebuffer);gl.viewport(0,0,width,height);gl.enable(gl.BLEND);gl.blendFunc(gl.SRC_ALPHA,layer.blend==='additive'?gl.ONE:gl.ONE_MINUS_SRC_ALPHA);
      const cos=Math.cos(layer.angle),sin=Math.sin(layer.angle),vertices=[];
      const m=layer.matrix||[cos*layer.scale[0],sin*layer.scale[0],-sin*layer.scale[1],cos*layer.scale[1],layer.origin[0],layer.origin[1]],points=layer.mesh||[[-.5*layer.size[0],.5*layer.size[1],0,0],[.5*layer.size[0],.5*layer.size[1],1,0],[-.5*layer.size[0],-.5*layer.size[1],0,1],[.5*layer.size[0],-.5*layer.size[1],1,1]];
      for(const [dx,dy,u,v]of points){const px=m[0]*dx+m[2]*dy+m[4]+offsetX,py=m[1]*dx+m[3]*dy+m[5]+offsetY;vertices.push((px-scene.width/2)*sx/(width/2),(py-scene.height/2)*sy/(height/2),u,v)}
      this.pass(texture,vertices,null,AtelierSceneProperties.number(layer.alpha_binding??layer.alpha,1,this.properties),AtelierSceneProperties.vector(layer.color_binding??layer.color,[1,1,1],this.properties),region);
    }
    if(postprocess){gl.bindFramebuffer(gl.FRAMEBUFFER,null);gl.viewport(0,0,width,height);gl.disable(gl.BLEND);this.pass(sceneTarget.texture,[-1,-1,0,0,1,-1,1,0,-1,1,0,1,1,1,1,1],null,1,[1,1,1],[1,1],grading,antialiasing)}
  }
  dispose(){this.disposed=true;cancelAnimationFrame(this.frame);window.removeEventListener('resize',this.resize);for(const {audio}of this.audio){audio.pause();audio.removeAttribute('src');audio.load()}for(const target of [this.composition,this.compositionOther])if(target)this.gl.deleteFramebuffer(target.framebuffer);for(const target of this.targets||[])this.gl.deleteFramebuffer(target.framebuffer);for(const texture of this.resources)this.gl.deleteTexture(texture);this.gl.deleteBuffer(this.buffer);this.gl.deleteProgram(this.program);this.gl.getExtension('WEBGL_lose_context')?.loseContext();this.canvas.remove()}
}
window.AtelierScenePlayer=AtelierScenePlayer;
