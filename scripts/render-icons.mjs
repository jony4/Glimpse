// Optional asset generation: install sharp and expose it through NODE_PATH.
// node scripts/render-icons.mjs
// Normal builds use the checked-in PNG/ICNS and do not require Node.js.
import { createRequire } from 'node:module';
import { mkdir, readFile, mkdtemp, rm } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { execFileSync } from 'node:child_process';
const require = createRequire(import.meta.url);
const sharp = require('sharp');
const source = await readFile(fileURLToPath(new URL('../assets/branding/glim.svg', import.meta.url)), 'utf8');
// Match standard macOS app artwork: an 824px tile in a 1024px canvas.
// Both icons share this silhouette and inset. The preview badge stays inside it.
const scale = 824 / 960;
const inset = 100 - 32 * scale;
const dockSource = source
  .replace('<path d="M388', '<g transform="translate(512 512) scale(1.22) translate(-512 -512)"><path d="M388')
  .replace('</svg>', '</g></svg>')
  .replace(/(<svg[^>]*>)/, `$1<g transform="translate(${inset} ${inset}) scale(${scale})">`)
  .replace('</svg>', '</g></svg>');
const previewSource = dockSource.replace('</svg>', `
  <circle cx="816" cy="816" r="74" fill="#A6E8D9" stroke="#F2F2F2" stroke-width="6"/>
  <ellipse cx="816" cy="816" rx="43" ry="26" fill="none" stroke="#142B38" stroke-width="7"/>
  <circle cx="816" cy="816" r="11" fill="#142B38"/>
</svg>`);
const directory = await mkdtemp(join(tmpdir(), 'glim-icons-'));
try {
  await sharp(Buffer.from(source)).resize(1024, 1024).png().toFile(fileURLToPath(new URL('../assets/branding/glim.png', import.meta.url)));
  for (const [name, svg] of [['Glim', dockSource], ['GlimPreview', previewSource]]) {
    const iconset = join(directory, `${name}.iconset`);
    await mkdir(iconset);
    for (const size of [16, 32, 128, 256, 512]) {
      for (const density of [1, 2]) {
        const suffix = density === 2 ? '@2x' : '';
        await sharp(Buffer.from(svg)).resize(size * density, size * density).png()
          .toFile(join(iconset, `icon_${size}x${size}${suffix}.png`));
      }
    }
    execFileSync('iconutil', ['-c', 'icns', iconset, '-o', fileURLToPath(new URL(`../assets/macos/${name}.icns`, import.meta.url))]);
  }
} finally {
  await rm(directory, { recursive: true, force: true });
}
