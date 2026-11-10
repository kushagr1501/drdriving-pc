#!/usr/bin/env node
// Portable browser build and asset packager. Run from any directory with Node.js and Rustup.
import fs from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { execFileSync } from 'node:child_process';
import {createHash} from 'node:crypto';

const args = new Set(process.argv.slice(2));
if (args.has('--help')) {
  console.log('node web/build.mjs [--no-build]\nBuild and package the browser game into web/dist. --no-build packages an existing release WASM.');
  process.exit(0);
}
for (const arg of args) {
  if (arg !== '--no-build') throw new Error(`Unknown argument: ${arg}`);
}

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const out = path.join(root, 'web', 'dist');
const wasm = path.join(root, 'target', 'wasm32-unknown-unknown', 'release', 'drdriving.wasm');
const maxFileBytes = 25 * 1024 * 1024;
const looseAssets = new Set(['logo.png', 'fonts/Roboto-Regular.ttf']);
const slots = [20, 21, 22, 23, 25, 26, 27];

function cargo(argv, options = {}) {
  return execFileSync('cargo', argv, { cwd: root, stdio: 'inherit', windowsHide: true, ...options });
}

function checkSize(name, bytes) {
  if (bytes > maxFileBytes) throw new Error(`${name} exceeds the 25 MiB static hosting file limit (${bytes} bytes).`);
}

async function walk(directory, relative = '') {
  const entries = await fs.readdir(directory, { withFileTypes: true });
  entries.sort((a, b) => a.name.localeCompare(b.name, 'en'));
  const files = [];
  for (const entry of entries) {
    const rel = relative ? `${relative}/${entry.name}` : entry.name;
    if (entry.isDirectory()) files.push(...await walk(path.join(directory, entry.name), rel));
    else if (entry.isFile()) files.push(rel);
    else throw new Error(`Unsupported asset entry: ${rel}`);
  }
  return files;
}

if (!args.has('--no-build')) {
  cargo(['build', '--locked', '--release', '-p', 'drdriving', '--target', 'wasm32-unknown-unknown']);
}

// Resolve the JS glue from the exact Macroquad package selected by Cargo.lock.
const metadata = JSON.parse(cargo(['metadata', '--format-version', '1', '--locked'], { stdio: ['ignore', 'pipe', 'inherit'], encoding: 'utf8' }));
const macroquad = metadata.packages.find(p => p.name === 'macroquad');
if (!macroquad) throw new Error('Macroquad was not found in Cargo metadata.');
const glue = path.join(path.dirname(macroquad.manifest_path), 'js', 'mq_js_bundle.js');

// Read and validate every input before replacing the existing generated output.
const outputs = new Map();
let customDomain='';
try{customDomain=(await fs.readFile(path.join(root,'web','CNAME'),'utf8')).trim();}catch(error){if(error.code!=='ENOENT')throw error;}
if(customDomain){
  if(!/^[a-z0-9]+(?:[a-z0-9.-]*[a-z0-9])?$/i.test(customDomain)||!customDomain.includes('.')||customDomain.includes('..'))throw new Error('web/CNAME must contain a domain name only.');
  outputs.set('CNAME',Buffer.from(customDomain+'\n'));
}
outputs.set('drdriving.wasm', await fs.readFile(wasm));
outputs.set('mq_js_bundle.js', await fs.readFile(glue));
for (const name of ['og.png', 'icon.png']) outputs.set(name, await fs.readFile(path.join(root, 'web', name)));
for (const name of ['launcher.css','launcher.js','display.css','display.js','manifest.webmanifest','porsche.webp']) outputs.set(name,await fs.readFile(path.join(root,'web',name)));
outputs.set('CREDITS.md', await fs.readFile(path.join(root, 'tools', 'car-quality', 'CREDITS.md')));
outputs.set('LICENSE.txt', await fs.readFile(path.join(root, 'LICENSE')));

const records = [];
let bundledFiles = 0;
for (const rel of await walk(path.join(root, 'assets'))) {
  const data = await fs.readFile(path.join(root, 'assets', ...rel.split('/')));
  if (looseAssets.has(rel)) {
    outputs.set(`assets/${rel}`, data);
    continue;
  }
  const name = Buffer.from(rel, 'utf8');
  if (name.length > 0xffff || data.length > 0xffffffff) throw new Error(`Asset cannot fit the bundle format: ${rel}`);
  // Identical to build.sh: [u16 UTF-8 path length][path][u32 data length][data].
  const prefix = Buffer.alloc(2);
  prefix.writeUInt16LE(name.length);
  const size = Buffer.alloc(4);
  size.writeUInt32LE(data.length);
  records.push(prefix, name, size, data);
  bundledFiles++;
}
const bundle = Buffer.concat(records);
outputs.set('assets.bin', bundle);

// Keep detailed cars outside assets.bin: each stays well below static hosting limits,
// and the browser can fetch them without copying them into the stock asset bundle.
const catalog = [];
for (const slot of slots) {
  const name = `car${slot}.carhq`;
  const data = await fs.readFile(path.join(root, 'web', 'car-pack', name));
  if (data.length < 12 || data.subarray(0, 8).toString('ascii') !== 'DRHQ0001') throw new Error(`Invalid detailed car package: ${name}`);
  const headerLength = data.readUInt32LE(8);
  if (headerLength > 1024 * 1024 || headerLength > data.length - 12) throw new Error(`Invalid detailed car header: ${name}`);
  const header = JSON.parse(data.subarray(12, 12 + headerLength).toString('utf8'));
  if (header.version !== 1 || typeof header.name !== 'string') throw new Error(`Unsupported detailed car: ${name}`);
  const target = `assets/cars/${name}`;
  const mobileTarget = `assets/cars/mobile/${name}`;
  const mobile = await fs.readFile(path.join(root,'web','car-pack','mobile',name));
  if (mobile.subarray(0,8).toString('ascii') !== 'DRHQ0001') throw new Error(`Invalid mobile car: ${name}`);
  outputs.set(target, data);
  outputs.set(mobileTarget, mobile);
  catalog.push({ slot, name: header.name, path: target, mobilePath: mobileTarget });
}
outputs.set('assets/cars/catalog.json', Buffer.from(`${JSON.stringify(catalog, null, 2)}\n`));
const html = await fs.readFile(path.join(root, 'web', 'index.html'), 'utf8');
if (!html.includes('__ASSET_BYTES__')) throw new Error('web/index.html is missing its asset-size placeholder.');
const origin=(customDomain?'https://'+customDomain:(process.env.DRDRIVING_SITE_URL??process.env.DRDRIVING_SITE_ORIGIN??'')).replace(/\/+$/,'');
let page=html.replaceAll('__ASSET_BYTES__',String(bundle.length)).replaceAll('__SITE_ORIGIN__',origin);
// Keep the readiness bridge, styles and compiled game on the same release when
// a player refreshes a previously cached GitHub Pages tab.
for(const name of ['launcher.css','launcher.js','display.css','display.js','porsche.webp','mq_js_bundle.js','drdriving.wasm','assets.bin']){
  const revision=createHash('sha256').update(outputs.get(name)).digest('hex').slice(0,12);
  page=page.replaceAll('"'+name+'"','"'+name+'?v='+revision+'"');
}
outputs.set('index.html',Buffer.from(page));
outputs.set('.nojekyll', Buffer.alloc(0));

for (const [name, data] of outputs) checkSize(name, data.length);
// Never recursively remove an arbitrary caller-provided path.
if (path.relative(root, out) !== path.join('web', 'dist')) throw new Error('Output must remain inside the repository web/dist directory.');
await fs.rm(out, { recursive: true, force: true });
await fs.mkdir(out, { recursive: true });
for (const [name, data] of outputs) {
  const destination = path.join(out, ...name.split('/'));
  await fs.mkdir(path.dirname(destination), { recursive: true });
  await fs.writeFile(destination, data);
}
const total = [...outputs.values()].reduce((sum, data) => sum + data.length, 0);
console.log(`Stock bundle: ${bundledFiles} assets, ${(bundle.length / 1048576).toFixed(1)} MiB`);
console.log(`Detailed cars: ${catalog.length} separate packages`);
console.log(`Built ${out}: ${outputs.size} files, ${(total / 1048576).toFixed(1)} MiB total; every file <= 25 MiB.`);
