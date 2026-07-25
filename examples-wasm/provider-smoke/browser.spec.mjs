import { test, expect } from '@playwright/test';
import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { extname, resolve } from 'node:path';

const precision = process.env.BOXDD_WASM_PRECISION === 'double' ? 'double' : 'single';
const providerModule = `box2d-sys-v1-${precision}`;
const artifactRoot = resolve(process.env.CARGO_TARGET_DIR || 'target', 'boxdd-provider-smoke');
const providerScript = `${providerModule}.js`;
const providerWasm = `${providerModule}.wasm`;
const appWasm = 'boxdd_provider_smoke.wasm';

const browserPage = `<!doctype html>
<meta charset="utf-8">
<title>boxdd provider browser smoke</title>
<body data-status="pending">loading</body>
<script type="module" src="/browser-smoke.mjs?precision=${precision}"></script>
`;

async function runBrowserSmoke() {
const params = new URL(import.meta.url).searchParams;
const precision = params.get('precision') === 'double' ? 'double' : 'single';
const providerModule = `box2d-sys-v1-${precision}`;
const artifact = (name) => `/artifacts/${name}`;

function setResult(status, value) {
  document.body.dataset.status = status;
  document.body.textContent = JSON.stringify(value);
}

try {
  const [providerBytes, appBytes] = await Promise.all([
    fetch(artifact(`${providerModule}.wasm`)).then((response) => response.arrayBuffer()),
    fetch(artifact('boxdd_provider_smoke.wasm')).then((response) => response.arrayBuffer()),
  ]);
  const { default: createProvider } = await import(artifact(`${providerModule}.js`));
  const memory = new WebAssembly.Memory({ initial: 2048, maximum: 8192 });
  const provider = await createProvider({
    wasmMemory: memory,
    wasmBinary: providerBytes,
    locateFile: (path) => new URL(artifact(path), location.href).href,
    print: (text) => console.log(`[${providerModule}] ${text}`),
    printErr: (text) => console.warn(`[${providerModule}] ${text}`),
  });
  if (provider.wasmMemory && provider.wasmMemory !== memory) {
    throw new Error('provider did not use the shared WebAssembly.Memory');
  }

  const appModule = await WebAssembly.compile(appBytes);
  const providerImports = WebAssembly.Module.imports(appModule)
    .filter((item) => item.kind === 'function' && item.module === providerModule)
    .map((item) => item.name);
  if (providerImports.length === 0) {
    throw new Error(`application does not import ${providerModule}`);
  }
  const importObject = { env: { memory }, [providerModule]: {} };
  for (const name of providerImports) {
    const exported = provider[`_${name}`] || provider[name];
    if (typeof exported !== 'function') {
      throw new Error(`provider is missing export for ${name}`);
    }
    importObject[providerModule][name] = exported;
  }

  const instance = await WebAssembly.instantiate(appModule, importObject);
  const beforeGrowth = memory.buffer;
  memory.grow(1);
  if (beforeGrowth === memory.buffer) {
    throw new Error('shared WebAssembly.Memory did not grow');
  }
  const postGrowthMetric = instance.exports.boxdd_provider_ray_hit_millimeters();
  if (postGrowthMetric < 0) {
    throw new Error(`provider failed after memory growth: ${postGrowthMetric}`);
  }

  const smoke = instance.exports.boxdd_provider_smoke();
  if (smoke !== 0) {
    throw new Error(`provider smoke failed with code ${smoke}`);
  }
  const metrics = {};
  for (const [label, name] of Object.entries({
    dropMillimeters: 'boxdd_provider_drop_millimeters',
    rayHitMillimeters: 'boxdd_provider_ray_hit_millimeters',
    shapeCastPermyriad: 'boxdd_provider_shape_cast_permyriad',
    jointErrorMillimeters: 'boxdd_provider_joint_error_millimeters',
  })) {
    const exported = instance.exports[name];
    if (typeof exported !== 'function') throw new Error(`${name} export is missing`);
    const value = exported();
    if (value < 0) throw new Error(`${name} failed with code ${value}`);
    metrics[label] = value;
  }
  if (instance.exports.boxdd_runtime_init() !== 0) throw new Error('runtime init failed');
  for (let frame = 0; frame < 30; frame += 1) {
    if (instance.exports.boxdd_runtime_step() < 0) throw new Error('runtime step failed');
  }
  const runtimeBodies = instance.exports.boxdd_runtime_body_count();
  if (runtimeBodies <= 0) throw new Error('runtime body count failed');
  const runtimeState = [];
  for (let index = 0; index < runtimeBodies; index += 1) {
    const body = {
      shape: instance.exports.boxdd_runtime_body_shape(index),
      xMillimeters: instance.exports.boxdd_runtime_body_x_millimeters(index),
      yMillimeters: instance.exports.boxdd_runtime_body_y_millimeters(index),
      angleMilliradians: instance.exports.boxdd_runtime_body_angle_milliradians(index),
      halfWidthMillimeters: instance.exports.boxdd_runtime_body_half_width_millimeters(index),
      halfHeightMillimeters: instance.exports.boxdd_runtime_body_half_height_millimeters(index),
      radiusMillimeters: instance.exports.boxdd_runtime_body_radius_millimeters(index),
    };
    if (body.shape === 1) {
      if (body.halfWidthMillimeters <= 0 || body.halfHeightMillimeters <= 0 || body.radiusMillimeters !== 0) {
        throw new Error(`invalid box geometry at runtime body ${index}: ${JSON.stringify(body)}`);
      }
    } else if (body.shape === 2) {
      if (body.halfWidthMillimeters !== 0 || body.halfHeightMillimeters !== 0 || body.radiusMillimeters <= 0) {
        throw new Error(`invalid circle geometry at runtime body ${index}: ${JSON.stringify(body)}`);
      }
    } else {
      throw new Error(`unknown runtime shape ${body.shape} at body ${index}`);
    }
    runtimeState.push(body);
  }
  setResult('passed', {
    precision,
    providerImports: providerImports.length,
    memoryGrew: true,
    metrics,
    runtimeBodies,
    runtimeState,
  });
} catch (error) {
  setResult('failed', { error: String(error), stack: error?.stack });
  throw error;
}
}

const browserScript = `(${runBrowserSmoke.toString()})();`;

let server;
let origin;

function contentType(pathname) {
  switch (extname(pathname)) {
    case '.html':
      return 'text/html; charset=utf-8';
    case '.mjs':
    case '.js':
      return 'text/javascript; charset=utf-8';
    case '.wasm':
      return 'application/wasm';
    default:
      return 'application/octet-stream';
  }
}

test.beforeAll(async () => {
  for (const file of [providerScript, providerWasm, appWasm]) {
    await readFile(resolve(artifactRoot, file));
  }
  server = createServer(async (request, response) => {
    const pathname = new URL(request.url, 'http://127.0.0.1').pathname;
    let body;
    let contentTypeHeader;
    if (pathname === '/browser-smoke.html') {
      body = browserPage;
      contentTypeHeader = 'text/html; charset=utf-8';
    } else if (pathname === '/browser-smoke.mjs') {
      body = browserScript;
      contentTypeHeader = 'text/javascript; charset=utf-8';
    } else if (pathname.startsWith('/artifacts/')) {
      const file = pathname.slice('/artifacts/'.length);
      if (![providerScript, providerWasm, appWasm].includes(file)) {
        response.writeHead(404).end();
        return;
      }
      body = await readFile(resolve(artifactRoot, file));
      contentTypeHeader = contentType(pathname);
    } else {
      response.writeHead(404).end();
      return;
    }
    response.writeHead(200, {
      'Content-Type': contentTypeHeader,
      'Cache-Control': 'no-store',
      'Access-Control-Allow-Origin': '*',
    });
    response.end(body);
  });
  await new Promise((resolveServer) => server.listen(0, '127.0.0.1', resolveServer));
  const address = server.address();
  origin = `http://127.0.0.1:${address.port}`;
});

test.afterAll(async () => {
  await new Promise((resolveServer, reject) => server.close((error) => (error ? reject(error) : resolveServer())));
});

test(`browser provider smoke (${precision})`, async ({ page }) => {
  const pageErrors = [];
  page.on('pageerror', (error) => pageErrors.push(String(error)));
  await page.goto(`${origin}/browser-smoke.html?precision=${precision}`);
  await expect(page.locator('body')).toHaveAttribute('data-status', 'passed');
  const result = JSON.parse(await page.locator('body').textContent());
  expect(result.precision).toBe(precision);
  expect(result.providerImports).toBeGreaterThan(0);
  expect(result.memoryGrew).toBe(true);
  expect(result.runtimeBodies).toBeGreaterThan(0);
  expect(result.runtimeState).toHaveLength(result.runtimeBodies);
  expect(result.runtimeState.every((body) => Number.isInteger(body.xMillimeters))).toBe(true);
  expect(result.runtimeState.every((body) => Number.isInteger(body.yMillimeters))).toBe(true);
  expect(result.runtimeState.every((body) => Number.isInteger(body.angleMilliradians))).toBe(true);
  expect(pageErrors).toEqual([]);
});
