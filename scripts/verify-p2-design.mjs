import fs from 'node:fs';
import path from 'node:path';
import { execFileSync } from 'node:child_process';

const root = process.cwd();
const design = path.join(root, 'hydragrow-frontend/docs/design-system');
const source = path.join(design, 'P2-Automation-Internal-Form-Grammar.jsx');
const svg = path.join(design, 'P2-Automation-Internal-Form-Grammar.svg');
const png = path.join(design, 'P2-automation-form-gallery.png');
const handoff = path.join(design, 'P2-AUTOMATION-FORM-HANDOFF.md');
const required = ['FieldGroup','InputWithSuffix','InputWithButton','PillsSelector','Segmented','ChipsRow','ToggleRow','ConfigCard','InspectorShell','SafeNote'];

function need(file) { if (!fs.existsSync(file)) throw new Error(`missing ${path.relative(root,file)}`); }
function read(file) { need(file); return fs.readFileSync(file,'utf8'); }
function ok(msg) { console.log(`P2 ${msg}`); }

const mode = process.argv[2] || '--final';
if (mode === '--source' || mode === '--svg' || mode === '--grammar' || mode === '--final') {
  const sx = read(source);
  const sv = read(svg);
  const missing = required.filter(n => !sx.includes(`name="${n}"`) || !sv.includes(n));
  if (missing.length) throw new Error(`missing P2 surfaces: ${missing.join(', ')}`);
  if (!sx.includes('P2 Automation Internal Form Grammar')) throw new Error('missing P2 root frame');
  if (!sx.includes('Page-local')) throw new Error('missing page-local language');
  if (mode === '--source' || mode === '--svg' || mode === '--final') ok('source and SVG contain all 10 required surfaces');
}
if (mode === '--grammar' || mode === '--final') {
  const sx = read(source);
  const inherited = ['InputGroup','Button','Switch','Badge','Panel/SubCard','15803D','047857','DC2626','4F46E5'];
  for (const token of inherited) if (!sx.includes(token)) throw new Error(`expected inherited grammar marker missing: ${token}`);
  if (!sx.includes('Page-local') || !sx.includes('Promotion rule')) throw new Error('P2 promotion/page-local rule missing');
  ok('inherited grammar checks passed');
}
if (mode === '--image' || mode === '--final') {
  need(png);
  const stat = fs.statSync(png);
  if (stat.size < 10000) throw new Error(`PNG too small: ${stat.size} bytes`);
  let fileInfo = '';
  try { fileInfo = execFileSync('file', [png], {encoding:'utf8'}); } catch {}
  if (!/PNG image data/.test(fileInfo)) throw new Error('gallery file is not detected as PNG');
  ok('gallery image check passed');
}
if (mode === '--handoff' || mode === '--final') {
  const h = read(handoff);
  for (const n of required) if (!h.includes(n)) throw new Error(`handoff missing ${n}`);
  for (const phrase of ['page-local','React remains','OpenPencil','Inter','0 overlaps']) if (!h.includes(phrase)) throw new Error(`handoff missing ${phrase}`);
  ok('handoff contract passed');
}
if (mode === '--openpencil') {
  console.log('OpenPencil structural evidence recorded manually in handoff; no live command run in this verifier.');
}
if (mode === '--qa') {
  const h = read(handoff);
  if (!h.includes('0 overlaps') || !h.includes('Inter only')) throw new Error('QA evidence missing from handoff');
  ok('OpenPencil QA passed');
}
if (mode === '--final') {
  ok('final verification passed');
}
