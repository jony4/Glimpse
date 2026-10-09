// Optional asset generation: npm install --no-save --prefix /tmp/glimpse-icons sharp
// NODE_PATH=/tmp/glimpse-icons/node_modules node scripts/render-icons.mjs
// Then: iconutil -c icns /tmp/Glimpse.iconset -o assets/macos/Glimpse.icns
// Normal builds use the checked-in PNG/ICNS and do not require Node.js.
import { createRequire } from 'node:module';
import { mkdir, readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
const require = createRequire(import.meta.url);
const sharp = require('sharp');
const source = await readFile(fileURLToPath(new URL('../assets/branding/glimpse.svg', import.meta.url)));
await mkdir('/tmp/Glimpse.iconset', { recursive: true });
await sharp(source).resize(1024, 1024).png().toFile(fileURLToPath(new URL('../assets/branding/glimpse.png', import.meta.url)));
for (const size of [16, 32, 128, 256, 512]) {
  for (const scale of [1, 2]) {
    const suffix = scale === 2 ? '@2x' : '';
    await sharp(source).resize(size * scale, size * scale).png().toFile(`/tmp/Glimpse.iconset/icon_${size}x${size}${suffix}.png`);
  }
}
