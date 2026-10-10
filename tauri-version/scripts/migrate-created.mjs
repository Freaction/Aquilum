import { readdir, readFile, rename, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { migrateNote } from './lib/migrate-created.mjs';

const SKIPPED_FOLDERS = new Set(['.git', '.obsidian', '.aquilum', 'node_modules', '.trash', 'Templates', 'Шаблоны']);
const SHOWN_EXAMPLES = 12;

async function markdownFiles(root, folder = root) {
  const entries = await readdir(folder, { withFileTypes: true });
  const found = [];
  for (const entry of entries) {
    const path = join(folder, entry.name);
    if (entry.isDirectory()) {
      if (SKIPPED_FOLDERS.has(entry.name) || entry.name.startsWith('.')) continue;
      found.push(...(await markdownFiles(root, path)));
      continue;
    }
    if (entry.name.toLowerCase().endsWith('.md')) found.push(path);
  }
  return found;
}

async function writeAtomically(path, text) {
  const staging = `${path}.migrating`;
  await writeFile(staging, text, 'utf8');
  await rename(staging, path);
}

async function main() {
  const [root, ...flags] = process.argv.slice(2);
  if (!root) {
    console.error('Использование: node scripts/migrate-created.mjs <путь к базе знаний> [--apply]');
    process.exit(2);
  }
  const apply = flags.includes('--apply');
  const files = await markdownFiles(root);
  const counts = { created: 0, merged: 0, declared: 0, 'no-date': 0, ambiguous: 0, failed: 0 };
  const examples = [];

  for (const path of files) {
    let text;
    try {
      text = await readFile(path, 'utf8');
    } catch (cause) {
      counts.failed += 1;
      console.error(`не прочиталось: ${relative(root, path)} — ${cause.message}`);
      continue;
    }
    const result = migrateNote(text);
    counts[result.reason] += 1;
    if (!result.changed) continue;
    if (examples.length < SHOWN_EXAMPLES) {
      examples.push(`${relative(root, path)} → created: ${result.value}`);
    }
    if (!apply) continue;
    try {
      await writeAtomically(path, result.text);
    } catch (cause) {
      counts.failed += 1;
      console.error(`не записалось: ${relative(root, path)} — ${cause.message}`);
    }
  }

  console.log(`заметок просмотрено: ${files.length}`);
  console.log(`frontmatter создан: ${counts.created}`);
  console.log(`дата добавлена в существующий frontmatter: ${counts.merged}`);
  console.log(`уже объявляют дату, пропущены: ${counts.declared}`);
  console.log(`даты в шапке нет, пропущены: ${counts['no-date']}`);
  if (counts.ambiguous > 0) {
    console.log(`дата читается двояко, пропущены: ${counts.ambiguous}`);
  }
  if (counts.failed > 0) console.log(`ошибок: ${counts.failed}`);
  if (examples.length > 0) {
    console.log('\nпримеры:');
    for (const example of examples) console.log(`  ${example}`);
  }
  if (!apply) {
    console.log('\nэто сухой прогон, файлы не изменены. Для записи добавьте --apply');
  }
}

main().catch((cause) => {
  console.error(cause);
  process.exit(1);
});
