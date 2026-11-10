// Capability tests with a DOM fixture, not real Safari/WebGL or phone FPS tests.
import fs from 'node:fs';
import vm from 'node:vm';
import assert from 'node:assert/strict';
const source=fs.readFileSync(new URL('./display.js',import.meta.url),'utf8');
function fixture({coarse=false,userAgent='Desktop',platform='Win32',touches=0,stored=null,request,standalone=false,blockedStorage=false}={}){
  const elements=new Map(),events=new Map(),writes=new Map();let reloaded=false,observed;
  const element=id=>{
    if(!elements.has(id))elements.set(id,{id,hidden:id==='display-settings',listeners:{},style:{setProperty(k,v){this[k]=v}},getContext(type,options){return {type,options}},focus(){document.activeElement=this},setAttribute(k,v){this[k]=v},addEventListener(type,fn){this.listeners[type]=fn}});
    return elements.get(id);
  };
  const root=element('root');root.requestFullscreen=request;
  const document={documentElement:root,getElementById:element,activeElement:element('fs'),addEventListener(type,fn){events.set(type,fn)}};
  const viewport={height:390,addEventListener(type,fn){this[type]=fn}};
  const window={innerHeight:400,visualViewport:viewport,matchMedia:q=>({matches:q==='(pointer:coarse)'?coarse:standalone}),addEventListener(){},ResizeObserver:true};
  const context=vm.createContext({document,window,navigator:{userAgent,platform,maxTouchPoints:touches,standalone},screen:{},ResizeObserver:class{constructor(fn){this.callback=fn;}observe(el){observed=el;}},localStorage:{getItem(){return stored},setItem(k,v){if(blockedStorage)throw Error('blocked');writes.set(k,v)}},location:{reload(){reloaded=true}}});
  vm.runInContext(source,context);
  return {display:context.drDisplay,element,document,events,viewport,writes,observed,get reloaded(){return reloaded}};
}
for(const device of [{coarse:true,userAgent:'iPhone'},{coarse:true,userAgent:'Android'},{platform:'MacIntel',touches:5}]){
  const f=fixture(device);assert.equal(f.display.lightweight,true);
  assert.equal(f.element('glcanvas').getContext('webgl').options.antialias,false);
  assert.equal(f.element('glcanvas').getContext('webgl2').options.antialias,false);
  f.element('fs').listeners.click({preventDefault(){}});assert.equal(f.element('display-settings').hidden,false,'Unsupported fullscreen must open guidance');
  assert.equal(f.element('screen-steps').hidden,false);assert.equal(f.observed.id,'game-viewport');
  assert.equal(f.element('root').style['--app-height'],'390px');
  f.viewport.height=300;f.viewport.resize();assert.equal(f.element('root').style['--app-height'],'300px');
  f.events.get('keydown')({key:'Escape',preventDefault(){},stopPropagation(){}});assert.equal(f.element('display-settings').hidden,true);
}
const desktop=fixture();assert.equal(desktop.display.lightweight,false);assert.equal(desktop.element('glcanvas').getContext('webgl').options,undefined);
assert.equal(fixture({coarse:true,stored:'high'}).display.lightweight,false);
assert.equal(fixture({stored:'performance'}).display.lightweight,true);
assert.equal(fixture({coarse:true,stored:'invalid'}).display.lightweight,true);
const home=fixture({coarse:true,standalone:true});assert.equal(home.element('screen-steps').hidden,true);
assert.match(home.element('screen-help').textContent,/Opened from your Home Screen/);
for(const request of [()=>Promise.reject(Error('denied')),()=>{throw Error('denied')}]){
  const f=fixture({request});f.element('fs').listeners.click({preventDefault(){}});await Promise.resolve();assert.equal(f.element('display-settings').hidden,false,'Rejected fullscreen must fall back gracefully');
}
let calls=0;const supported=fixture({request:()=>{calls++;return Promise.resolve();}});
supported.element('fs').listeners.click({preventDefault(){}});await Promise.resolve();assert.equal(calls,1);assert.equal(supported.element('display-settings').hidden,true);
const change=fixture();change.element('graphics-quality').value='performance';change.element('graphics-quality').listeners.change();assert.equal(change.element('quality-apply').hidden,false);
change.element('quality-apply').listeners.click();assert.equal(change.writes.get('drdriving.graphics.v1'),'performance');assert(change.reloaded);
const blocked=fixture({blockedStorage:true});blocked.element('quality-apply').listeners.click();assert.equal(blocked.reloaded,false);
desktop.element('welcome').style.display='none';assert(desktop.display.active());desktop.document.hidden=true;assert.equal(desktop.display.active(),false);
console.log('Display fixtures passed: iPhone/iPad/Android Auto, High override, GL antialias attributes, viewport resize, fullscreen success/rejection/fallback, Home Screen mode, settings storage and background rendering.');
