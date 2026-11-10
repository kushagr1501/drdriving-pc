// Build phone LODs from the reviewed HQ packages; preserve materials and wheel rigs.
// Run: cd tools/car-quality; npm ci; node export-mobile-cars.mjs
import fs from 'node:fs/promises';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {MeshoptSimplifier} from 'meshoptimizer';
import sharp from 'sharp';

const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'../..');
const directory=path.join(root,'web/car-pack');
await MeshoptSimplifier.ready;
await fs.mkdir(path.join(directory,'mobile'),{recursive:true});
for(const slot of [20,21,22,23,25,26,27]) {
  const name=`car${slot}.carhq`, input=await fs.readFile(path.join(directory,name));
  const header=JSON.parse(input.subarray(12,12+input.readUInt32LE(8)));
  const blob=input.subarray(12+input.readUInt32LE(8));
  const read=range=>blob.subarray(range.offset,range.offset+range.length);
  const buffers=[];let offset=0;
  const append=data=>{const range={offset,length:data.length};buffers.push(data);offset+=data.length;return range;};
  const images=[];
  for(const image of header.images) {
    const png=await sharp(read(image)).resize({width:1024,height:1024,fit:'inside',withoutEnlargement:true}).png().toBuffer();
    images.push(append(png));
  }
  const parts=[];let triangles=0,maxError=0;
  const ratio=Math.min(1,24000/header.triangles);
  for(const part of header.parts) {
    // Copy into aligned storage: JSON/blob offsets need not be 4-byte aligned.
    const vertices=new Float32Array(Uint8Array.from(read(part.vertices)).buffer);
    let indices=new Uint32Array(Uint8Array.from(read(part.indices)).buffer);
    if(ratio<1 && indices.length>240) {
      const attributes=new Float32Array(part.vertexCount*5);
      for(let i=0;i<part.vertexCount;i++)attributes.set(vertices.subarray(i*12+3,i*12+8),i*5);
      const target=Math.max(240,Math.floor(indices.length*ratio/3)*3);
      // Protect the large body panels; denser rims occupy fewer screen pixels.
      // Bound appearance error instead of forcing every part to the budget.
      const tolerance=part.wheel>=0?0.008:0.004;
      const [simplified,error]=MeshoptSimplifier.simplifyWithAttributes(indices,vertices,12,attributes,5,[0.5,0.5,0.5,0.1,0.1],null,target,tolerance,['Permissive']);
      indices=simplified;maxError=Math.max(maxError,error);
    }
    const [remap,count]=MeshoptSimplifier.compactMesh(indices);
    const compact=new Float32Array(count*12);
    for(let old=0;old<remap.length;old++)if(remap[old]!==0xffffffff)compact.set(vertices.subarray(old*12,old*12+12),remap[old]*12);
    triangles+=indices.length/3;
    parts.push({...part,vertexCount:count,indexCount:indices.length,vertices:append(Buffer.from(compact.buffer)),indices:append(Buffer.from(indices.buffer))});
  }
  const json=Buffer.from(JSON.stringify({...header,images,parts,triangles,lod:{sourceTriangles:header.triangles,maxError,textureLimit:1024}}));
  const prefix=Buffer.alloc(12);prefix.write('DRHQ0001');prefix.writeUInt32LE(json.length,8);
  const output=Buffer.concat([prefix,json,...buffers]);
  await fs.writeFile(path.join(directory,'mobile',name),output);
  console.log(`${header.name}: ${header.triangles} -> ${triangles} triangles, ${(input.length/1048576).toFixed(1)} -> ${(output.length/1048576).toFixed(1)} MiB, error ${maxError.toFixed(4)}`);
}
