import { expect, test } from '@playwright/test';
import { constants } from 'node:fs';
import { createServer } from 'node:http';
import { lstat, open, realpath } from 'node:fs/promises';
import { dirname, extname, isAbsolute, join, relative, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const fixtureRoot = dirname(fileURLToPath(import.meta.url));
const repositoryRoot = resolve(fixtureRoot, '../..');
const pagesRoot = resolve(repositoryRoot, 'docs/pages');
const runtimeManifestPath = '/wasm/generated/boxdd-pages-runtime-v1.json';
const runtimeBuildCommand = 'cargo run --locked -p xtask -- build-pages-wasm';

let canonicalPagesRoot;
let origin;
let server;

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
  }

  const loader = await readOrdinaryPagesFile('/bevy-testbed/loader.js');
  if (loader.includes('const runtimeTrust = null;')) {
    throw new Error(
      `Pages loader has no runtime trust anchor; run \`${runtimeBuildCommand}\` before this test`,
    );
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
      const body = await readOrdinaryPagesFile(pathname);
      const responsePath = pathname.endsWith('/') ? `${pathname}index.html` : pathname;
      response.writeHead(200, {
        'Cache-Control': 'no-store',
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

test('published Bevy runtime survives shared-memory growth and keeps stepping physics', async ({ page }) => {
  const consoleErrors = [];
  const pageErrors = [];
  const requestFailures = [];
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
    requestFailures.push(`${request.method()} ${request.url()}: ${request.failure()?.errorText ?? 'unknown error'}`);
  });
  page.on('response', (response) => {
    if (response.status() < 200 || response.status() >= 300) {
      unexpectedHttp.push(`HTTP ${response.status()}: ${response.url()}`);
    }
  });

  await page.goto(`${origin}/bevy-testbed/?boxdd-runtime-proof=1`, { waitUntil: 'domcontentloaded' });
  await expect.poll(
    () => page.evaluate(() => ({
      ready: window.BOXDD_BEVY_TESTBED_READY === true,
      state: document.querySelector('#bevy-status')?.dataset.state ?? null,
      status: document.querySelector('#bevy-status')?.textContent?.trim() ?? null,
    })),
    { timeout: 45_000, message: 'the verified Bevy runtime did not reach its running state' },
  ).toMatchObject({ ready: true, state: 'running' });

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
  expect(Object.keys(evidence).sort()).toEqual(['memoryProof', 'providerCalls', 'stepCalls']);
  expect(evidence.providerCalls).toBeGreaterThan(0);
  expect(evidence.stepCalls).toBeGreaterThan(0);
  expect(Object.keys(evidence.memoryProof).sort()).toEqual([
    'byteLengthAfterGrowth',
    'byteLengthBeforeGrowth',
    'memoryGrew',
    'postGrowthPhysicsStep',
    'requested',
    'staleBufferDetached',
    'stepCallsAfterGrowth',
    'stepCallsBeforeGrowth',
  ]);
  expect(evidence.memoryProof).toMatchObject({
    memoryGrew: true,
    postGrowthPhysicsStep: true,
    requested: true,
    staleBufferDetached: true,
  });
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
  expect(requestFailures).toEqual([]);
  expect(unexpectedHttp).toEqual([]);
});
