import { createServer } from 'node:http'
import { readFile, readdir, writeFile, mkdtemp, rm } from 'node:fs/promises'
import { spawn } from 'node:child_process'
import { join, extname, relative, resolve } from 'node:path'
import { tmpdir } from 'node:os'
import { fileURLToPath } from 'node:url'
import { DatabaseSync } from 'node:sqlite'

const [vault, output, dataDir = join(process.env.APPDATA ?? '', 'com.dmitriy.aquilum-app')] = process.argv.slice(2)
if (!vault || !output) {
  console.error('нужно: golden.mjs <база знаний> <выходной.json> [каталог данных Таури]')
  process.exit(1)
}

const here = fileURLToPath(new URL('.', import.meta.url))
const foliate = resolve(here, '../../../../tauri-version/vendor/foliate-js')
const BOOKS = new Set(['.epub', '.fb2'])
const TYPES = { '.js': 'text/javascript', '.html': 'text/html', '.json': 'application/json' }

const walk = async dir => (await Promise.all((await readdir(dir, { withFileTypes: true }))
  .filter(entry => !entry.name.startsWith('.'))
  .map(entry => entry.isDirectory() ? walk(join(dir, entry.name)) : [join(dir, entry.name)]))).flat()

const saved = (() => {
  try {
    const db = new DatabaseSync(join(dataDir, 'ui-state.sqlite3'), { readOnly: true })
    return db.prepare('SELECT book_file, cfi FROM reader_states WHERE cfi IS NOT NULL').all()
  } catch {
    return []
  }
})()

const books = (await walk(vault))
  .filter(path => BOOKS.has(extname(path).toLowerCase()))
  .map(path => {
    const file = relative(vault, path).replaceAll('\\', '/')
    return { path, file, saved: saved.filter(row => row.book_file === file).map(row => row.cfi) }
  })

let browser
const server = createServer(async (req, res) => {
  const url = decodeURIComponent(req.url.split('?')[0])
  try {
    if (url === '/list') return res.end(JSON.stringify(books.map(({ file, saved }) => ({ file, saved }))))
    if (url.startsWith('/book/')) return res.end(await readFile(books[Number(url.slice(6))].path))
    if (url === '/page.html') {
      res.setHeader('Content-Type', TYPES['.html'])
      return res.end(await readFile(join(here, 'page.html')))
    }
    if (url.startsWith('/foliate/')) {
      res.setHeader('Content-Type', TYPES[extname(url)] ?? 'application/octet-stream')
      return res.end(await readFile(join(foliate, url.slice(9))))
    }
    if (url === '/log' && req.method === 'POST') {
      const chunks = []
      for await (const chunk of req) chunks.push(chunk)
      console.error(Buffer.concat(chunks).toString())
      return res.end('ok')
    }
    if (url === '/result' && req.method === 'POST') {
      const chunks = []
      for await (const chunk of req) chunks.push(chunk)
      await writeFile(output, Buffer.concat(chunks))
      res.end('ok')
      console.log(`книг: ${books.length}, эталон: ${output}`)
      browser?.kill()
      server.close()
      return
    }
    res.statusCode = 404
    res.end()
  } catch (error) {
    res.statusCode = 500
    res.end(String(error))
  }
})

server.listen(0, '127.0.0.1', async () => {
  const { port } = server.address()
  const profile = await mkdtemp(join(tmpdir(), 'aq-golden-'))
  const edge = ['C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe', 'C:/Program Files/Microsoft/Edge/Application/msedge.exe']
  const exe = process.env.AQ_BROWSER ?? edge[0]
  browser = spawn(exe, ['--headless=new', '--disable-gpu', '--no-first-run', `--user-data-dir=${profile}`, `http://127.0.0.1:${port}/page.html`], { stdio: 'ignore' })
  browser.on('exit', () => rm(profile, { recursive: true, force: true }).catch(() => {}))
})
