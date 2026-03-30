import sharp from "sharp";
import { writeFileSync, mkdirSync } from "fs";
import { resolve, dirname } from "path";
import { fileURLToPath } from "url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = resolve(__dirname, "..");
const src = resolve(root, "icon.png");
const iconsDir = resolve(root, "src-tauri", "icons");

mkdirSync(iconsDir, { recursive: true });

// 各サイズのPNGバッファを生成
const sizes = [16, 24, 32, 48, 64, 128, 256];
const pngBuffers = [];
for (const size of sizes) {
  const buf = await sharp(src)
    .resize(size, size, { fit: "contain", background: { r: 0, g: 0, b: 0, alpha: 0 } })
    .png()
    .toBuffer();
  pngBuffers.push({ size, buf });
}

// ICOファイルを手動構築（PNGデータをそのまま格納）
function buildIco(images) {
  const headerSize = 6;
  const dirEntrySize = 16;
  const dirSize = dirEntrySize * images.length;
  let dataOffset = headerSize + dirSize;

  // ヘッダー
  const header = Buffer.alloc(headerSize);
  header.writeUInt16LE(0, 0);          // reserved
  header.writeUInt16LE(1, 2);          // type = ICO
  header.writeUInt16LE(images.length, 4);

  // ディレクトリエントリ
  const dirEntries = Buffer.alloc(dirSize);
  const dataChunks = [];

  for (let i = 0; i < images.length; i++) {
    const { size, buf } = images[i];
    const off = i * dirEntrySize;
    dirEntries.writeUInt8(size < 256 ? size : 0, off);       // width
    dirEntries.writeUInt8(size < 256 ? size : 0, off + 1);   // height
    dirEntries.writeUInt8(0, off + 2);                         // color count
    dirEntries.writeUInt8(0, off + 3);                         // reserved
    dirEntries.writeUInt16LE(1, off + 4);                      // planes
    dirEntries.writeUInt16LE(32, off + 6);                     // bpp
    dirEntries.writeUInt32LE(buf.length, off + 8);             // data size
    dirEntries.writeUInt32LE(dataOffset, off + 12);            // data offset
    dataOffset += buf.length;
    dataChunks.push(buf);
  }

  return Buffer.concat([header, dirEntries, ...dataChunks]);
}

const icoBuffer = buildIco(pngBuffers);
writeFileSync(resolve(iconsDir, "icon.ico"), icoBuffer);
console.log("Generated: src-tauri/icons/icon.ico");

// 32x32 PNG
await sharp(src).resize(32, 32).png().toFile(resolve(iconsDir, "32x32.png"));
console.log("Generated: src-tauri/icons/32x32.png");

// 128x128 PNG
await sharp(src).resize(128, 128).png().toFile(resolve(iconsDir, "128x128.png"));
console.log("Generated: src-tauri/icons/128x128.png");

console.log("Done!");
