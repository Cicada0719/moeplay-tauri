const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');
const dir = 'src/lib/api';
const mods = ['games','scraper','saves','metadata','settings','dashboard','tasks','downloads','format','platformImport','emulators','system'];
const headIndex = execSync('git show HEAD:src/lib/api/index.ts').toString();
const m = headIndex.match(/import type \{([\s\S]*?)\} from [\"']..\/types[\"'];?/);
if (!m) { console.log('TYPE LINE NOT FOUND'); process.exit(1); }
const TYPES = Array.from(m[1].matchAll(/\b([A-Za-z_][A-Za-z0-9_]*)\b/g)).map((x) => x[1]);
console.log('types:', TYPES.length);
for (const mod of mods) {
  const file = path.join(dir, mod + '.ts');
  const text = fs.readFileSync(file, 'utf8');
  if (text.includes('from "./types";')) continue;
  const localTypes = new Set();
  for (const mm of text.matchAll(/(?:export )?(?:type|interface) ([A-Za-z_][A-Za-z0-9_]*)/g)) localTypes.add(mm[1]);
  const used = TYPES.filter((t) => !localTypes.has(t) && new RegExp('\\b' + t + '\\b').test(text));
  const typeImport = used.length ? '\nimport type { ' + used.join(', ') + ' } from "./types";' : '';
  const updated = text.replace(/import \{ invokeCmd \} from "\.\/core";/, 'import { invokeCmd } from "./core";' + typeImport);
  fs.writeFileSync(file, updated);
  console.log(mod + ': +' + used.length + ' types');
}
