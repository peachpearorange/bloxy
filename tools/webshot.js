const { chromium } = require('playwright');
const http = require('http');
const fs = require('fs');
const path = require('path');

const [name, opts] = process.argv.slice(2);
const root = 'target/web';
const types = { '.html': 'text/html', '.js': 'text/javascript', '.wasm': 'application/wasm' };
const server = http.createServer((request, response) => {
  const file = path.join(root, decodeURIComponent(request.url.split('?')[0]).replace(/\/$/, '/index.html'));
  fs.readFile(file, (error, data) => {
    response.writeHead(error ? 404 : 200, { 'Content-Type': types[path.extname(file)] || 'application/octet-stream' });
    response.end(data);
  });
});

server.listen(0, async () => {
  const browser = await chromium.launch({
    args: ['--enable-unsafe-webgpu', '--enable-features=Vulkan', '--use-vulkan=swiftshader', '--disable-vulkan-surface']
  });
  const page = await browser.newPage({ viewport: { width: 1600, height: 900 }, acceptDownloads: true });
  page.on('console', message => {
    const text = message.text();
    if (message.type() === 'error' || /panicked|ERROR|WARN|shot waits/.test(text)) console.log(text.slice(0, 400));
  });
  page.on('pageerror', error => console.log('page error:', error.message));
  const query = `?connect=&opts=${encodeURIComponent((opts || '{shot: 2}').trim().replace(/^\{|\}$/g, ''))}`;
  const download = page.waitForEvent('download', { timeout: 0 });
  await page.goto(`http://localhost:${server.address().port}/${query}`);
  const shot = `screenshots/web-${name}.png`;
  await (await download).saveAs(shot);
  console.log(shot);
  await browser.close();
  server.close();
});
