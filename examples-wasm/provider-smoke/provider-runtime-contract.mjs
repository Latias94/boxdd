const PROVIDER_MODULE_PATTERN = /^(box2d-sys-v)(\d+)-(single|double)$/;

export function parseProviderModule(moduleName) {
  const match = PROVIDER_MODULE_PATTERN.exec(moduleName);
  if (!match) {
    throw new Error(`invalid Box2D provider module identity: ${moduleName}`);
  }

  const abiVersion = Number(match[2]);
  if (
    !Number.isSafeInteger(abiVersion) ||
    abiVersion < 0 ||
    String(abiVersion) !== match[2]
  ) {
    throw new Error(`invalid Box2D provider ABI version: ${moduleName}`);
  }

  return {
    module: moduleName,
    abi: `${match[1]}${match[2]}`,
    abiVersion,
    precision: match[3],
  };
}

function providerModule(identity) {
  return `box2d-sys-v${identity.abiVersion}-${identity.precision}`;
}

function providerExport(provider, name) {
  const exported = provider[`_${name}`] ?? provider[name];
  if (typeof exported !== 'function') {
    throw new Error(`provider export ${name} is not a function`);
  }
  return exported;
}

function proveProviderHeapBoundary(provider, providerHeapLimitBytes) {
  const limitBytes = safeInteger(providerHeapLimitBytes, 'provider heap limit');
  if (limitBytes === 0 || limitBytes > 0xffff_ffff) {
    throw new Error(`provider heap limit does not fit wasm32 uintptr_t: ${String(limitBytes)}`);
  }
  const code = providerExport(provider, 'providerHeapBoundaryProbe')(
    limitBytes,
  );
  if (code !== 0) {
    throw new Error(`provider heap boundary probe failed with code ${code}`);
  }
  return {
    limitBytes,
    overflowRejected: true,
  };
}

export function inspectProviderContract(appModule, expectedModule) {
  const expected = parseProviderModule(expectedModule);
  const imports = WebAssembly.Module.imports(appModule).filter(
    (item) => item.kind === 'function' && item.module.startsWith('box2d-sys-'),
  );
  const modules = [...new Set(imports.map((item) => item.module))];
  if (modules.length !== 1) {
    throw new Error(
      `application must import exactly one Box2D provider module; found ${JSON.stringify(modules)}`,
    );
  }

  const actual = parseProviderModule(modules[0]);
  if (actual.module !== expected.module) {
    throw new Error(
      `application provider identity mismatch: expected ${expected.module}, found ${actual.module}`,
    );
  }

  const names = [...new Set(imports.map((item) => item.name))].sort();
  if (names.length === 0) {
    throw new Error(`application does not import functions from ${actual.module}`);
  }
  return { ...actual, names };
}

export function resolveProviderFunctions(provider, names) {
  const refreshMemoryViews = provider.boxddRefreshMemoryViews;
  if (typeof refreshMemoryViews !== 'function') {
    throw new Error('provider does not expose boxddRefreshMemoryViews');
  }
  const initialHeap = refreshMemoryViews();
  if (!(initialHeap instanceof Uint8Array) || initialHeap !== provider.HEAPU8) {
    throw new Error('provider does not expose its canonical Emscripten HEAPU8 view');
  }

  const functions = Object.create(null);
  for (const name of names) {
    const exported = providerExport(provider, name);
    functions[name] = (...args) => {
      refreshMemoryViews();
      return exported(...args);
    };
  }
  return functions;
}

export function createProviderImportObject(memory, moduleName, functions) {
  return {
    env: { memory },
    [moduleName]: { ...functions },
  };
}

async function expectLinkFailure(
  label,
  appModule,
  importObject,
  providerCallCount,
  allowMissingNamespaceTypeError = false,
) {
  try {
    await WebAssembly.instantiate(appModule, importObject);
  } catch (error) {
    if (
      !(error instanceof WebAssembly.LinkError) &&
      !(allowMissingNamespaceTypeError && error instanceof TypeError)
    ) {
      throw new Error(
        `${label} failed with ${error?.constructor?.name || typeof error}, not a WebAssembly link rejection`,
      );
    }
    if (providerCallCount() !== 0) {
      throw new Error(`${label} reached a provider function before link rejection`);
    }
    return true;
  }
  throw new Error(`${label} unexpectedly instantiated the application`);
}

export async function proveProviderLinkFailures(appModule, memory, contract, functions) {
  let providerCalls = 0;
  const watchedFunctions = Object.fromEntries(
    Object.entries(functions).map(([name, exported]) => [
      name,
      (...args) => {
        providerCalls += 1;
        return exported(...args);
      },
    ]),
  );
  const providerCallCount = () => providerCalls;

  const oldProviderAbi = await expectLinkFailure(
    'old provider ABI',
    appModule,
    createProviderImportObject(
      memory,
      providerModule({ ...contract, abiVersion: contract.abiVersion - 1 }),
      watchedFunctions,
    ),
    providerCallCount,
    true,
  );
  const wrongPrecision = await expectLinkFailure(
    'wrong provider precision',
    appModule,
    createProviderImportObject(
      memory,
      providerModule({
        ...contract,
        precision: contract.precision === 'single' ? 'double' : 'single',
      }),
      watchedFunctions,
    ),
    providerCallCount,
    true,
  );

  return {
    oldProviderAbi,
    wrongPrecision,
    providerCallsBeforePhysics: providerCalls,
  };
}

export class RefreshableMemoryViews {
  constructor(memory) {
    this.memory = memory;
    this.refresh();
  }

  refresh() {
    if (this.buffer === this.memory.buffer) return false;
    this.buffer = this.memory.buffer;
    this.bytes = new Uint8Array(this.buffer);
    this.data = new DataView(this.buffer);
    return true;
  }
}

export function captureMemoryViewsBeforeGrowth(memory, views, provider) {
  const providerHeap = provider.boxddRefreshMemoryViews();
  if (providerHeap.buffer !== memory.buffer || providerHeap !== provider.HEAPU8) {
    throw new Error('provider HEAPU8 does not initially bind the shared WebAssembly.Memory');
  }
  return {
    buffer: views.buffer,
    bytes: views.bytes,
    data: views.data,
    providerHeap,
    byteLength: views.buffer.byteLength,
  };
}

export function proveMemoryViewsAfterRustGrowth(memory, views, provider, stale) {
  if (stale.buffer === memory.buffer || memory.buffer.byteLength <= stale.byteLength) {
    throw new Error('Rust allocator did not replace and grow the shared memory buffer');
  }

  let staleTypedArrayRejected = false;
  try {
    stale.bytes.set([1], 0);
  } catch (error) {
    staleTypedArrayRejected = error instanceof TypeError;
  }
  let staleDataViewRejected = false;
  try {
    stale.data.getUint8(0);
  } catch (error) {
    staleDataViewRejected = error instanceof TypeError;
  }
  if (
    stale.buffer.byteLength !== 0 ||
    stale.bytes.byteLength !== 0 ||
    stale.bytes[0] !== undefined ||
    stale.providerHeap.byteLength !== 0 ||
    !staleTypedArrayRejected ||
    !staleDataViewRejected
  ) {
    throw new Error('pre-growth typed memory views remained usable after Rust heap growth');
  }

  views.refresh();
  if (views.buffer !== memory.buffer) {
    throw new Error('typed memory views were not rebound after Rust allocator growth');
  }
  if (!(views.bytes instanceof Uint8Array) || !(views.data instanceof DataView)) {
    throw new Error('refreshed memory views have the wrong JavaScript types');
  }
  const refreshedProviderHeap = provider.boxddRefreshMemoryViews();
  const providerHeapViewRefreshed =
    refreshedProviderHeap instanceof Uint8Array &&
    refreshedProviderHeap === provider.HEAPU8 &&
    refreshedProviderHeap.buffer === memory.buffer;
  if (!providerHeapViewRefreshed) {
    throw new Error('Emscripten HEAPU8 was not rebound after Rust allocator growth');
  }

  const probeOffset = stale.byteLength;
  const original = views.data.getUint32(probeOffset, true);
  views.data.setUint32(probeOffset, 0x78563412, true);
  const refreshedViewsReadWrite =
    views.bytes[probeOffset] === 0x12 &&
    views.bytes[probeOffset + 1] === 0x34 &&
    views.bytes[probeOffset + 2] === 0x56 &&
    views.bytes[probeOffset + 3] === 0x78 &&
    refreshedProviderHeap[probeOffset] === 0x12 &&
    refreshedProviderHeap[probeOffset + 3] === 0x78;
  views.data.setUint32(probeOffset, original, true);
  if (!refreshedViewsReadWrite) {
    throw new Error('refreshed Uint8Array and DataView do not share readable, writable memory');
  }

  return {
    memoryGrew: true,
    staleTypedArrayRejected,
    staleDataViewRejected,
    refreshedViewsReadWrite,
    providerHeapViewRefreshed,
    providerHeapReadWrite: refreshedViewsReadWrite,
  };
}

function requiredExport(instance, name) {
  const exported = instance.exports[name];
  if (typeof exported !== 'function') {
    throw new Error(`${name} export is missing from Rust wasm`);
  }
  return exported;
}

function callAppExport(instance, views, name, ...args) {
  const value = requiredExport(instance, name)(...args);
  views.refresh();
  return value;
}

function safeInteger(value, label) {
  const number = typeof value === 'bigint' ? Number(value) : value;
  if (!Number.isSafeInteger(number) || number < 0) {
    throw new Error(label + ' is not a non-negative safe integer: ' + String(value));
  }
  return number;
}

export async function runProviderPhysicsScenario({
  appModule,
  memory,
  provider,
  providerHeapLimitBytes,
  contract,
  functions,
}) {
  const providerHeapBoundary = proveProviderHeapBoundary(provider, providerHeapLimitBytes);
  const linkFailures = await proveProviderLinkFailures(appModule, memory, contract, functions);
  let memoryGrew = false;
  let providerGlueCallsAfterGrowth = 0;
  const observedFunctions = Object.fromEntries(
    Object.entries(functions).map(([name, exported]) => [
      name,
      (...args) => {
        if (memoryGrew) providerGlueCallsAfterGrowth += 1;
        return exported(...args);
      },
    ]),
  );
  const instance = await WebAssembly.instantiate(
    appModule,
    createProviderImportObject(memory, contract.module, observedFunctions),
  );
  requiredExport(instance, 'boxdd_provider_smoke');

  const memoryViews = new RefreshableMemoryViews(memory);
  const smoke = callAppExport(instance, memoryViews, 'boxdd_provider_smoke');
  if (smoke !== 0) {
    throw new Error('boxdd provider smoke failed with code ' + smoke);
  }
  for (const reset of ['boxdd_allocator_probe_reset', 'boxdd_runtime_reset']) {
    if (callAppExport(instance, memoryViews, reset) !== 0) {
      throw new Error(reset + ' failed before allocator pressure');
    }
  }

  const idleBox2dBytes = safeInteger(
    callAppExport(instance, memoryViews, 'boxdd_provider_box2d_byte_count'),
    'idle Box2D bytes',
  );
  const runtimeInit = callAppExport(instance, memoryViews, 'boxdd_runtime_init');
  if (runtimeInit !== 0) {
    throw new Error('boxdd runtime init failed with code ' + runtimeInit);
  }
  const activeBox2dBytes = safeInteger(
    callAppExport(instance, memoryViews, 'boxdd_provider_box2d_byte_count'),
    'active Box2D bytes',
  );
  if (activeBox2dBytes <= idleBox2dBytes) {
    throw new Error('retained Box2D world did not increase the C-side byte count');
  }

  const staleViews = captureMemoryViewsBeforeGrowth(memory, memoryViews, provider);
  const alignedAllocationBytes = 1024 * 1024;
  const alignedAllocationAlignment = 64 * 1024;
  const alignedPush = callAppExport(
    instance,
    memoryViews,
    'boxdd_allocator_aligned_probe_push',
    alignedAllocationBytes,
    alignedAllocationAlignment,
    0xa7,
  );
  if (
    alignedPush !== 0 ||
    callAppExport(instance, memoryViews, 'boxdd_allocator_aligned_probe_validate') !== 1
  ) {
    throw new Error('Rust allocator failed the explicit 64 KiB alignment probe');
  }
  const pressureChunks = 5;
  const pressureChunkBytes = 16 * 1024 * 1024;
  for (let index = 0; index < pressureChunks; index += 1) {
    const pattern = 0x31 + index;
    const pushed = callAppExport(
      instance,
      memoryViews,
      'boxdd_allocator_probe_push',
      pressureChunkBytes,
      pattern,
    );
    if (pushed !== 0) {
      throw new Error(
        'Rust allocator pressure chunk ' + index + ' failed with code ' + pushed,
      );
    }
    if (memory.buffer.byteLength > staleViews.byteLength) memoryGrew = true;
    const step = callAppExport(instance, memoryViews, 'boxdd_runtime_step');
    if (step < 0) {
      throw new Error('interleaved Box2D runtime step failed with code ' + step);
    }
    const validated = callAppExport(instance, memoryViews, 'boxdd_allocator_probe_validate');
    const alignedValidated = callAppExport(
      instance,
      memoryViews,
      'boxdd_allocator_aligned_probe_validate',
    );
    if (validated !== index + 1 || alignedValidated !== 1) {
      throw new Error('Rust allocation contents failed after interleaved step ' + index);
    }
  }
  if (!memoryGrew) {
    throw new Error('Rust allocator pressure did not grow shared WebAssembly.Memory');
  }

  const pressureBytes = pressureChunks * pressureChunkBytes + alignedAllocationBytes;
  const pressureAllocations = pressureChunks + 1;
  const memoryProof = proveMemoryViewsAfterRustGrowth(
    memory,
    memoryViews,
    provider,
    staleViews,
  );
  const postGrowthMetric = callAppExport(
    instance,
    memoryViews,
    'boxdd_provider_ray_hit_millimeters',
  );
  if (postGrowthMetric < 0) {
    throw new Error('provider failed after memory growth with code ' + postGrowthMetric);
  }
  if (providerGlueCallsAfterGrowth === 0) {
    throw new Error('post-growth physics did not traverse the Emscripten provider exports');
  }
  memoryProof.providerGlueCallsAfterGrowth = providerGlueCallsAfterGrowth;

  if (callAppExport(instance, memoryViews, 'boxdd_allocator_probe_reset') !== 0) {
    throw new Error('Rust allocator pressure reset failed');
  }
  const allocatorProof = {
    pressureAllocations,
    pressureBytes,
    released: true,
    alignmentVerified: true,
    alignedAllocationBytes,
    alignedAllocationAlignment,
    idleBox2dBytes,
    activeBox2dBytes,
  };

  const metricExports = {
    dropMillimeters: 'boxdd_provider_drop_millimeters',
    rayHitMillimeters: 'boxdd_provider_ray_hit_millimeters',
    shapeCastPermyriad: 'boxdd_provider_shape_cast_permyriad',
    jointErrorMillimeters: 'boxdd_provider_joint_error_millimeters',
  };
  const metrics = {};
  for (const [label, exportName] of Object.entries(metricExports)) {
    const value = callAppExport(instance, memoryViews, exportName);
    if (value < 0) {
      throw new Error(`${exportName} failed with code ${value}`);
    }
    metrics[label] = value;
  }

  for (let frame = 0; frame < 30; frame += 1) {
    const code = callAppExport(instance, memoryViews, 'boxdd_runtime_step');
    if (code < 0) throw new Error(`boxdd runtime step failed with code ${code}`);
  }

  const runtimeBodies = callAppExport(instance, memoryViews, 'boxdd_runtime_body_count');
  if (runtimeBodies <= 0) {
    throw new Error(`boxdd runtime body count failed with code ${runtimeBodies}`);
  }
  const runtimeState = [];
  for (let index = 0; index < runtimeBodies; index += 1) {
    const body = {
      shape: callAppExport(instance, memoryViews, 'boxdd_runtime_body_shape', index),
      xMillimeters: callAppExport(
        instance,
        memoryViews,
        'boxdd_runtime_body_x_millimeters',
        index,
      ),
      yMillimeters: callAppExport(
        instance,
        memoryViews,
        'boxdd_runtime_body_y_millimeters',
        index,
      ),
      angleMilliradians: callAppExport(
        instance,
        memoryViews,
        'boxdd_runtime_body_angle_milliradians',
        index,
      ),
      halfWidthMillimeters: callAppExport(
        instance,
        memoryViews,
        'boxdd_runtime_body_half_width_millimeters',
        index,
      ),
      halfHeightMillimeters: callAppExport(
        instance,
        memoryViews,
        'boxdd_runtime_body_half_height_millimeters',
        index,
      ),
      radiusMillimeters: callAppExport(
        instance,
        memoryViews,
        'boxdd_runtime_body_radius_millimeters',
        index,
      ),
    };
    if (body.shape === 1) {
      if (
        body.halfWidthMillimeters <= 0 ||
        body.halfHeightMillimeters <= 0 ||
        body.radiusMillimeters !== 0
      ) {
        throw new Error(`invalid box geometry at runtime body ${index}: ${JSON.stringify(body)}`);
      }
    } else if (body.shape === 2) {
      if (
        body.halfWidthMillimeters !== 0 ||
        body.halfHeightMillimeters !== 0 ||
        body.radiusMillimeters <= 0
      ) {
        throw new Error(`invalid circle geometry at runtime body ${index}: ${JSON.stringify(body)}`);
      }
    } else {
      throw new Error(`unknown runtime shape ${body.shape} at body ${index}`);
    }
    runtimeState.push(body);
  }
  if (callAppExport(instance, memoryViews, 'boxdd_runtime_reset') !== 0) {
    throw new Error('boxdd runtime reset failed');
  }
  const releasedBox2dBytes = safeInteger(
    callAppExport(instance, memoryViews, 'boxdd_provider_box2d_byte_count'),
    'released Box2D bytes',
  );
  if (releasedBox2dBytes !== idleBox2dBytes) {
    throw new Error(
      'Box2D C-side byte count did not return to baseline: ' +
        JSON.stringify({ idleBox2dBytes, activeBox2dBytes, releasedBox2dBytes }),
    );
  }
  allocatorProof.releasedBox2dBytes = releasedBox2dBytes;

  return {
    providerImports: contract.names.length,
    memoryProof,
    linkFailures,
    metrics,
    runtimeBodies,
    runtimeState,
    allocatorProof,
    providerHeapBoundary,
  };
}
