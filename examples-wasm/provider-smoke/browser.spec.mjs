import { test, expect } from '@playwright/test';
import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { dirname, extname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const precision = process.env.BOXDD_WASM_PRECISION === 'double' ? 'double' : 'single';
const providerModule = `box2d-sys-v1-${precision}`;
const artifactRoot = resolve(process.env.CARGO_TARGET_DIR || 'target', 'boxdd-provider-smoke');
const providerScript = `${providerModule}.js`;
const providerWasm = `${providerModule}.wasm`;
const appWasm = 'boxdd_provider_smoke.wasm';
const providerRuntimeContract = 'provider-runtime-contract.mjs';
const fixtureRoot = dirname(fileURLToPath(import.meta.url));

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
  const providerContract = inspectProviderContract(appModule, providerModule);
  const providerFunctions = resolveProviderFunctions(provider, providerContract.names);
  const result = await runProviderPhysicsScenario({
    appModule,
    memory,
    provider,
    contract: providerContract,
    functions: providerFunctions,
  });
  setResult('passed', {
    precision,
    ...result,
  });
} catch (error) {
  setResult('failed', { error: String(error), stack: error?.stack });
  throw error;
}
}

const browserScript = `
import {
  inspectProviderContract,
  resolveProviderFunctions,
  runProviderPhysicsScenario,
} from '/provider-runtime-contract.mjs';

(${runBrowserSmoke.toString()})();
`;

let server;
let origin;
let providerRuntimeContractSource;

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
  providerRuntimeContractSource = await readFile(resolve(fixtureRoot, providerRuntimeContract));
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
    } else if (pathname === `/${providerRuntimeContract}`) {
      body = providerRuntimeContractSource;
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
  expect(result.memoryProof).toEqual({
    memoryGrew: true,
    staleTypedArrayRejected: true,
    staleDataViewRejected: true,
    refreshedViewsReadWrite: true,
    providerHeapViewRefreshed: true,
    providerHeapReadWrite: true,
    providerGlueCallsAfterGrowth: expect.any(Number),
  });
  expect(result.memoryProof.providerGlueCallsAfterGrowth).toBeGreaterThan(0);
  expect(result.linkFailures).toEqual({
    oldProviderAbi: true,
    wrongPrecision: true,
    wrongFunctionType: true,
    providerCallsBeforePhysics: 0,
  });
  expect(result.runtimeBodies).toBeGreaterThan(0);
  expect(result.runtimeState).toHaveLength(result.runtimeBodies);
  expect(result.runtimeState.every((body) => Number.isInteger(body.xMillimeters))).toBe(true);
  expect(result.runtimeState.every((body) => Number.isInteger(body.yMillimeters))).toBe(true);
  expect(result.runtimeState.every((body) => Number.isInteger(body.angleMilliradians))).toBe(true);
  expect(pageErrors).toEqual([]);
});
