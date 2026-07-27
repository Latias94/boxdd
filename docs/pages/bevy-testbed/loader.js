const runtimeTrust = null;
const runtimeManifestUrl = new URL("../wasm/generated/boxdd-pages-runtime-v2.json", import.meta.url);
const expectedAssets = Object.freeze([
  Object.freeze({ role: "provider_js", path: "wasm/generated/box2d-sys-v1-single.js" }),
  Object.freeze({ role: "provider_wasm", path: "wasm/generated/box2d-sys-v1-single.wasm" }),
  Object.freeze({ role: "app_js", path: "bevy-testbed/generated/bevy_boxdd_testbed.js" }),
  Object.freeze({ role: "app_wasm", path: "bevy-testbed/generated/bevy_boxdd_testbed_bg.wasm" }),
  Object.freeze({ role: "provider_shim_js", path: "bevy-testbed/generated/box2d-provider-shim.js" }),
]);
const manifestKeys = Object.freeze([
  "adapter_abi_version",
  "adapter_source_sha256",
  "assets",
  "crate_version",
  "emscripten_sdk_contract_sha256",
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
const identityKeys = Object.freeze(manifestKeys.filter((key) => key !== "assets"));
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

async function fetchArrayBufferWithProgress(url, label) {
  setStatus("loading", `Downloading ${label}`, "Starting download.", { loaded: 0, total: 0 });
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`${label} download failed with HTTP ${response.status}`);
  }

  const total = Number(response.headers.get("Content-Length")) || 0;
  if (!response.body) {
    const buffer = await response.arrayBuffer();
    setStatus("loading", `Downloading ${label}`, "Download complete.", {
      loaded: buffer.byteLength,
      total: total || buffer.byteLength,
    });
    return buffer;
  }

  const reader = response.body.getReader();
  const chunks = [];
  let loaded = 0;
  for (;;) {
    const { done, value } = await reader.read();
    if (done) {
      break;
    }
    chunks.push(value);
    loaded += value.byteLength;
    setStatus("loading", `Downloading ${label}`, "Downloading runtime asset.", { loaded, total });
  }

  const bytes = new Uint8Array(loaded);
  let offset = 0;
  for (const chunk of chunks) {
    bytes.set(chunk, offset);
    offset += chunk.byteLength;
  }
  setStatus("loading", `Downloading ${label}`, "Download complete.", { loaded, total: total || loaded });
  return bytes.buffer;
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
  if (!runtimeTrust) {
    throw new Error("Pages runtime trust anchor is absent; publish assets with build-pages-wasm");
  }
  const bytes = await fetchArrayBufferWithProgress(runtimeManifestUrl, "runtime manifest");
  await verifySha256(bytes, runtimeTrust.manifest_sha256, "runtime manifest");

  let manifest;
  try {
    manifest = JSON.parse(decodeUtf8(bytes, "runtime manifest"));
  } catch (error) {
    throw new Error("runtime manifest is not valid JSON", { cause: error });
  }
  assertExactObjectKeys(manifest, manifestKeys, "runtime manifest");
  for (const key of identityKeys) {
    if (manifest[key] !== runtimeTrust[key]) {
      throw new Error(`runtime manifest identity ${key} does not match the loader trust anchor`);
    }
  }
  if (!Array.isArray(manifest.assets) || manifest.assets.length !== expectedAssets.length) {
    throw new Error("runtime manifest must contain the exact qualified asset set");
  }
  manifest.assets.forEach((asset, index) => {
    assertExactObjectKeys(asset, assetKeys, `runtime asset ${index}`);
    const expected = expectedAssets[index];
    if (asset.role !== expected.role || asset.path !== expected.path) {
      throw new Error(`runtime asset ${index} does not match the canonical role and path`);
    }
    if (!Number.isSafeInteger(asset.byte_length) || asset.byte_length <= 0) {
      throw new Error(`runtime asset ${asset.role} has an invalid byte length`);
    }
    if (!/^[0-9a-f]{64}$/.test(asset.sha256)) {
      throw new Error(`runtime asset ${asset.role} has an invalid SHA-256`);
    }
  });
  return manifest;
}

async function loadVerifiedRuntimeAssets(manifest) {
  const verified = new Map();
  for (const asset of manifest.assets) {
    const bytes = await fetchArrayBufferWithProgress(pageAssetUrl(asset.path), asset.role);
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

function replaceShimImport(appBytes, shimModuleUrl) {
  const source = decodeUtf8(appBytes, "Bevy app JavaScript");
  const shimModuleName = "box2d-provider-shim.js";
  const specifier = '"./box2d-provider-shim.js"';
  const shimImportPattern = /^import \* as (import[0-9]+) from "\.\/box2d-provider-shim\.js";?$/;
  const importLines = [];
  const importBindings = new Set();
  for (const [lineNumber, line] of source.split(/\r?\n/).entries()) {
    if (!line.includes(shimModuleName)) {
      continue;
    }
    const match = shimImportPattern.exec(line);
    if (!match || importBindings.has(match[1])) {
      throw new Error("Bevy app JavaScript contains an unsupported wasm-bindgen provider shim import");
    }
    importBindings.add(match[1]);
    importLines.push(lineNumber);
  }
  if (
    importLines.length === 0 ||
    importLines.some((lineNumber, offset) => lineNumber !== importLines[0] + offset) ||
    !importBindings.has("import1")
  ) {
    throw new Error("Bevy app JavaScript must contain one contiguous block of qualified wasm-bindgen provider shim imports");
  }
  return source.replaceAll(specifier, JSON.stringify(shimModuleUrl));
}

async function importVerifiedRuntimeModules(assets) {
  const shimUrl = URL.createObjectURL(
    new Blob([assets.get("provider_shim_js")], { type: "text/javascript" }),
  );
  const providerUrl = URL.createObjectURL(
    new Blob([assets.get("provider_js")], { type: "text/javascript" }),
  );
  const appSource = replaceShimImport(assets.get("app_js"), shimUrl);
  const appUrl = URL.createObjectURL(new Blob([appSource], { type: "text/javascript" }));

  try {
    return await Promise.all([import(providerUrl), import(appUrl), import(shimUrl)]);
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
    { boxddProviderRuntimeEvidence, setBox2dProvider, setBoxddAppExports },
  ] = await importVerifiedRuntimeModules(assets);
  const memory = new WebAssembly.Memory({ initial: 4096, maximum: 8192 });

  setStatus("loading", "Starting Box2D provider", `Instantiating the shared Box2D C provider for ${sceneName}.`);
  const provider = await createProvider({
    wasmMemory: memory,
    wasmBinary: assets.get("provider_wasm"),
    locateFile: (path) => pageAssetUrl(`wasm/generated/${path}`).href,
    print: (text) => console.log(`[box2d-sys-v1-single] ${text}`),
    printErr: (text) => console.warn(`[box2d-sys-v1-single] ${text}`),
  });

  if (provider.wasmMemory && provider.wasmMemory !== memory) {
    throw new Error("Box2D provider did not use the shared WebAssembly.Memory");
  }
  const adapterAbiVersion = provider._boxddAdapter_AbiVersion || provider.boxddAdapter_AbiVersion;
  if (typeof adapterAbiVersion !== "function" || adapterAbiVersion() !== manifest.adapter_abi_version) {
    throw new Error("Box2D provider runtime adapter ABI does not match the verified manifest");
  }

  setBox2dProvider(provider);
  setStatus("loading", `Starting ${sceneName}`, "Instantiating the Rust Bevy + egui wasm module.");

  const bevyExports = await initBevyTestbed({
    module_or_path: assets.get("app_wasm"),
    memory,
  });
  setBoxddAppExports(bevyExports);

  const initialEvidence = await waitForProviderStep(
    boxddProviderRuntimeEvidence,
    0,
    "initial runtime proof",
  );
  const proofRequested = new URLSearchParams(window.location.search).get("boxdd-runtime-proof") === "1";
  const memoryProof = {
    requested: proofRequested,
    memoryGrew: false,
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
    memoryProof.memoryGrew = memory.buffer !== staleBuffer;
    memoryProof.staleBufferDetached = staleBuffer.byteLength === 0;
    memoryProof.byteLengthAfterGrowth = memory.buffer.byteLength;
    if (
      !memoryProof.memoryGrew ||
      !memoryProof.staleBufferDetached ||
      staleProviderHeap.byteLength !== 0 ||
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
      throw new Error("Emscripten HEAPU8 was not rebound after external memory.grow");
    }
    const probeOffset = memoryProof.byteLengthBeforeGrowth;
    const refreshedData = new DataView(memory.buffer);
    const original = refreshedData.getUint32(probeOffset, true);
    refreshedProviderHeap.set([0x12, 0x34, 0x56, 0x78], probeOffset);
    memoryProof.providerHeapReadWrite =
      refreshedData.getUint32(probeOffset, true) === 0x78563412;
    refreshedData.setUint32(probeOffset, original, true);
    if (!memoryProof.providerHeapReadWrite) {
      throw new Error("refreshed Emscripten HEAPU8 is not readable and writable");
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
