import fs from 'node:fs';
import assert from 'node:assert/strict';
for(const slot of [20,21,22,23,25,26,27]) {
  const file=`local-test-assets/quality-upgrade/car${slot}.carhq`,data=fs.readFileSync(file);
  assert.equal(data.subarray(0,8).toString(),'DRHQ0001');
  const length=data.readUInt32LE(8),header=JSON.parse(data.subarray(12,12+length));
  const blob=data.subarray(12+length);
  const slice=({offset,length})=>{assert(offset>=0&&length>=0&&offset+length<=blob.length);return blob.subarray(offset,offset+length)};
  assert.equal(header.version,1);assert.equal(header.wheels.length,4);
  assert.equal(header.wheels.filter(w=>w.front).length,2);
  for(const w of header.wheels)assert.equal(w.front,w.pivot[2]>0);
  for(const im of header.images){const p=slice(im);assert.equal(p.subarray(1,4).toString(),'PNG');assert(p.readUInt32BE(16)<=4096&&p.readUInt32BE(20)<=4096);}
  let triangles=0,zeroNormals=0;const wheelTriangles=[0,0,0,0];
  for(const part of header.parts){
    assert(part.material>=0&&part.material<header.materials.length);
    const vertices=slice(part.vertices),indices=slice(part.indices);
    assert.equal(vertices.length,part.vertexCount*48);assert.equal(indices.length,part.indexCount*4);
    assert.equal(part.indexCount%3,0);triangles+=part.indexCount/3;
    if(part.wheel>=0)wheelTriangles[part.wheel]+=part.indexCount/3;
    for(let offset=0;offset<indices.length;offset+=4)assert(indices.readUInt32LE(offset)<part.vertexCount);
    for(let offset=0;offset<vertices.length;offset+=48){
      const v=Array.from({length:12},(_,i)=>vertices.readFloatLE(offset+i*4));assert(v.every(Number.isFinite));
      const norm=Math.hypot(...v.slice(3,6));if(norm<0.1)zeroNormals++;else assert(Math.abs(norm-1)<0.002);
      assert(Math.abs(Math.abs(v[11])-1)<0.001);
    }
  }
  assert.equal(triangles,header.triangles);assert(wheelTriangles.every(n=>n>10));
  assert.equal(zeroNormals,0,'Zero normals would create unstable shading');
  console.log(`${header.name}: ${triangles} triangles, ${header.parts.length} material/axle groups, ${header.images.length} textures; valid`);
}
