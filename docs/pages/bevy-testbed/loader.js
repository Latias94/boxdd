const runtimeContract = null;
const runtimeManifestUrl = new URL("../wasm/generated/boxdd-pages-runtime-v2.json", import.meta.url);
const MAX_RUNTIME_MANIFEST_BYTES = 1024 * 1024;
const MAX_RUNTIME_ASSET_BYTES = 536870912;
const MAX_RUNTIME_TOTAL_ASSET_BYTES = 536870912;
const RUNTIME_FETCH_INACTIVITY_TIMEOUT_MS = 30 * 1000;
const RUNTIME_FETCH_TOTAL_TIMEOUT_MS = 5 * 60 * 1000;
const expectedAssets = Object.freeze([
  Object.freeze({ role: "provider_js", path: "wasm/generated/box2d-sys-v2-single.js" }),
  Object.freeze({ role: "provider_wasm", path: "wasm/generated/box2d-sys-v2-single.wasm" }),
  Object.freeze({ role: "app_js", path: "bevy-testbed/generated-v2/bevy_boxdd_testbed.js" }),
  Object.freeze({ role: "app_wasm", path: "bevy-testbed/generated-v2/bevy_boxdd_testbed_bg.wasm" }),
  Object.freeze({ role: "provider_shim_js", path: "bevy-testbed/generated-v2/box2d-provider-shim.js" }),
]);
const manifestKeys = Object.freeze([
  "adapter_abi_version",
  "adapter_source_sha256",
  "assets",
  "crate_version",
  "emscripten_version",
  "effective_source_sha256",
  "precision",
  "provider",
  "provider_abi",
  "publisher_repository",
  "publisher_workflow",
  "recording_contract_blake3",
  "schema",
  "schema_version",
  "source_commit",
  "source_tree",
  "target",
  "upstream_sha",
  "wasm_provider_contract_sha256",
]);
const contractKeys = Object.freeze([
  "adapter_abi_version",
  "precision",
  "provider",
  "provider_abi",
  "schema",
  "schema_version",
  "target",
]);
const assetKeys = Object.freeze(["byte_length", "path", "role", "sha256"]);

const statusPanel = document.querySelector("#bevy-status");
const appRoot = document.querySelector("#bevy-app");
const sceneId = appRoot?.dataset.sceneId || "";
const sceneName = appRoot?.dataset.sceneName || "Bevy testbed";
const isExamplePage = Boolean(sceneId);

function setStatus(state, title, detail, progress) {
  statusPanel.dataset.state = state;
  statusPanel.replaceChildren();

  const titleNode = document.createElement("strong");
  titleNode.textContent = title;
  const detailNode = document.createElement("span");
  detailNode.textContent = detail;
  statusPanel.append(titleNode, detailNode);

  if (progress) {
    const progressNode = document.createElement("progress");
    progressNode.value = progress.loaded;
    if (progress.total) {
      progressNode.max = progress.total;
    } else {
      progressNode.removeAttribute("value");
    }

    const progressText = document.createElement("small");
    progressText.textContent = progressTextFor(progress.loaded, progress.total);
    statusPanel.append(progressNode, progressText);
  }
}

function pageAssetUrl(path) {
  return new URL(`../${path}`, import.meta.url);
}

function runtimeAssetUrl(asset) {
  const url = pageAssetUrl(asset.path);
  url.searchParams.set("sha256", asset.sha256);
  return url;
}

function progressTextFor(loaded, total) {
  if (total) {
    const percent = Math.min(100, Math.round((loaded / total) * 100));
    return `${formatBytes(loaded)} / ${formatBytes(total)} (${percent}%)`;
  }
  return `${formatBytes(loaded)} downloaded`;
}

function formatBytes(bytes) {
  if (!Number.isFinite(bytes) || bytes <= 0) {
    return "0 B";
  }
  const units = ["B", "KiB", "MiB", "GiB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return unit === 0 ? `${value} ${units[unit]}` : `${value.toFixed(2)} ${units[unit]}`;
}

async function fetchArrayBufferWithProgress(url, label, maxBytes, cache = "default") {
  if (!Number.isSafeInteger(maxBytes) || maxBytes <= 0) {
    throw new Error(`${label} byte limit is invalid`);
  }
  setStatus("loading", `Downloading ${label}`, "Starting download.", { loaded: 0, total: 0 });
  const controller = new AbortController();
  let inactivityTimeout;
  let inactivityExpired = false;
  let totalTimeout;
  let totalExpired = false;
  const armInactivityTimeout = () => {
    clearTimeout(inactivityTimeout);
    inactivityTimeout = setTimeout(() => {
      inactivityExpired = true;
      controller.abort();
    }, RUNTIME_FETCH_INACTIVITY_TIMEOUT_MS);
  };
  const disarmInactivityTimeout = () => clearTimeout(inactivityTimeout);
  const disarmTotalTimeout = () => clearTimeout(totalTimeout);

  try {
    totalTimeout = setTimeout(() => {
      totalExpired = true;
      controller.abort();
    }, RUNTIME_FETCH_TOTAL_TIMEOUT_MS);
    armInactivityTimeout();
    const response = await fetch(url, { cache, signal: controller.signal });
    disarmInactivityTimeout();
    if (!response.ok) {
      throw new Error(`${label} download failed with HTTP ${response.status}`);
    }

    const contentEncoding = response.headers.get("Content-Encoding");
    const identityEncoded = contentEncoding === null
      || contentEncoding.trim().toLowerCase() === "identity";
    const contentLength = identityEncoded ? response.headers.get("Content-Length") : null;
    let contentLengthKnown = false;
    let total = 0;
    if (contentLength !== null) {
      contentLengthKnown = true;
      if (!/^(0|[1-9][0-9]*)$/.test(contentLength)) {
        await response.body?.cancel();
        throw new Error(`${label} Content-Length is invalid`);
      }
      total = Number(contentLength);
      if (!Number.isSafeInteger(total)) {
        await response.body?.cancel();
        throw new Error(`${label} Content-Length exceeds the browser integer limit`);
      }
      if (total > maxBytes) {
        await response.body?.cancel();
        throw new Error(`${label} exceeds its ${maxBytes}-byte limit`);
      }
    }
    if (!response.body) {
      throw new Error(`${label} response does not expose a readable stream`);
    }

    const reader = response.body.getReader();
    const bytes = contentLengthKnown ? new Uint8Array(total) : null;
    const chunks = bytes ? null : [];
    let loaded = 0;
    for (;;) {
      armInactivityTimeout();
      const { done, value } = await reader.read();
      disarmInactivityTimeout();
      if (done) {
        break;
      }
      const nextLoaded = loaded + value.byteLength;
      if (nextLoaded > maxBytes) {
        await reader.cancel();
        throw new Error(`${label} exceeds its ${maxBytes}-byte limit`);
      }
      if (contentLengthKnown && nextLoaded > total) {
        await reader.cancel();
        throw new Error(`${label} exceeds its declared Content-Length`);
      }
      if (bytes) {
        bytes.set(value, loaded);
      } else {
        chunks.push(value);
      }
      loaded = nextLoaded;
      setStatus("loading", `Downloading ${label}`, "Downloading runtime asset.", { loaded, total });
    }

    if (contentLengthKnown && loaded !== total) {
      throw new Error(`${label} ended after ${loaded} bytes; expected ${total}`);
    }
    let complete = bytes;
    if (!complete) {
      complete = new Uint8Array(loaded);
      let offset = 0;
      for (const chunk of chunks) {
        complete.set(chunk, offset);
        offset += chunk.byteLength;
      }
    }
    setStatus("loading", `Downloading ${label}`, "Download complete.", { loaded, total: total || loaded });
    return complete.buffer;
  } catch (error) {
    if (totalExpired) {
      throw new Error(
        `${label} download exceeded ${RUNTIME_FETCH_TOTAL_TIMEOUT_MS}ms`,
        { cause: error },
      );
    }
    if (inactivityExpired) {
      throw new Error(
        `${label} download stalled for ${RUNTIME_FETCH_INACTIVITY_TIMEOUT_MS}ms`,
        { cause: error },
      );
    }
    throw error;
  } finally {
    disarmInactivityTimeout();
    disarmTotalTimeout();
  }
}

function assertExactObjectKeys(value, expected, label) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new Error(`${label} must be an object`);
  }
  const actual = Object.keys(value).sort();
  const required = [...expected].sort();
  if (actual.length !== required.length || actual.some((key, index) => key !== required[index])) {
    throw new Error(`${label} fields do not match the canonical schema`);
  }
}

function decodeUtf8(bytes, label) {
  try {
    return new TextDecoder("utf-8", { fatal: true }).decode(bytes);
  } catch (error) {
    throw new Error(`${label} is not valid UTF-8`, { cause: error });
  }
}

async function sha256Hex(bytes) {
  if (!globalThis.crypto?.subtle) {
    throw new Error("Web Crypto SHA-256 is unavailable; refusing unverified runtime assets");
  }
  const digest = await globalThis.crypto.subtle.digest("SHA-256", bytes);
  return Array.from(new Uint8Array(digest), (byte) => byte.toString(16).padStart(2, "0")).join("");
}

async function verifySha256(bytes, expected, label) {
  if (!/^[0-9a-f]{64}$/.test(expected)) {
    throw new Error(`${label} manifest SHA-256 is malformed`);
  }
  const actual = await sha256Hex(bytes);
  if (actual !== expected) {
    throw new Error(`${label} SHA-256 mismatch: expected ${expected}, got ${actual}`);
  }
}

async function loadRuntimeManifest() {
  if (!runtimeContract) {
    throw new Error("Pages runtime contract is absent; publish assets with build-pages-wasm");
  }
  const bytes = await fetchArrayBufferWithProgress(
    runtimeManifestUrl,
    "runtime manifest",
    MAX_RUNTIME_MANIFEST_BYTES,
    "no-store",
  );

  let manifest;
  try {
    manifest = JSON.parse(decodeUtf8(bytes, "runtime manifest"));
  } catch (error) {
    throw new Error("runtime manifest is not valid JSON", { cause: error });
  }
  assertExactObjectKeys(manifest, manifestKeys, "runtime manifest");
  for (const key of contractKeys) {
    if (manifest[key] !== runtimeContract[key]) {
      throw new Error(`runtime manifest ${key} does not match the loader contract`);
    }
  }
  for (const [key, digits] of [
    ["source_commit", 40],
    ["source_tree", 40],
    ["upstream_sha", 40],
    ["adapter_source_sha256", 64],
    ["effective_source_sha256", 64],
    ["recording_contract_blake3", 64],
    ["wasm_provider_contract_sha256", 64],
  ]) {
    if (typeof manifest[key] !== "string" || !new RegExp(`^[0-9a-f]{${digits}}$`).test(manifest[key])) {
      throw new Error(`runtime manifest ${key} is malformed`);
    }
  }
  for (const key of ["crate_version", "emscripten_version", "publisher_repository", "publisher_workflow"]) {
    if (typeof manifest[key] !== "string" || manifest[key].length === 0) {
      throw new Error(`runtime manifest ${key} must be a non-empty string`);
    }
  }
  if (!Array.isArray(manifest.assets) || manifest.assets.length !== expectedAssets.length) {
    throw new Error("runtime manifest must contain the exact qualified asset set");
  }
  let totalAssetBytes = 0;
  manifest.assets.forEach((asset, index) => {
    assertExactObjectKeys(asset, assetKeys, `runtime asset ${index}`);
    const expected = expectedAssets[index];
    if (asset.role !== expected.role || asset.path !== expected.path) {
      throw new Error(`runtime asset ${index} does not match the canonical role and path`);
    }
    if (
      !Number.isSafeInteger(asset.byte_length)
      || asset.byte_length <= 0
      || asset.byte_length > MAX_RUNTIME_ASSET_BYTES
    ) {
      throw new Error(`runtime asset ${asset.role} has an invalid byte length`);
    }
    if (!/^[0-9a-f]{64}$/.test(asset.sha256)) {
      throw new Error(`runtime asset ${asset.role} has an invalid SHA-256`);
    }
    totalAssetBytes += asset.byte_length;
    if (!Number.isSafeInteger(totalAssetBytes) || totalAssetBytes > MAX_RUNTIME_TOTAL_ASSET_BYTES) {
      throw new Error("runtime assets exceed the qualified cohort byte limit");
    }
  });
  return manifest;
}

async function loadVerifiedRuntimeAssets(manifest) {
  const verified = new Map();
  for (const asset of manifest.assets) {
    const bytes = await fetchArrayBufferWithProgress(runtimeAssetUrl(asset), asset.role, asset.byte_length);
    if (bytes.byteLength !== asset.byte_length) {
      throw new Error(
        `${asset.role} byte length mismatch: expected ${asset.byte_length}, got ${bytes.byteLength}`,
      );
    }
    await verifySha256(bytes, asset.sha256, asset.role);
    verified.set(asset.role, bytes);
  }
  return verified;
}

async function importVerifiedRuntimeModules(assets) {
  const shimUrl = URL.createObjectURL(
    new Blob([assets.get("provider_shim_js")], { type: "text/javascript" }),
  );
  const providerUrl = URL.createObjectURL(
    new Blob([assets.get("provider_js")], { type: "text/javascript" }),
  );
  const appUrl = URL.createObjectURL(
    new Blob([assets.get("app_js")], { type: "text/javascript" }),
  );

  try {
    if (!HTMLScriptElement.supports?.("importmap")) {
      throw new Error("This browser does not support import maps");
    }
    const [providerModule, shimModule] = await Promise.all([
      import(providerUrl),
      import(shimUrl),
    ]);
    const importMap = document.createElement("script");
    importMap.type = "importmap";
    importMap.textContent = JSON.stringify({
      imports: { "box2d-sys-v2-single": shimUrl },
    });
    document.head.append(importMap);
    const appModule = await import(appUrl);
    return [providerModule, appModule, shimModule];
  } finally {
    URL.revokeObjectURL(appUrl);
    URL.revokeObjectURL(providerUrl);
    URL.revokeObjectURL(shimUrl);
  }
}

async function waitForProviderStep(providerEvidence, previousSteps, label) {
  const deadline = performance.now() + 20_000;
  for (;;) {
    const evidence = providerEvidence();
    if (
      Number.isSafeInteger(evidence.providerCalls) &&
      Number.isSafeInteger(evidence.stepCalls) &&
      evidence.providerCalls >= evidence.stepCalls &&
      evidence.stepCalls > previousSteps
    ) {
      return evidence;
    }
    if (performance.now() >= deadline) {
      throw new Error(`${label} did not observe a Box2D physics step before the deadline`);
    }
    await new Promise((resolve) => requestAnimationFrame(resolve));
  }
}

async function main() {
  setStatus("loading", "Verifying runtime identity", `Checking the published assets for ${sceneName}.`);
  const manifest = await loadRuntimeManifest();
  const assets = await loadVerifiedRuntimeAssets(manifest);
  setStatus("loading", "Loading verified modules", `Preparing the browser runtime for ${sceneName}.`);
  const [
    { default: createProvider },
    { default: initBevyTestbed },
    { boxddProviderRuntimeEvidence, setBox2dProvider },
  ] = await importVerifiedRuntimeModules(assets);
  const memory = new WebAssembly.Memory({
    initial: 2048,
    maximum: 8192,
  });
  const byteLengthBeforeApp = memory.buffer.byteLength;

  setStatus("loading", "Starting Box2D provider", `Instantiating the shared Box2D C provider for ${sceneName}.`);
  const provider = await createProvider({
    wasmMemory: memory,
    wasmBinary: assets.get("provider_wasm"),
    locateFile: (path) => pageAssetUrl(`wasm/generated/${path}`).href,
    print: (text) => console.log(`[box2d-sys-v2-single] ${text}`),
    printErr: (text) => console.warn(`[box2d-sys-v2-single] ${text}`),
  });

  if (provider.wasmMemory && provider.wasmMemory !== memory) {
    throw new Error("Box2D provider did not use the shared WebAssembly.Memory");
  }
  const adapterAbiVersion = provider._boxddAdapter_AbiVersion || provider.boxddAdapter_AbiVersion;
  if (typeof adapterAbiVersion !== "function" || adapterAbiVersion() !== manifest.adapter_abi_version) {
    throw new Error("Box2D provider runtime adapter ABI does not match the verified manifest");
  }
  const proofRequested = new URLSearchParams(window.location.search).get("boxdd-runtime-proof") === "1";

  setBox2dProvider(provider);
  setStatus("loading", `Starting ${sceneName}`, "Instantiating the Rust Bevy + egui wasm module.");

  await initBevyTestbed({
    module_or_path: assets.get("app_wasm"),
    memory,
  });

  const initialEvidence = await waitForProviderStep(
    boxddProviderRuntimeEvidence,
    0,
    "initial runtime proof",
  );
  const memoryProof = {
    requested: proofRequested,
    memoryGrew: false,
    growthObservedDuringApp: memory.buffer.byteLength > byteLengthBeforeApp,
    externalGrowth: false,
    staleBufferDetached: false,
    providerHeapViewRefreshed: false,
    providerHeapReadWrite: false,
    postGrowthPhysicsStep: false,
    byteLengthBeforeGrowth: memory.buffer.byteLength,
    byteLengthAfterGrowth: memory.buffer.byteLength,
    stepCallsBeforeGrowth: initialEvidence.stepCalls,
    stepCallsAfterGrowth: initialEvidence.stepCalls,
  };
  if (proofRequested) {
    const staleBuffer = memory.buffer;
    const staleProviderHeap = provider.boxddRefreshMemoryViews();
    if (staleProviderHeap !== provider.HEAPU8 || staleProviderHeap.buffer !== staleBuffer) {
      throw new Error("Box2D provider HEAPU8 does not bind the shared WebAssembly.Memory");
    }
    memory.grow(1);
    memoryProof.externalGrowth = true;
    memoryProof.memoryGrew = memory.buffer !== staleBuffer;
    memoryProof.staleBufferDetached =
      staleBuffer.byteLength === 0 && staleProviderHeap.byteLength === 0;
    memoryProof.byteLengthAfterGrowth = memory.buffer.byteLength;
    if (
      !memoryProof.memoryGrew ||
      !memoryProof.staleBufferDetached ||
      memoryProof.byteLengthAfterGrowth <= memoryProof.byteLengthBeforeGrowth
    ) {
      throw new Error("shared WebAssembly.Memory did not detach and grow its buffer");
    }

    const refreshedProviderHeap = provider.boxddRefreshMemoryViews();
    memoryProof.providerHeapViewRefreshed =
      refreshedProviderHeap instanceof Uint8Array &&
      refreshedProviderHeap === provider.HEAPU8 &&
      refreshedProviderHeap.buffer === memory.buffer;
    if (!memoryProof.providerHeapViewRefreshed) {
      throw new Error("Emscripten HEAPU8 was not rebound after external memory growth");
    }
    const probeOffset = memoryProof.byteLengthBeforeGrowth;
    const refreshedData = new DataView(memory.buffer);
    refreshedData.setUint32(probeOffset, 0x78563412, true);
    memoryProof.providerHeapReadWrite =
      refreshedProviderHeap[probeOffset] === 0x12 &&
      refreshedProviderHeap[probeOffset + 3] === 0x78;
    if (!memoryProof.providerHeapReadWrite) {
      throw new Error("Emscripten HEAPU8 and DataView do not share the grown memory");
    }

    const postGrowthEvidence = await waitForProviderStep(
      boxddProviderRuntimeEvidence,
      memoryProof.stepCallsBeforeGrowth,
      "post-growth runtime proof",
    );
    memoryProof.stepCallsAfterGrowth = postGrowthEvidence.stepCalls;
    memoryProof.postGrowthPhysicsStep = true;
  }

  window.BOXDD_BEVY_RUNTIME_EVIDENCE = () => {
    const evidence = boxddProviderRuntimeEvidence();
    return Object.freeze({
      providerCalls: evidence.providerCalls,
      stepCalls: evidence.stepCalls,
      memoryProof: Object.freeze({ ...memoryProof }),
    });
  };

  window.BOXDD_BEVY_TESTBED_READY = true;
  window.BOXDD_BEVY_EXAMPLE_READY = true;
  window.BOXDD_BEVY_SCENE_ID = sceneId;
  setStatus(
    "running",
    `${sceneName} running`,
    isExamplePage
      ? "This dedicated example page is running the selected Box2D scene in Bevy."
      : "The scene browser, egui controls, and Box2D simulation are running in this canvas.",
  );
}

main().catch((error) => {
  console.error(error);
  const message = error instanceof Error ? error.message : String(error);
  setStatus("error", `${sceneName} failed`, message);
});
