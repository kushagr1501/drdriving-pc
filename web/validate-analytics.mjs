// DOM/queue fixtures, not browser or Vercel dashboard verification.
import fs from 'node:fs';
import vm from 'node:vm';
import assert from 'node:assert/strict';
const source=fs.readFileSync(new URL('./analytics.js',import.meta.url),'utf8');
function fixture({host='dr-driving.meowsy.space',dnt=false,gpc=false}={}){
  const scripts=[];
  const context=vm.createContext({URL,location:{hostname:host},navigator:{doNotTrack:dnt?'1':'0',globalPrivacyControl:gpc},window:{},document:{getElementById:id=>scripts.find(s=>s.id===id),createElement:()=>({dataset:{}}),head:{appendChild:s=>scripts.push(s)}}});
  vm.runInContext(source,context);return {context,scripts};
}
const production=fixture();assert.equal(production.scripts.length,1);
const script=production.scripts[0];
assert.equal(script.src,'https://dr-driving-analytics.vercel.app/collect/script.js');
assert.equal(script.dataset.viewEndpoint,'https://dr-driving-analytics.vercel.app/collect/view');
assert(script.async&&script.defer);
assert.equal(production.context.window.vaq.length,1);
assert.equal(production.context.window.vaq[0][0],'beforeSend');
const beforeSend=production.context.window.vaq[0][1];
assert.equal(beforeSend({type:'pageview',url:'https://dr-driving.meowsy.space/?db=private-test-value#name'}).url,'https://dr-driving.meowsy.space/');
assert.equal(beforeSend({type:'pageview',url:'invalid'}),null);
assert.doesNotThrow(()=>script.onerror());
vm.runInContext(source,production.context);assert.equal(production.scripts.length,1,'Duplicate loader');
for(const options of [{host:'localhost'},{host:'127.0.0.1'},{host:'preview.vercel.app'},{host:'kushagr1501.github.io'},{dnt:true},{gpc:true}]){
  const f=fixture(options);assert.equal(f.scripts.length,0);assert.equal(f.context.window.va,undefined);
}
console.log('Analytics fixtures passed: production-only async loader, one initialization, URL redaction, privacy opt-out, no local/preview traffic, and harmless load failure.');
