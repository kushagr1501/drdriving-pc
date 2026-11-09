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
for(const car of cars){const b=fs.readFileSync(path.join(dist,car.path));assert.equal(b.subarray(0,8).toString(),'DRHQ0001');const h=JSON.parse(b.subarray(12,12+b.readUInt32LE(8)));assert.equal(h.name,car.name);assert.equal(h.wheels.length,4);}
const module=new WebAssembly.Module(fs.readFileSync(path.join(dist,'drdriving.wasm')));
const ids=new Map();
const element=id=>{if(!ids.has(id))ids.set(id,{style:{},textContent:'',listeners:{},focus(){this.focused=true},addEventListener(event,fn){this.listeners[event]=fn}});return ids.get(id)};
const storage=new Map([['drdriving.save','original progression']]);let plugin;
const context=vm.createContext({console,TextEncoder,TextDecoder,Uint8Array,URLSearchParams,WebAssembly,
  wasm_memory:new WebAssembly.Memory({initial:2}),location:{search:'',reload(){}},
  document:{getElementById:element,documentElement:{},addEventListener(){}},window:{addEventListener(){}},
  localStorage:{getItem:k=>storage.get(k)??null,setItem:(k,v)=>storage.set(k,v)},
  fetch:async()=>({ok:true,headers:{get:()=>null},body:{getReader:()=>({read:async()=>({done:true})})}}),
  miniquad_add_plugin:p=>{plugin=p},load:()=>{},setTimeout:()=>{},screen:{}});
for(const [index,match] of [...html.matchAll(/<script(?:\s[^>]*)?>([\s\S]*?)<\/script>/g)].entries()){
  if(match[1].trim())new vm.Script(match[1],{filename:`inline-${index}.js`}).runInContext(context);
}
assert(plugin);const imports={env:{}};plugin.register_plugin(imports);
for(const i of WebAssembly.Module.imports(module).filter(i=>i.name.startsWith('dr_')))assert.equal(typeof imports.env[i.name],'function',`Missing browser import ${i.name}`);
const text=new TextEncoder().encode('{"coin":2147483647,"name":"Driver"}');
new Uint8Array(context.wasm_memory.buffer,0,text.length).set(text);
imports.env.dr_storage_set(0,text.length);
assert.equal(storage.get('drdriving.save'),'original progression');
assert.equal(storage.get('drdriving.sandbox.save.v1'),new TextDecoder().decode(text));
assert.equal(imports.env.dr_storage_get(200,1),-text.length);
assert.equal(imports.env.dr_storage_get(200,1024),text.length);
assert.equal(new TextDecoder().decode(new Uint8Array(context.wasm_memory.buffer,200,text.length)),new TextDecoder().decode(text));
element('play').listeners.click();assert.equal(element('welcome').style.display,'none');assert(element('glcanvas').focused);
console.log(`Valid WASM, ${files.size} bundled assets, seven detailed cars, complete browser imports, isolated save round-trip, and play-screen fixture checks passed.`);
