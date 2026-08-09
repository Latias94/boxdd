import { expect, test } from '@playwright/test';
import { constants } from 'node:fs';
import { createServer } from 'node:http';
import { lstat, open, realpath } from 'node:fs/promises';
import { dirname, extname, isAbsolute, join, relative, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { gzipSync } from 'node:zlib';

const fixtureRoot = dirname(fileURLToPath(import.meta.url));
const repositoryRoot = resolve(fixtureRoot, '../..');
const pagesRoot = resolve(repositoryRoot, 'docs/pages');
const runtimeManifestPath = '/wasm/generated/boxdd-pages-runtime-v2.json';
const runtimeBuildCommand = 'cargo run --locked -p xtask -- build-pages-wasm';

let canonicalPagesRoot;
let origin;
let server;
let expectedRuntimeFetchPaths;
let expectedRuntimeAssetHashes;
let overflowResponse;
let overflowAssetResponse;
let tamperedAssetResponse;
let stalledResponse;
let slowResponse;
let gzipManifestResponse;
let loaderInactivityTimeoutOverrideMs;
let loaderTotalTimeoutOverrideMs;

class HttpError extends Error {
  constructor(status, message, options) {
    super(message, options);
    this.status = status;
  }
}

function contentType(pathname) {
  switch (extname(pathname).toLowerCase()) {
    case '.css':
      return 'text/css; charset=utf-8';
    case '.html':
      return 'text/html; charset=utf-8';
    case '.js':
    case '.mjs':
      return 'text/javascript; charset=utf-8';
    case '.json':
      return 'application/json; charset=utf-8';
    case '.wasm':
      return 'application/wasm';
    case '.png':
      return 'image/png';
    case '.svg':
      return 'image/svg+xml';
    case '.woff':
      return 'font/woff';
    case '.woff2':
      return 'font/woff2';
    default:
      return 'application/octet-stream';
  }
}

function isWithinRoot(path) {
  const pathFromRoot = relative(canonicalPagesRoot, path);
  return pathFromRoot !== '..'
    && !pathFromRoot.startsWith(`..${sep}`)
    && !isAbsolute(pathFromRoot);
}

function requestSegments(pathname) {
  let decoded;
  try {
    decoded = decodeURIComponent(pathname);
  } catch (error) {
    throw new HttpError(400, 'request path is not valid UTF-8 percent encoding', { cause: error });
  }

  if (!decoded.startsWith('/') || decoded.includes('\0') || decoded.includes('\\')) {
    throw new HttpError(400, 'request path is malformed');
  }

  const segments = decoded.split('/').slice(1);
  const servesDirectoryIndex = segments.at(-1) === '';
  if (servesDirectoryIndex) {
    segments.pop();
  }
  if (segments.some((segment) => segment === '' || segment === '.' || segment === '..')) {
    throw new HttpError(403, 'request path traversal is forbidden');
  }
  if (servesDirectoryIndex) {
    segments.push('index.html');
  }
  return segments;
}

async function readOrdinaryPagesFile(pathname) {
  const segments = requestSegments(pathname);
  if (segments.length === 0) {
    throw new HttpError(404, 'not found');
  }

  let current = pagesRoot;
  let expectedFileMetadata;
  for (const [index, segment] of segments.entries()) {
    current = join(current, segment);
    let metadata;
    try {
      metadata = await lstat(current);
    } catch (error) {
      if (error?.code === 'ENOENT' || error?.code === 'ENOTDIR') {
        throw new HttpError(404, 'not found', { cause: error });
      }
      throw error;
    }
    if (metadata.isSymbolicLink()) {
      throw new HttpError(403, 'symbolic links are forbidden');
    }
    const isFinal = index === segments.length - 1;
    if ((!isFinal && !metadata.isDirectory()) || (isFinal && !metadata.isFile())) {
      throw new HttpError(404, 'not found');
    }
    if (isFinal) {
      expectedFileMetadata = metadata;
    }
  }

  const canonicalFile = await realpath(current);
  if (!isWithinRoot(canonicalFile)) {
    throw new HttpError(403, 'request escaped the Pages root');
  }

  let file;
  try {
    file = await open(current, constants.O_RDONLY | constants.O_NOFOLLOW);
    const metadata = await file.stat();
    if (
      !metadata.isFile()
      || metadata.dev !== expectedFileMetadata.dev
      || metadata.ino !== expectedFileMetadata.ino
    ) {
      throw new HttpError(403, 'served file changed during security validation');
    }
    return await file.readFile();
  } catch (error) {
    if (error?.code === 'ELOOP') {
      throw new HttpError(403, 'symbolic links are forbidden', { cause: error });
    }
    throw error;
  } finally {
    await file?.close();
  }
}

async function requireGeneratedRuntime() {
  let manifestBytes;
  try {
    manifestBytes = await readOrdinaryPagesFile(runtimeManifestPath);
  } catch (error) {
    throw new Error(
      `generated Pages runtime is missing or unsafe; run \`${runtimeBuildCommand}\` before this test`,
      { cause: error },
    );
  }

  let manifest;
  try {
    manifest = JSON.parse(manifestBytes.toString('utf8'));
  } catch (error) {
    throw new Error('generated Pages runtime manifest is not valid JSON', { cause: error });
  }
  if (!Array.isArray(manifest.assets) || manifest.assets.length === 0) {
    throw new Error('generated Pages runtime manifest does not list any assets');
  }
  expectedRuntimeFetchPaths = new Set([runtimeManifestPath]);
  expectedRuntimeAssetHashes = new Map();
  for (const asset of manifest.assets) {
    if (
      !asset
      || typeof asset.path !== 'string'
      || asset.path.startsWith('/')
      || asset.path.split('/').some((segment) => segment === '' || segment === '.' || segment === '..')
    ) {
      throw new Error('generated Pages runtime manifest contains an invalid asset path');
    }
    try {
      await readOrdinaryPagesFile(`/${asset.path}`);
    } catch (error) {
      throw new Error(`generated Pages runtime asset is missing or unsafe: ${asset.path}`, { cause: error });
    }
    expectedRuntimeFetchPaths.add(`/${asset.path}`);
    expectedRuntimeAssetHashes.set(`/${asset.path}`, asset.sha256);
  }

  const loader = await readOrdinaryPagesFile('/bevy-testbed/loader.js');
  if (loader.includes('const runtimeContract = null;')) {
    throw new Error(
      `Pages loader has no runtime contract; run \`${runtimeBuildCommand}\` before this test`,
    );
  }
}

function responseCacheControl(pathname) {
  if (pathname === runtimeManifestPath) {
    return 'no-store';
  }
  if (pathname.endsWith('.wasm')) {
    return 'max-age=600';
  }
  if (pathname.endsWith('.js')) {
    return 'max-age=14400';
  }
  return 'no-store';
}

async function streamOverflowFixture(response, fixture, type) {
  response.on('close', () => {
    fixture.closed = true;
  });
  response.writeHead(200, {
    'Cache-Control': 'no-store',
    'Content-Type': type,
    'X-Content-Type-Options': 'nosniff',
  });
  const chunk = Buffer.alloc(16 * 1024, 0x20);
  while (fixture.bytesSent < fixture.totalBytes && !response.destroyed) {
    response.write(chunk);
    fixture.bytesSent += chunk.byteLength;
    await new Promise((resolveDelay) => setTimeout(resolveDelay, 2));
  }
  if (!response.destroyed) {
    fixture.completed = true;
    response.end();
  }
}

function streamStalledFixture(response, fixture, type) {
  fixture.response = response;
  response.on('close', () => {
    fixture.closed = true;
  });
  response.writeHead(200, {
    'Cache-Control': 'no-store',
    'Content-Type': type,
    'X-Content-Type-Options': 'nosniff',
  });
  const prefix = Buffer.from('{');
  response.write(prefix);
  fixture.bytesSent += prefix.byteLength;
}

async function streamSlowFixture(response, fixture, type) {
  fixture.response = response;
  response.on('close', () => {
    fixture.closed = true;
  });
  response.writeHead(200, {
    'Cache-Control': 'no-store',
    'Content-Type': type,
    'X-Content-Type-Options': 'nosniff',
  });
  while (!response.destroyed) {
    response.write(fixture.bytesSent === 0 ? Buffer.from('{') : Buffer.from(' '));
    fixture.bytesSent += 1;
    await new Promise((resolveDelay) => setTimeout(resolveDelay, fixture.intervalMs));
  }
}

async function startPagesServer() {
  server = createServer(async (request, response) => {
    try {
      if (request.method !== 'GET' && request.method !== 'HEAD') {
        response.writeHead(405, { Allow: 'GET, HEAD' }).end();
        return;
      }
      const pathname = new URL(request.url ?? '/', 'http://127.0.0.1').pathname;
      if (request.method === 'GET' && pathname === runtimeManifestPath && slowResponse) {
        await streamSlowFixture(response, slowResponse, 'application/json; charset=utf-8');
        return;
      }
      if (request.method === 'GET' && pathname === runtimeManifestPath && stalledResponse) {
        streamStalledFixture(response, stalledResponse, 'application/json; charset=utf-8');
        return;
      }
      if (request.method === 'GET' && pathname === runtimeManifestPath && overflowResponse) {
        await streamOverflowFixture(response, overflowResponse, 'application/json; charset=utf-8');
        return;
      }
      if (
        request.method === 'GET'
        && overflowAssetResponse
        && pathname === overflowAssetResponse.pathname
      ) {
        await streamOverflowFixture(response, overflowAssetResponse, contentType(pathname));
        return;
      }
      let body = await readOrdinaryPagesFile(pathname);
      if (
        request.method === 'GET'
        && tamperedAssetResponse
        && pathname === tamperedAssetResponse.pathname
      ) {
        if (body.byteLength === 0) {
          throw new Error('cannot tamper with an empty runtime asset');
        }
        body = Buffer.from(body);
        body[0] ^= 0x01;
        tamperedAssetResponse.served = true;
      }
      if (pathname === '/bevy-testbed/loader.js' && loaderInactivityTimeoutOverrideMs) {
        const source = body.toString('utf8');
        const replacement = `const RUNTIME_FETCH_INACTIVITY_TIMEOUT_MS = ${loaderInactivityTimeoutOverrideMs};`;
        const overridden = source.replace(
          'const RUNTIME_FETCH_INACTIVITY_TIMEOUT_MS = 30 * 1000;',
          replacement,
        );
        if (overridden === source) {
          throw new Error('Pages loader inactivity timeout declaration was not found');
        }
        body = Buffer.from(overridden);
      }
      if (pathname === '/bevy-testbed/loader.js' && loaderTotalTimeoutOverrideMs) {
        const source = body.toString('utf8');
        const replacement = `const RUNTIME_FETCH_TOTAL_TIMEOUT_MS = ${loaderTotalTimeoutOverrideMs};`;
        const overridden = source.replace(
          'const RUNTIME_FETCH_TOTAL_TIMEOUT_MS = 5 * 60 * 1000;',
          replacement,
        );
        if (overridden === source) {
          throw new Error('Pages loader total timeout declaration was not found');
        }
        body = Buffer.from(overridden);
      }
      let contentEncoding;
      if (request.method === 'GET' && pathname === runtimeManifestPath && gzipManifestResponse) {
        gzipManifestResponse.decodedByteLength = body.byteLength;
        body = gzipSync(body);
        gzipManifestResponse.compressedByteLength = body.byteLength;
        gzipManifestResponse.served = true;
        contentEncoding = 'gzip';
      }
      const responsePath = pathname.endsWith('/') ? `${pathname}index.html` : pathname;
      response.writeHead(200, {
        'Cache-Control': responseCacheControl(responsePath),
        ...(contentEncoding ? { 'Content-Encoding': contentEncoding } : {}),
        'Content-Length': body.byteLength,
        'Content-Type': contentType(responsePath),
        'X-Content-Type-Options': 'nosniff',
      });
      response.end(request.method === 'HEAD' ? undefined : body);
    } catch (error) {
      const status = error instanceof HttpError ? error.status : 500;
      response.writeHead(status, {
        'Cache-Control': 'no-store',
        'Content-Type': 'text/plain; charset=utf-8',
        'X-Content-Type-Options': 'nosniff',
      });
      response.end(status === 500 ? 'internal server error' : error.message);
    }
  });
  await new Promise((resolveServer, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', resolveServer);
  });
  const address = server.address();
  origin = `http://127.0.0.1:${address.port}`;
}

async function runtimeEvidence(page) {
  return page.evaluate(() => {
    if (typeof window.BOXDD_BEVY_RUNTIME_EVIDENCE !== 'function') {
      throw new Error('BOXDD_BEVY_RUNTIME_EVIDENCE must be a function returning a live snapshot');
    }
    return window.BOXDD_BEVY_RUNTIME_EVIDENCE();
  });
}

test.beforeAll(async () => {
  const rootMetadata = await lstat(pagesRoot);
  if (rootMetadata.isSymbolicLink() || !rootMetadata.isDirectory()) {
    throw new Error('docs/pages must be an ordinary directory');
  }
  canonicalPagesRoot = await realpath(pagesRoot);
  await requireGeneratedRuntime();
  await startPagesServer();
});

test.afterAll(async () => {
  if (server) {
    await new Promise((resolveServer, reject) => {
      server.close((error) => (error ? reject(error) : resolveServer()));
    });
  }
});

test('published Bevy runtime survives Rust-owned memory growth and keeps stepping physics', async ({ page }) => {
  const consoleErrors = [];
  const pageErrors = [];
  const requestFailures = [];
  const verifiedRuntimeFetches = new Set();
  const toleratedRuntimeAborts = [];
  const unexpectedHttp = [];

  page.on('console', (message) => {
    if (message.type() === 'error') {
      consoleErrors.push(message.text());
    }
  });
  page.on('pageerror', (error) => pageErrors.push(String(error)));
  page.on('request', (request) => {
    const url = new URL(request.url());
    if ((url.protocol === 'http:' || url.protocol === 'https:') && url.origin !== origin) {
      unexpectedHttp.push(`external request: ${request.method()} ${request.url()}`);
    }
  });
  page.on('requestfailed', (request) => {
    const url = new URL(request.url());
    const errorText = request.failure()?.errorText ?? 'unknown error';
    if (
      url.origin === origin
      && expectedRuntimeFetchPaths.has(url.pathname)
      && request.resourceType() === 'fetch'
      && errorText === 'net::ERR_ABORTED'
    ) {
      // Chromium can report cancellation after the loader has consumed verified bytes into a Blob.
      toleratedRuntimeAborts.push(`${request.method()} ${url.pathname}: ${errorText}`);
      return;
    }
    requestFailures.push(`${request.method()} ${request.url()}: ${errorText}`);
  });
  page.on('response', (response) => {
    const url = new URL(response.url());
    if (
      url.origin === origin
      && expectedRuntimeFetchPaths.has(url.pathname)
      && response.status() >= 200
      && response.status() < 300
    ) {
      verifiedRuntimeFetches.add(url.pathname);
      const expectedSha256 = expectedRuntimeAssetHashes.get(url.pathname);
      if (expectedSha256 && url.searchParams.get('sha256') !== expectedSha256) {
        unexpectedHttp.push(`runtime asset lacks its SHA-256 cache key: ${response.url()}`);
      }
    }
    if (response.status() < 200 || response.status() >= 300) {
      unexpectedHttp.push(`HTTP ${response.status()}: ${response.url()}`);
    }
  });

  await page.goto(`${origin}/bevy-testbed/?boxdd-runtime-proof=1`, { waitUntil: 'domcontentloaded' });
  const readRuntimeStatus = () => page.evaluate(() => ({
    ready: window.BOXDD_BEVY_TESTBED_READY === true,
    state: document.querySelector('#bevy-status')?.dataset.state ?? null,
    status: document.querySelector('#bevy-status')?.textContent?.trim() ?? null,
  }));
  try {
    await expect.poll(
      readRuntimeStatus,
      { timeout: 45_000, message: 'the verified Bevy runtime did not reach its running state' },
    ).toMatchObject({ ready: true, state: 'running' });
  } catch (error) {
    throw new Error(
      `the verified Bevy runtime did not reach its running state: ${JSON.stringify(await readRuntimeStatus())}; page errors: ${JSON.stringify(pageErrors)}; console errors: ${JSON.stringify(consoleErrors)}`,
      { cause: error },
    );
  }

  const canvas = page.locator('#bevy-canvas');
  await expect(canvas).toBeVisible();
  const canvasDimensions = await canvas.evaluate((element) => ({
    backingHeight: element.height,
    backingWidth: element.width,
    clientHeight: element.clientHeight,
    clientWidth: element.clientWidth,
  }));
  expect(canvasDimensions.backingWidth).toBeGreaterThan(0);
  expect(canvasDimensions.backingHeight).toBeGreaterThan(0);
  expect(canvasDimensions.clientWidth).toBeGreaterThan(0);
  expect(canvasDimensions.clientHeight).toBeGreaterThan(0);

  const evidence = await runtimeEvidence(page);
  expect(Object.keys(evidence).sort()).toEqual([
    'memoryProof',
    'providerCalls',
    'stepCalls',
  ]);
  expect(evidence.providerCalls).toBeGreaterThan(0);
  expect(evidence.stepCalls).toBeGreaterThan(0);
  expect(Object.keys(evidence.memoryProof).sort()).toEqual([
    'byteLengthAfterGrowth',
    'byteLengthBeforeGrowth',
    'externalGrowth',
    'growthObservedDuringApp',
    'memoryGrew',
    'postGrowthPhysicsStep',
    'providerHeapReadWrite',
    'providerHeapViewRefreshed',
    'requested',
    'staleBufferDetached',
    'stepCallsAfterGrowth',
    'stepCallsBeforeGrowth',
  ]);
  expect(evidence.memoryProof).toMatchObject({
    memoryGrew: true,
    postGrowthPhysicsStep: true,
    providerHeapReadWrite: true,
    providerHeapViewRefreshed: true,
    externalGrowth: true,
    requested: true,
    staleBufferDetached: true,
  });
  expect(typeof evidence.memoryProof.growthObservedDuringApp).toBe('boolean');
  expect(evidence.memoryProof.byteLengthBeforeGrowth).toBeGreaterThan(0);
  expect(evidence.memoryProof.byteLengthAfterGrowth)
    .toBeGreaterThan(evidence.memoryProof.byteLengthBeforeGrowth);
  expect(evidence.memoryProof.stepCallsAfterGrowth)
    .toBeGreaterThan(evidence.memoryProof.stepCallsBeforeGrowth);
  expect(evidence.stepCalls).toBeGreaterThanOrEqual(evidence.memoryProof.stepCallsAfterGrowth);

  const stepCallsAtReady = evidence.stepCalls;
  await expect.poll(
    async () => (await runtimeEvidence(page)).stepCalls,
    { timeout: 10_000, message: 'physics stopped stepping after the post-growth proof' },
  ).toBeGreaterThan(stepCallsAtReady);

  expect(pageErrors).toEqual([]);
  expect(consoleErrors).toEqual([]);
  const missingRuntimeFetches = [...expectedRuntimeFetchPaths]
    .filter((path) => !verifiedRuntimeFetches.has(path));
  expect(
    missingRuntimeFetches,
    `expected every manifest asset to return 2xx; tolerated aborts: ${JSON.stringify(toleratedRuntimeAborts)}`,
  ).toEqual([]);
  expect(requestFailures).toEqual([]);
  expect(unexpectedHttp).toEqual([]);
});

test('loader accepts an automatically decoded gzip runtime manifest', async ({ page }) => {
  const fixture = {
    compressedByteLength: 0,
    decodedByteLength: 0,
    served: false,
  };
  gzipManifestResponse = fixture;

  try {
    await page.goto(`${origin}/bevy-testbed/?boxdd-runtime-proof=1`, {
      waitUntil: 'domcontentloaded',
    });
    await expect.poll(
      () => page.evaluate(() => ({
        ready: window.BOXDD_BEVY_TESTBED_READY === true,
        state: document.querySelector('#bevy-status')?.dataset.state ?? null,
      })),
      { timeout: 45_000, message: 'the gzip-backed runtime did not reach its running state' },
    ).toMatchObject({ ready: true, state: 'running' });

    expect(fixture.served).toBe(true);
    expect(fixture.compressedByteLength).toBeGreaterThan(0);
    expect(fixture.decodedByteLength).toBeGreaterThan(0);
    expect(fixture.compressedByteLength).not.toBe(fixture.decodedByteLength);
  } finally {
    gzipManifestResponse = undefined;
  }
});

test('loader cancels an oversized streaming manifest before buffering it completely', async ({ page }) => {
  const fixture = {
    bytesSent: 0,
    closed: false,
    completed: false,
    totalBytes: 2 * 1024 * 1024,
  };
  overflowResponse = fixture;

  try {
    await page.goto(`${origin}/bevy-testbed/`, { waitUntil: 'domcontentloaded' });
    await expect.poll(
      () => page.evaluate(() => ({
        state: document.querySelector('#bevy-status')?.dataset.state ?? null,
        status: document.querySelector('#bevy-status')?.textContent?.trim() ?? null,
      })),
      { timeout: 10_000, message: 'the loader did not reject the oversized manifest stream' },
    ).toMatchObject({ state: 'error' });

    const status = await page.locator('#bevy-status').textContent();
    expect(status).toContain('runtime manifest exceeds its 1048576-byte limit');
    await expect.poll(
      () => fixture.closed,
      { timeout: 5_000, message: 'the browser did not cancel the oversized response' },
    ).toBe(true);
    expect(fixture.completed).toBe(false);
    expect(fixture.bytesSent).toBeLessThan(fixture.totalBytes);
  } finally {
    overflowResponse = undefined;
  }
});

test('loader aborts a runtime manifest stream that stops making progress', async ({ page }) => {
  const fixture = {
    bytesSent: 0,
    closed: false,
    response: undefined,
  };
  stalledResponse = fixture;
  loaderInactivityTimeoutOverrideMs = 100;

  try {
    await page.goto(`${origin}/bevy-testbed/`, { waitUntil: 'domcontentloaded' });
    await expect.poll(
      () => page.evaluate(() => ({
        state: document.querySelector('#bevy-status')?.dataset.state ?? null,
        status: document.querySelector('#bevy-status')?.textContent?.trim() ?? null,
      })),
      { timeout: 5_000, message: 'the loader did not abort the stalled manifest stream' },
    ).toMatchObject({ state: 'error' });

    const status = await page.locator('#bevy-status').textContent();
    expect(status).toContain('runtime manifest download stalled for 100ms');
    await expect.poll(
      () => fixture.closed,
      { timeout: 5_000, message: 'the browser did not close the stalled response' },
    ).toBe(true);
    expect(fixture.bytesSent).toBe(1);
  } finally {
    stalledResponse = undefined;
    loaderInactivityTimeoutOverrideMs = undefined;
    fixture.response?.destroy();
  }
});

test('loader aborts a runtime manifest that makes progress beyond the total deadline', async ({ page }) => {
  const fixture = {
    bytesSent: 0,
    closed: false,
    intervalMs: 20,
    response: undefined,
  };
  slowResponse = fixture;
  loaderInactivityTimeoutOverrideMs = 100;
  loaderTotalTimeoutOverrideMs = 150;

  try {
    await page.goto(`${origin}/bevy-testbed/`, { waitUntil: 'domcontentloaded' });
    await expect.poll(
      () => page.evaluate(() => ({
        state: document.querySelector('#bevy-status')?.dataset.state ?? null,
        status: document.querySelector('#bevy-status')?.textContent?.trim() ?? null,
      })),
      { timeout: 5_000, message: 'the loader did not enforce the total manifest deadline' },
    ).toMatchObject({ state: 'error' });

    const status = await page.locator('#bevy-status').textContent();
    expect(status).toContain('runtime manifest download exceeded 150ms');
    await expect.poll(
      () => fixture.closed,
      { timeout: 5_000, message: 'the browser did not close the slow response' },
    ).toBe(true);
    expect(fixture.bytesSent).toBeGreaterThan(1);
  } finally {
    slowResponse = undefined;
    loaderInactivityTimeoutOverrideMs = undefined;
    loaderTotalTimeoutOverrideMs = undefined;
    fixture.response?.destroy();
  }
});

test('loader cancels an asset that exceeds its manifest length before instantiation', async ({ page }) => {
  const manifest = JSON.parse((await readOrdinaryPagesFile(runtimeManifestPath)).toString('utf8'));
  const asset = manifest.assets[0];
  const fixture = {
    bytesSent: 0,
    closed: false,
    completed: false,
    pathname: `/${asset.path}`,
    totalBytes: asset.byte_length + 2 * 1024 * 1024,
  };
  overflowAssetResponse = fixture;
  await page.addInitScript(() => {
    globalThis.BOXDD_TEST_WASM_INSTANTIATIONS = 0;
    for (const name of ['instantiate', 'instantiateStreaming']) {
      const original = WebAssembly[name];
      WebAssembly[name] = (...args) => {
        globalThis.BOXDD_TEST_WASM_INSTANTIATIONS += 1;
        return original(...args);
      };
    }
  });

  try {
    await page.goto(`${origin}/bevy-testbed/`, { waitUntil: 'domcontentloaded' });
    await expect.poll(
      () => page.evaluate(() => ({
        state: document.querySelector('#bevy-status')?.dataset.state ?? null,
        status: document.querySelector('#bevy-status')?.textContent?.trim() ?? null,
      })),
      { timeout: 10_000, message: 'the loader did not reject the oversized asset stream' },
    ).toMatchObject({ state: 'error' });

    const status = await page.locator('#bevy-status').textContent();
    expect(status).toContain(`${asset.role} exceeds its ${asset.byte_length}-byte limit`);
    await expect.poll(
      () => fixture.closed,
      { timeout: 5_000, message: 'the browser did not cancel the oversized asset response' },
    ).toBe(true);
    expect(fixture.completed).toBe(false);
    expect(fixture.bytesSent).toBeLessThan(fixture.totalBytes);
    expect(await page.evaluate(() => globalThis.BOXDD_TEST_WASM_INSTANTIATIONS)).toBe(0);
  } finally {
    overflowAssetResponse = undefined;
  }
});

test('loader rejects a same-length tampered asset before importing runtime modules', async ({ page }) => {
  const manifest = JSON.parse((await readOrdinaryPagesFile(runtimeManifestPath)).toString('utf8'));
  const asset = manifest.assets[0];
  const fixture = {
    pathname: `/${asset.path}`,
    served: false,
  };
  tamperedAssetResponse = fixture;
  await page.addInitScript(() => {
    globalThis.BOXDD_TEST_RUNTIME_OBJECT_URLS = 0;
    globalThis.BOXDD_TEST_WASM_INSTANTIATIONS = 0;

    const createObjectURL = URL.createObjectURL.bind(URL);
    URL.createObjectURL = (...args) => {
      globalThis.BOXDD_TEST_RUNTIME_OBJECT_URLS += 1;
      return createObjectURL(...args);
    };
    for (const name of ['instantiate', 'instantiateStreaming']) {
      const original = WebAssembly[name];
      WebAssembly[name] = (...args) => {
        globalThis.BOXDD_TEST_WASM_INSTANTIATIONS += 1;
        return original(...args);
      };
    }
  });

  try {
    await page.goto(`${origin}/bevy-testbed/`, { waitUntil: 'domcontentloaded' });
    await expect.poll(
      () => page.evaluate(() => ({
        state: document.querySelector('#bevy-status')?.dataset.state ?? null,
        status: document.querySelector('#bevy-status')?.textContent?.trim() ?? null,
      })),
      { timeout: 10_000, message: 'the loader did not reject the tampered runtime asset' },
    ).toMatchObject({ state: 'error' });

    const status = await page.locator('#bevy-status').textContent();
    expect(status).toContain(`${asset.role} SHA-256 mismatch`);
    expect(fixture.served).toBe(true);
    expect(await page.evaluate(() => globalThis.BOXDD_TEST_RUNTIME_OBJECT_URLS)).toBe(0);
    expect(await page.evaluate(() => globalThis.BOXDD_TEST_WASM_INSTANTIATIONS)).toBe(0);
  } finally {
    tamperedAssetResponse = undefined;
  }
});
