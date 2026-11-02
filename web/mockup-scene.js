// Shared by the live editor and local HyperFrames output. Three.js 0.187.0 / MIT.
import * as THREE from './vendor/three.module.js';
export async function createMockup(canvas,spec,frontURL,backURL){
 const renderer=new THREE.WebGLRenderer({canvas,antialias:true,alpha:false,preserveDrawingBuffer:true});
 renderer.setPixelRatio(1);renderer.setSize(spec.output_width,spec.output_height,false);
 renderer.outputColorSpace=THREE.SRGBColorSpace;renderer.toneMapping=THREE.ACESFilmicToneMapping;renderer.toneMappingExposure=1.15;
 renderer.shadowMap.enabled=true;renderer.shadowMap.type=THREE.PCFSoftShadowMap;
 const scene=new THREE.Scene();scene.background=new THREE.Color(spec.background);
 const camera=new THREE.PerspectiveCamera(33,spec.output_width/spec.output_height,.05,80);
 const group=new THREE.Group();scene.add(group);
 const texture=await new THREE.TextureLoader().loadAsync(frontURL);texture.colorSpace=THREE.SRGBColorSpace;texture.anisotropy=Math.min(8,renderer.capabilities.getMaxAnisotropy());
 const back=backURL?await new THREE.TextureLoader().loadAsync(backURL):null;if(back)back.colorSpace=THREE.SRGBColorSpace;
 const W=spec.width,H=spec.height,D=spec.depth;
 const surface=(color,map=null)=>new THREE.MeshStandardMaterial({color,map,roughness:spec.roughness,metalness:0});
 const face=surface('#ffffff',texture),rear=surface(back?'#ffffff':spec.cover_color,back),edge=surface(spec.cover_color);
 const mesh=(w,h,d,m,x=0,y=0,z=0)=>{const o=new THREE.Mesh(new THREE.BoxGeometry(w,h,d),m);o.position.set(x,y,z);o.castShadow=true;o.receiveShadow=true;group.add(o);return o;};
 if(spec.kind==='book'){
  // +Z is the front, +Y the top, -X the spine. Front text is never mirrored.
  const th=Math.min(.045,D*.16);const paper=surface('#eee8da');
  mesh(W-.045,H-.065,D-2*th,paper,.017,0,0);
  mesh(W,H,th,[edge,edge,edge,edge,face,edge],0,0,D/2-th/2);
  mesh(W,H,th,[edge,edge,edge,edge,edge,rear],0,0,-D/2+th/2);
  mesh(.045,H,D,edge,-W/2+.018,0,0);
  // Fine page edges, generated without randomness or imported artwork.
  const lineMaterial=surface('#d0c8b8');for(let i=1;i<22;i++)mesh(W-.085,.0015,.002,lineMaterial,.026,-H/2+.045+(H-.09)*i/22,D/2-th-.001);
 }else if(spec.kind==='box'){
  mesh(W,H,D,[edge,edge,edge,edge,face,rear]);
  mesh(W+.006,.009,D+.006,surface(spec.cover_color),0,H*.40,0);
 }else{
  const cardDepth=Math.min(.018,D/5);for(let i=0;i<5;i++){const o=mesh(W,H,cardDepth,[edge,edge,edge,edge,face,rear],(i-2)*.014,-(i-2)*.012,(i-2)*cardDepth*1.6);o.rotation.z=(i-4)*.012;}
 }
 const floor=new THREE.Mesh(new THREE.PlaneGeometry(200,200),surface(spec.background));floor.rotation.x=-Math.PI/2;floor.position.y=-H/2-.012;floor.receiveShadow=false;scene.add(floor);
 // A deterministic soft contact shadow avoids a harsh cut-out cast shadow in listing images.
 const shadowCanvas=document.createElement('canvas');shadowCanvas.width=256;shadowCanvas.height=256;const ctx=shadowCanvas.getContext('2d');const gradient=ctx.createRadialGradient(128,128,5,128,128,120);gradient.addColorStop(0,'rgba(20,27,22,0.40)');gradient.addColorStop(.35,'rgba(20,27,22,0.25)');gradient.addColorStop(1,'rgba(20,27,22,0)');ctx.fillStyle=gradient;ctx.fillRect(0,0,256,256);
 const shadowTexture=new THREE.CanvasTexture(shadowCanvas);const shadow=new THREE.Mesh(new THREE.PlaneGeometry(W*1.75,Math.max(.65,D*3)),new THREE.MeshBasicMaterial({map:shadowTexture,transparent:true,depthWrite:false}));shadow.rotation.x=-Math.PI/2;shadow.position.y=-H/2-.009;scene.add(shadow);
 const hemi=new THREE.HemisphereLight(0xe4efff,0x6d6357,1.4);scene.add(hemi);
 const key=new THREE.DirectionalLight(spec.light==='warm'?0xffddbb:0xffffff,4.2);key.position.set(-3,5,6);key.castShadow=true;key.shadow.mapSize.set(2048,2048);key.shadow.camera.left=-4;key.shadow.camera.right=4;key.shadow.camera.top=4;key.shadow.camera.bottom=-4;key.shadow.normalBias=.012;key.shadow.bias=-.0002;key.shadow.radius=4;scene.add(key);
 const rim=new THREE.DirectionalLight(0xb6d7ff,2.1);rim.position.set(4,2,-3);scene.add(rim);
 const fill=new THREE.DirectionalLight(0xffffff,.7);fill.position.set(3,2,5);scene.add(fill);
 let frames=0,disposed=false;
 function renderAt(time=0){if(disposed)return;const u=Math.max(0,Math.min(1,time/6));const ease=u*u*(3-2*u);group.rotation.set(0,THREE.MathUtils.degToRad(spec.yaw+(spec.sweep||0)*ease),THREE.MathUtils.degToRad(spec.tilt));
  const fit=Math.max(H,W/(spec.output_width/spec.output_height),D)*2.1/spec.zoom;
  const elevation=THREE.MathUtils.degToRad(spec.elevation);camera.position.set(0,Math.sin(elevation)*fit,Math.cos(elevation)*fit);camera.lookAt(0,-H*.035,0);renderer.render(scene,camera);frames++;canvas.dataset.renderCount=frames;canvas.dataset.time=time;
 }
 renderAt(0);
 return {renderAt,dispose(){disposed=true;scene.traverse(o=>{o.geometry?.dispose();if(o.material){for(const m of Array.isArray(o.material)?o.material:[o.material])m.dispose();}});texture.dispose();back?.dispose();shadowTexture.dispose();renderer.dispose();},renderer};
}
