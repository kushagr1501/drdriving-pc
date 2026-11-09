import fs from 'node:fs';
import path from 'node:path';
import sharp from 'sharp';
import {NodeIO} from '@gltf-transform/core';
import {ALL_EXTENSIONS} from '@gltf-transform/extensions';
import {dequantize} from '@gltf-transform/functions';
import {MeshoptDecoder} from 'meshoptimizer';
await MeshoptDecoder.ready;
const io=new NodeIO().registerExtensions(ALL_EXTENSIONS).registerDependencies({'meshopt.decoder':MeshoptDecoder});
const cars=JSON.parse(fs.readFileSync('tools/car-quality/cars.json'));
cars.push({id:'bmw-320i',slot:27,brand:'BMW',model:'320i',paint:{materials:['material_0']}});
const output=path.resolve('local-test-assets/quality-upgrade');fs.mkdirSync(output,{recursive:true});
const v3=(m,v,w=1)=>[0,1,2].map(r=>m[r]*v[0]+m[4+r]*v[1]+m[8+r]*v[2]+w*m[12+r]);
const normalize=v=>{const l=Math.hypot(...v)||1;return v.map(n=>n/l)};
const cross=(a,b)=>[a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0]];
const sub=(a,b)=>a.map((v,i)=>v-b[i]);
const data=a=>Array.from({length:a.getCount()},(_,i)=>a.getElement(i,[]));
const paintColor={'bmw-320i':[0.43,0.018,0.012,1],'audi-q3':[0.018,0.095,0.16,1],'tesla-cybertruck':[0.43,0.46,0.49,1],'mclaren-720s':[0.9,0.16,0.015,1],'lamborghini-urus':[0.60,0.32,0.012,1]};
for(const car of cars) {
  if(process.argv[2]&&car.id!==process.argv[2])continue;
  const source=car.id==='bmw-320i'?'local-test-assets/bmw-320i':`local-test-assets/modern-cars/${car.id}`;
  const doc=await io.read(path.join(source,'source.glb'));await doc.transform(dequantize());
  const mats=doc.getRoot().listMaterials(),parts=[];
  doc.getRoot().getDefaultScene().traverse(n=>{
    for(let p=n;p;p=p.getParentNode())if(/LOD[1-9]|crack/i.test(p.getName()))return;
    for(const p of n.getMesh()?.listPrimitives()??[]) {
      if(p.getMode()!==4)continue;
      const world=n.getWorldMatrix(),pos=data(p.getAttribute('POSITION')).map(v=>v3(world,v));
      const norms=p.getAttribute('NORMAL')?data(p.getAttribute('NORMAL')).map(v=>{
        // Inverse transpose of the node's affine 3x3, including nonuniform scale.
        const a=[world[0],world[1],world[2]],b=[world[4],world[5],world[6]],c=[world[8],world[9],world[10]];
        const bc=cross(b,c),ca=cross(c,a),ab=cross(a,b),det=a.reduce((s,x,i)=>s+x*bc[i],0);
        return normalize(v.map((_,i)=>(bc[i]*v[0]+ca[i]*v[1]+ab[i]*v[2])/(det||1)));
      }):pos.map(()=>[0,0,0]);
      const mat=p.getMaterial(),info=mat?.getBaseColorTextureInfo(),transform=info?.getExtension('KHR_texture_transform');
      const uvA=p.getAttribute(`TEXCOORD_${transform?.getTexCoord()??info?.getTexCoord()??0}`);
      let uv=uvA?data(uvA):pos.map(()=>[0,0]);
      if(transform){const o=transform.getOffset(),s=transform.getScale(),r=transform.getRotation();uv=uv.map(([u,v])=>[o[0]+Math.cos(r)*u*s[0]-Math.sin(r)*v*s[1],o[1]+Math.sin(r)*u*s[0]+Math.cos(r)*v*s[1]]);}
      const indices=p.getIndices()?Array.from(p.getIndices().getArray()):pos.map((_,i)=>i);
      if(!p.getAttribute('NORMAL'))for(let f=0;f<indices.length;f+=3){const [a,b,c]=indices.slice(f,f+3),normal=cross(sub(pos[b],pos[a]),sub(pos[c],pos[a]));for(const i of [a,b,c])norms[i]=norms[i].map((v,k)=>v+normal[k]);}
      // Rebuild smooth paint normals across UV seams. Some exports carry split
      // shading normals on otherwise continuous door and quarter panels.
      if(/carpaint|material_0$|^body$/i.test(mat?.getName()??'')) {
        const accumulated=new Map(),keys=pos.map(p=>p.map(v=>v.toFixed(5)).join(','));
        for(let f=0;f<indices.length;f+=3){
          const face=indices.slice(f,f+3),[a,b,c]=face,normal=cross(sub(pos[b],pos[a]),sub(pos[c],pos[a]));
          for(let k=0;k<3;k++){
            const i=face[k],sum=accumulated.get(keys[i])??[0,0,0];
            // Weight by face area so thin door-edge bevels cannot distort a large panel.
            accumulated.set(keys[i],sum.map((x,j)=>x+normal[j]));
          }
        }
        for(let i=0;i<norms.length;i++){
          const candidate=accumulated.get(keys[i]);
          if(candidate&&Math.hypot(...candidate)>1e-9)norms[i]=normalize(candidate);
        }
      }
      // Degenerate authoring faces occasionally carry a zero normal. They must
      // not feed normalize(0) into the shader, which can produce bright specks.
      parts.push({name:n.getName(),material:Math.max(0,mats.indexOf(mat)),pos,norms:norms.map(n=>Math.hypot(...n)>1e-9?normalize(n):[0,1,0]),uv,indices});
    }
  });
  const points=parts.flatMap(p=>p.pos),mean=points.reduce((s,v)=>[s[0]+v[0]/points.length,s[1]+v[2]/points.length],[0,0]);
  const cov=points.reduce((s,v)=>{const x=v[0]-mean[0],z=v[2]-mean[1];return[s[0]+x*x,s[1]+x*z,s[2]+z*z]},[0,0,0]);
  let angle=0.5*Math.atan2(2*cov[1],cov[0]-cov[2]);
  // Same forward conventions as the visually checked initial pack.
  let facing=['audi-q3','mclaren-720s'].includes(car.id)?-1:1;
  if(car.id==='bmw-320i'){angle=Math.PI/2;facing=1;}
  const cs=Math.cos(angle),sn=Math.sin(angle),orient=v=>[(sn*v[0]-cs*v[2])*facing,v[1],(cs*v[0]+sn*v[2])*facing];
  for(const p of parts){p.pos=p.pos.map(orient);p.norms=p.norms.map(orient);}
  const lo=[Infinity,Infinity,Infinity],hi=[-Infinity,-Infinity,-Infinity];
  for(const p of parts)for(const v of p.pos)for(let k=0;k<3;k++){lo[k]=Math.min(lo[k],v[k]);hi[k]=Math.max(hi[k],v[k]);}
  const length=car.id==='bmw-320i'?4.5:4.6,scale=length/(hi[2]-lo[2]);
  for(const p of parts)p.pos=p.pos.map(v=>[(v[0]-(lo[0]+hi[0])/2)*scale,(v[1]-lo[1])*scale,(v[2]-(lo[2]+hi[2])/2)*scale]);
  const rig=JSON.parse(fs.readFileSync(path.join(source,'wheel-rig.json'))),wheels=rig.wheels;
  if(car.id==='lamborghini-urus')for(const w of wheels)w.center=[-w.center[0],w.center[1],-w.center[2]];
  let blobs=[];let offset=0;
  const add=b=>{const location={offset,length:b.length};blobs.push(b);offset+=b.length;return location};
  const imageIds=new Map(),images=[];
  const image=async tex=>{
    if(!tex)return -1;if(imageIds.has(tex))return imageIds.get(tex);
    const id=images.length;imageIds.set(tex,id);
    const buffer=await sharp(tex.getImage()).resize(4096,4096,{fit:'inside',withoutEnlargement:true}).png().toBuffer();
    images.push(add(buffer));return id;
  };
  const materials=[];
  for(const mat of mats) {
    const name=mat.getName(),lower=name.toLowerCase(),glass=/glass|window|windscreen|windshield/.test(lower)||mat.getAlphaMode()==='BLEND'&&mat.getAlpha()<0.85;
    const paint=car.paint?.materials?.includes(name)||/carpaint|body_paint|_paint_|material_0$/.test(lower)||car.id==='mclaren-720s'&&lower==='body';
    const rubber=/rubber|tyre|tire|carbon|mesh|grille|interior|base_material/.test(lower);
    const metal=/chrome|rim|velg|silver|metal|disc|brake/.test(lower)&&!rubber;
    let base=mat.getBaseColorFactor(),metallic=mat.getMetallicFactor(),roughness=mat.getRoughnessFactor();
    if(paint&&!mat.getBaseColorTexture())base=paintColor[car.id]??base;
    if(paint){roughness=Math.max(0.28,Math.min(roughness,0.38));metallic=Math.min(metallic,0.18);}
    if(car.id==='porsche-911-c4s'&&!glass){metallic=0.20;roughness=0.32;}
    if(rubber){roughness=Math.max(roughness,0.66);metallic=0.04;}
    if(metal){metallic=Math.max(metallic,0.78);roughness=Math.min(roughness,0.28);}
    if(car.id==='tesla-cybertruck'&&/body|gray/.test(lower)){metallic=0.88;roughness=0.27;}
    if(glass){base=[0.025,0.04,0.055,0.92];roughness=0.24;metallic=0.04;}
    materials.push({name,base,metallic,roughness,clearcoat:paint?1:glass?0.9:0.1,glass:glass?1:0,normalScale:mat.getNormalScale(),alphaCutoff:mat.getAlphaMode()==='MASK'?mat.getAlphaCutoff():0,
      baseMap:await image(glass?null:mat.getBaseColorTexture()),normalMap:await image(glass?null:mat.getNormalTexture()),surfaceMap:await image(glass?null:mat.getMetallicRoughnessTexture()),occlusionMap:await image(glass?null:mat.getOcclusionTexture()),emissionMap:await image(glass?null:mat.getEmissiveTexture()),emission:glass?[0,0,0]:mat.getEmissiveFactor()});
  }
  const textureEnd=offset;
  const exported=[];let total=0,counts=[0,0,0,0];
  for(const part of parts) {
    // Classify connected components, keeping wheel meshes separate from adjacent bodywork.
    const count=part.indices.length/3,parent=Array.from({length:count},(_,i)=>i),owner=new Map();
    const find=i=>{while(parent[i]!==i){parent[i]=parent[parent[i]];i=parent[i]}return i};
    for(let f=0;f<count;f++)for(let k=0;k<3;k++){const p=part.pos[part.indices[f*3+k]],key=p.map(v=>v.toFixed(4)).join(',');if(owner.has(key))parent[find(f)]=find(owner.get(key));else owner.set(key,f);}
    const components=new Map();for(let f=0;f<count;f++){const root=find(f);if(!components.has(root))components.set(root,[]);components.get(root).push(f);}
    const faceWheels=Array(count).fill(-1);
    for(const faces of components.values()) {
      const pts=faces.flatMap(f=>part.indices.slice(f*3,f*3+3).map(i=>part.pos[i]));
      for(let w=0;w<4;w++) {
        const wheel=wheels[w],r=wheel.radius*1.20,center=wheel.center;
        const fit=pts.every(v=>Math.abs(v[0]-center[0])<0.43&&Math.hypot(v[1]-center[1],v[2]-center[2])<r);
        if(fit&&!/caliper|calliper/i.test(part.name)){for(const f of faces)faceWheels[f]=w;break;}
      }
    }
    const tangents=part.pos.map(()=>[0,0,0]),bitangents=part.pos.map(()=>[0,0,0]);
    for(let f=0;f<count;f++) {
      const ids=part.indices.slice(f*3,f*3+3),[a,b,c]=ids,e1=sub(part.pos[b],part.pos[a]),e2=sub(part.pos[c],part.pos[a]),u1=sub(part.uv[b],part.uv[a]),u2=sub(part.uv[c],part.uv[a]);
      const det=u1[0]*u2[1]-u1[1]*u2[0];if(Math.abs(det)<1e-9)continue;
      const t=e1.map((v,k)=>(v*u2[1]-e2[k]*u1[1])/det),bt=e1.map((v,k)=>(e2[k]*u1[0]-v*u2[0])/det);
      for(const i of ids){tangents[i]=tangents[i].map((v,k)=>v+t[k]);bitangents[i]=bitangents[i].map((v,k)=>v+bt[k]);}
    }
    for(let group=-1;group<4;group++) {
      const remap=new Map(),vertices=[],indices=[];
      for(let f=0;f<count;f++)if(faceWheels[f]===group) {
        for(let k=0;k<3;k++){
          const index=part.indices[f*3+k];let dst=remap.get(index);
          if(dst===undefined){dst=remap.size;remap.set(index,dst);const n=part.norms[index],t=tangents[index],ndot=n.reduce((s,v,k)=>s+v*t[k],0);let tangent=normalize(t.map((v,k)=>v-n[k]*ndot));if(Math.hypot(...tangent)<0.1)tangent=normalize(cross(n,Math.abs(n[1])<0.9?[0,1,0]:[1,0,0]));const sign=cross(n,tangent).reduce((s,v,k)=>s+v*bitangents[index][k],0)<0?-1:1;vertices.push(...part.pos[index],...n,...part.uv[index],...tangent,sign);}
          indices.push(dst);
        }
      }
      if(!indices.length)continue;
      total+=indices.length/3;if(group>=0)counts[group]+=indices.length/3;
      exported.push({material:part.material,wheel:group,vertexCount:vertices.length/12,indexCount:indices.length,vertices:add(Buffer.from(new Float32Array(vertices).buffer)),indices:add(Buffer.from(new Uint32Array(indices).buffer))});
    }
  }
  if(counts.some(n=>n<10))throw new Error(`${car.id}: wheel extraction incomplete ${counts}`);
  // Hundreds of authoring nodes can share one material/axle. Consolidate these
  // without simplifying geometry, avoiding hundreds of sampler-binding flushes.
  const payload=Buffer.concat(blobs),groups=new Map();
  for(const p of exported){const key=`${p.material}:${p.wheel}`;if(!groups.has(key))groups.set(key,[]);groups.get(key).push(p);}
  blobs=[payload.subarray(0,textureEnd)];offset=textureEnd;
  const merged=[];
  for(const group of groups.values()){
    const vertices=[],indices=[];let vertexCount=0,indexCount=0;
    for(const p of group){
      vertices.push(payload.subarray(p.vertices.offset,p.vertices.offset+p.vertices.length));
      const ix=Buffer.from(payload.subarray(p.indices.offset,p.indices.offset+p.indices.length));
      for(let i=0;i<ix.length;i+=4)ix.writeUInt32LE(ix.readUInt32LE(i)+vertexCount,i);
      indices.push(ix);vertexCount+=p.vertexCount;indexCount+=p.indexCount;
    }
    merged.push({material:group[0].material,wheel:group[0].wheel,vertexCount,indexCount,vertices:add(Buffer.concat(vertices)),indices:add(Buffer.concat(indices))});
  }
  const header={version:1,name:`${car.brand} ${car.model}`,materials,images,parts:merged,wheels:wheels.map(w=>({pivot:w.center,front:w.center[2]>0})),bounds:[(hi[0]-lo[0])*scale,(hi[1]-lo[1])*scale,length],triangles:total};
  const json=Buffer.from(JSON.stringify(header)),prefix=Buffer.alloc(12);prefix.write('DRHQ0001');prefix.writeUInt32LE(json.length,8);
  const packageBytes=Buffer.concat([prefix,json,...blobs]);
  fs.writeFileSync(path.join(output,`car${car.slot}.carhq`),packageBytes);
  fs.mkdirSync('web/car-pack',{recursive:true});
  fs.writeFileSync(path.join('web/car-pack',`car${car.slot}.carhq`),packageBytes);
  fs.writeFileSync(path.join(output,`${car.id}.json`),JSON.stringify({triangles:total,wheels:counts,textures:images.length,materials:materials.map(m=>({name:m.name,base:m.base,metal:m.metallic,rough:m.roughness,glass:m.glass})),bytes:offset},null,2));
  console.log(JSON.stringify({id:car.id,triangles:total,wheels:counts,textures:images.length,mb:(offset/1048576).toFixed(1)}));
}
