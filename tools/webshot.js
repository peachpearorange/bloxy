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
    channel: 'chromium',
    args: ['--enable-unsafe-webgpu', '--enable-features=Vulkan', '--use-vulkan=swiftshader', '--use-angle=vulkan', '--disable-vulkan-surface', '--disable-gpu-watchdog', '--disable-gpu-process-crash-limit']
  });
  const page = await browser.newPage({ viewport: { width: 1600, height: 900 }, acceptDownloads: true });
  page.on('console', message => {
    const text = message.text().replace(/%c/g, '').replace(/ color: .*$/s, '');
    if (/Quitting the application|panicked at/.test(text)) setTimeout(() => process.exit(1), 2000);
    if (process.env.VERBOSE || message.type() === 'error' || message.type() === 'warning' || /panicked|ERROR|WARN/.test(text)) console.log(text.slice(0, 2000));
  });
  await page.addInitScript(() => {
    const create = GPUDevice.prototype.createShaderModule;
    GPUDevice.prototype.createShaderModule = function (descriptor) {
      const module = create.call(this, descriptor);
      module.getCompilationInfo().then(info => {
        for (const message of info.messages.filter(message => message.type === 'error')) {
          const line = descriptor.code.split('\n')[message.lineNum - 1] || '';
          console.error(`WGSL error ${message.lineNum}:${message.linePos} ${message.message}\n  > ${line.trim()}\n  in shader starting: ${descriptor.code.slice(0, 120).replace(/\n/g, ' ')}`);
        }
      });
      return module;
    };
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
