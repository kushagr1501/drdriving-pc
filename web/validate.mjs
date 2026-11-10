// Browser-bridge unit checks with an explicit DOM fixture, not browser/WebGL QA.
import fs from 'node:fs';
import vm from 'node:vm';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import assert from 'node:assert/strict';
const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
const dist=path.join(root,'web/dist');
const html=fs.readFileSync(path.join(dist,'index.html'),'utf8');
assert(!/__ASSET_BYTES__|__SITE_ORIGIN__/.test(html),'Unresolved build placeholder');
assert(!html.includes('krishnassh.github.io'),'Stale upstream deployment URL');
const bundle=fs.readFileSync(path.join(dist,'assets.bin'));let offset=0;const files=new Map();
while(offset<bundle.length){
  const n=bundle.readUInt16LE(offset);offset+=2;
  const name=bundle.subarray(offset,offset+n).toString();offset+=n;
  const size=bundle.readUInt32LE(offset);offset+=4;
  assert(offset+size<=bundle.length);assert(!files.has(name));files.set(name,bundle.subarray(offset,offset+size));offset+=size;
}
assert.equal(offset,bundle.length);
for(const name of ['ui.png','envi/car-reflections.png','envi/car-reflections-blur.png','sounds/engine-rpm-7.wav','sounds/motor-rpm-7.wav'])assert(files.has(name),`Missing bundled ${name}`);
const cars=JSON.parse(fs.readFileSync(path.join(dist,'assets/cars/catalog.json')));
assert.equal(cars.length,7);assert.equal(new Set(cars.map(c=>c.slot)).size,7);
for(const car of cars){
  let original;
  for(const file of [car.path,car.mobilePath]){
    assert(file.startsWith('assets/cars/')&&!file.includes('..'));
    const b=fs.readFileSync(path.join(dist,file));assert.equal(b.subarray(0,8).toString(),'DRHQ0001');
    const h=JSON.parse(b.subarray(12,12+b.readUInt32LE(8))),blob=b.subarray(12+b.readUInt32LE(8));
    assert.equal(h.name,car.name);assert.equal(h.wheels.length,4);
    let triangles=0;
    for(const part of h.parts){
      const v=part.vertices,i=part.indices;
      assert(v.offset+v.length<=blob.length&&i.offset+i.length<=blob.length);
      assert.equal(v.length,part.vertexCount*48);assert.equal(i.length,part.indexCount*4);assert.equal(part.indexCount%3,0);
      for(let n=0;n<v.length;n+=4)assert(Number.isFinite(blob.readFloatLE(v.offset+n)));
      for(let n=0;n<i.length;n+=4)assert(blob.readUInt32LE(i.offset+n)<part.vertexCount);
      triangles+=part.indexCount/3;
    }
    assert.equal(triangles,h.triangles);
    if(file===car.path)original=h;
    else{
      assert.deepEqual(h.wheels,original.wheels,'Mobile wheel pivots/steering changed');
      assert.deepEqual(h.materials,original.materials,'Mobile material bindings changed');
      assert.deepEqual(h.bounds,original.bounds);
      assert.deepEqual(h.parts.map(p=>[p.material,p.wheel]),original.parts.map(p=>[p.material,p.wheel]));
      assert(h.triangles<=80000,'Mobile car exceeds geometry budget');
      if(original.triangles>70000)assert(h.triangles<original.triangles*0.65,'Dense car was not simplified');
      for(const image of h.images){
        const png=blob.subarray(image.offset,image.offset+image.length);
        assert.equal(png.subarray(1,4).toString(),'PNG');assert(png.readUInt32BE(16)<=1024&&png.readUInt32BE(20)<=1024);
      }
    }
  }
}
const manifest=JSON.parse(fs.readFileSync(path.join(dist,'manifest.webmanifest')));
assert.equal(manifest.display,'standalone');assert.equal(manifest.start_url,'./');
assert(html.includes('rel="manifest"')&&html.includes('rel="apple-touch-icon"'));
const module=new WebAssembly.Module(fs.readFileSync(path.join(dist,'drdriving.wasm')));
const ids=new Map();
const element=id=>{if(!ids.has(id))ids.set(id,{id,hidden:id==='display-settings',style:{setProperty(k,v){this[k]=v}},dataset:{},textContent:'',listeners:{},getContext(type,attributes){this.contextAttributes=attributes;return{}},focus(){this.focused=true},setAttribute(k,v){this[k]=v},removeAttribute(k){delete this[k]},addEventListener(event,fn){this.listeners[event]=fn}});return ids.get(id)};
const storage=new Map([['drdriving.save','original progression']]);let plugin;
const context=vm.createContext({console,TextEncoder,TextDecoder,Uint8Array,URLSearchParams,WebAssembly,
  wasm_memory:new WebAssembly.Memory({initial:2}),location:{search:'',reload(){}},
  navigator:{userAgent:'Desktop',platform:'Win32',maxTouchPoints:0},
  document:{getElementById:element,documentElement:element('root'),addEventListener(){}},window:{innerHeight:720,addEventListener(){}},
  localStorage:{getItem:k=>storage.get(k)??null,setItem:(k,v)=>storage.set(k,v)},
  fetch:async()=>({ok:true,headers:{get:()=>null},body:{getReader:()=>({read:async()=>({done:true})})}}),
  miniquad_add_plugin:p=>{plugin=p},load:()=>{},setTimeout:()=>{},screen:{}});
new vm.Script(fs.readFileSync(path.join(dist,'display.js'),'utf8'),{filename:'display.js'}).runInContext(context);
new vm.Script(fs.readFileSync(path.join(dist,'launcher.js'),'utf8'),{filename:'launcher.js'}).runInContext(context);
for(const [index,match] of [...html.matchAll(/<script(?:\s[^>]*)?>([\s\S]*?)<\/script>/g)].entries()){
  if(match[1].trim())new vm.Script(match[1],{filename:`inline-${index}.js`}).runInContext(context);
}
assert(plugin);const imports={env:{}};plugin.register_plugin(imports);
for(const i of WebAssembly.Module.imports(module).filter(i=>i.name.startsWith('dr_')))assert.equal(typeof imports.env[i.name],'function',`Missing browser import ${i.name}`);
assert.equal(imports.env.dr_mobile_rendering(),0,'Desktop Auto must retain High');
assert.equal(imports.env.dr_game_active(),0,'Do not render underneath the start page');
const text=new TextEncoder().encode('{"coin":2147483647,"name":"Driver"}');
new Uint8Array(context.wasm_memory.buffer,0,text.length).set(text);
imports.env.dr_storage_set(0,text.length);
assert.equal(storage.get('drdriving.save'),'original progression');
assert.equal(storage.get('drdriving.sandbox.save.v1'),new TextDecoder().decode(text));
assert.equal(imports.env.dr_storage_get(200,1),-text.length);
assert.equal(imports.env.dr_storage_get(200,1024),text.length);
assert.equal(new TextDecoder().decode(new Uint8Array(context.wasm_memory.buffer,200,text.length)),new TextDecoder().decode(text));
assert.equal(element('play').disabled,true);
element('play').listeners.click();assert.notEqual(element('welcome').style.display,'none','Play must wait for the game');
imports.env.dr_game_ready();assert.equal(element('play').disabled,false);
element('touch-tab').listeners.click();assert.equal(element('keyboard-panel').hidden,true);assert.equal(element('touch-panel').hidden,false);
element('keyboard-tab').listeners.click();assert.equal(element('keyboard-panel').hidden,false);
element('play').listeners.click();assert.equal(element('welcome').style.display,'none');assert(element('glcanvas').focused);
assert.equal(imports.env.dr_game_active(),1);
console.log(`Valid WASM, ${files.size} bundled assets, seven HQ/mobile car pairs with matching rigs, complete browser imports, isolated save round-trip, and play-screen fixture checks passed.`);
