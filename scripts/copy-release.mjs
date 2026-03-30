import { copyFileSync, mkdirSync } from "fs";
import { resolve, dirname } from "path";
import { fileURLToPath } from "url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = resolve(__dirname, "..");
const src = resolve(root, "src-tauri/target/release/FileClassifier.exe");
const destDir = resolve(root, "release");
const dest = resolve(destDir, "FileClassifier.exe");

mkdirSync(destDir, { recursive: true });
copyFileSync(src, dest);
console.log(`Copied: ${dest}`);
