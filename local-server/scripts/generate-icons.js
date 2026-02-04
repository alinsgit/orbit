import { Resvg } from '@resvg/resvg-js';
import sharp from 'sharp';
import { readFileSync, writeFileSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const iconsDir = join(__dirname, '../src-tauri/icons');
const svgPath = join(iconsDir, 'orbit-logo.svg');

async function generateIcons() {
  console.log('Reading SVG...');
  const svg = readFileSync(svgPath, 'utf8');

  // Render SVG to PNG at high resolution
  const resvg = new Resvg(svg, {
    fitTo: { mode: 'width', value: 512 },
  });
  const pngData = resvg.render();
  const pngBuffer = pngData.asPng();

  console.log('Generated 512x512 base PNG');

  // Generate different sizes
  const sizes = [
    { name: 'icon.png', size: 512 },
    { name: '128x128@2x.png', size: 256 },
    { name: '128x128.png', size: 128 },
    { name: '32x32.png', size: 32 },
    { name: 'Square310x310Logo.png', size: 310 },
    { name: 'Square284x284Logo.png', size: 284 },
    { name: 'Square150x150Logo.png', size: 150 },
    { name: 'Square142x142Logo.png', size: 142 },
    { name: 'Square107x107Logo.png', size: 107 },
    { name: 'Square89x89Logo.png', size: 89 },
    { name: 'Square71x71Logo.png', size: 71 },
    { name: 'Square44x44Logo.png', size: 44 },
    { name: 'Square30x30Logo.png', size: 30 },
    { name: 'StoreLogo.png', size: 50 },
  ];

  for (const { name, size } of sizes) {
    const resized = await sharp(pngBuffer)
      .resize(size, size)
      .png()
      .toBuffer();
    writeFileSync(join(iconsDir, name), resized);
    console.log(`Generated ${name} (${size}x${size})`);
  }

  // Generate ICO (Windows) - contains multiple sizes
  const icoSizes = [16, 24, 32, 48, 64, 128, 256];
  const icoBuffers = await Promise.all(
    icoSizes.map(size =>
      sharp(pngBuffer).resize(size, size).png().toBuffer()
    )
  );

  // Simple ICO file format
  const icoBuffer = createIco(icoBuffers, icoSizes);
  writeFileSync(join(iconsDir, 'icon.ico'), icoBuffer);
  console.log('Generated icon.ico');

  console.log('Done! All icons generated.');
}

function createIco(pngBuffers, sizes) {
  // ICO header: 6 bytes
  // ICO directory entries: 16 bytes each
  // PNG data follows

  const numImages = pngBuffers.length;
  const headerSize = 6 + (numImages * 16);

  let offset = headerSize;
  const entries = [];

  for (let i = 0; i < numImages; i++) {
    const size = sizes[i];
    const pngSize = pngBuffers[i].length;

    entries.push({
      width: size >= 256 ? 0 : size,
      height: size >= 256 ? 0 : size,
      colors: 0,
      reserved: 0,
      planes: 1,
      bpp: 32,
      size: pngSize,
      offset: offset
    });

    offset += pngSize;
  }

  const buffer = Buffer.alloc(offset);

  // ICO header
  buffer.writeUInt16LE(0, 0);      // Reserved
  buffer.writeUInt16LE(1, 2);      // Type: 1 = ICO
  buffer.writeUInt16LE(numImages, 4); // Number of images

  // Directory entries
  let pos = 6;
  for (const entry of entries) {
    buffer.writeUInt8(entry.width, pos);
    buffer.writeUInt8(entry.height, pos + 1);
    buffer.writeUInt8(entry.colors, pos + 2);
    buffer.writeUInt8(entry.reserved, pos + 3);
    buffer.writeUInt16LE(entry.planes, pos + 4);
    buffer.writeUInt16LE(entry.bpp, pos + 6);
    buffer.writeUInt32LE(entry.size, pos + 8);
    buffer.writeUInt32LE(entry.offset, pos + 12);
    pos += 16;
  }

  // PNG data
  for (const pngBuffer of pngBuffers) {
    pngBuffer.copy(buffer, pos);
    pos += pngBuffer.length;
  }

  return buffer;
}

generateIcons().catch(console.error);
