import { createHash } from 'node:crypto';
import { readdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const nameIndex = process.argv.indexOf('--name');
const name = nameIndex < 0 ? 'v1-integrada' : process.argv[nameIndex + 1];
if (!name || !/^[a-z0-9-]+$/.test(name)) throw new Error('Nome de checkpoint inválido.');
const output = `docs/checkpoints/${name}.sha256`;
const excluded = new Set(['node_modules', 'dist', 'target', '.cache', '.git', 'gen']);

async function files(directory) {
  const entries = await readdir(path.join(root, directory), { withFileTypes: true });
  const result = [];
  for (const entry of entries) {
    if (excluded.has(entry.name)) continue;
    const relative = path.posix.join(directory, entry.name);
    if (relative.startsWith('docs/checkpoints/') && relative.endsWith('.sha256')) continue;
    if (relative === output || /\.(db|db-wal|db-shm|log)$/.test(relative)) continue;
    if (entry.isDirectory()) result.push(...await files(relative));
    else if (entry.isFile()) result.push(relative);
  }
  return result;
}

if (process.argv.includes('--verify')) {
  const manifest = (await readFile(path.join(root, output), 'utf8')).trim().split('\n');
  const expected = new Map(manifest.map((line) => [line.slice(66), line.slice(0, 64)]));
  const actual = (await files('')).sort();
  const failures = [];
  for (const relative of actual) {
    const hash = createHash('sha256').update(await readFile(path.join(root, relative))).digest('hex');
    if (expected.get(relative) !== hash) failures.push(relative);
    expected.delete(relative);
  }
  failures.push(...expected.keys());
  if (failures.length) {
    console.error(`Checkpoint divergente: ${failures.join(', ')}`);
    process.exitCode = 1;
  } else {
    console.log(`Checkpoint ${name} íntegro: ${manifest.length} arquivos conferidos.`);
  }
} else {
  const lines = [];
  for (const relative of (await files('')).sort()) {
    const hash = createHash('sha256').update(await readFile(path.join(root, relative))).digest('hex');
    lines.push(`${hash}  ${relative}`);
  }
  await writeFile(path.join(root, output), `${lines.join('\n')}\n`);
  console.log(`Checkpoint ${name} criado com ${lines.length} arquivos.`);
}
