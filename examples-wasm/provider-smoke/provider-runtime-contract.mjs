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
  const functions = Object.create(null);
  for (const name of names) {
    const exported = provider[`_${name}`] ?? provider[name];
    if (typeof exported !== 'function') {
      throw new Error(`provider export ${name} is not a function`);
    }
    functions[name] = exported;
  }
  return functions;
}

export function createProviderImportObject(memory, moduleName, functions) {
  return {
    env: { memory },
    [moduleName]: { ...functions },
  };
}

async function createIncompatibleWasmFunction() {
  // (module (func (export "wrong") (param externref) (result externref) local.get 0))
  // Box2D's C ABI imports use only numeric Wasm value types, so this callable can never match.
  const module = await WebAssembly.compile(
    Uint8Array.of(
      0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00,
      0x01, 0x06, 0x01, 0x60, 0x01, 0x6f, 0x01, 0x6f,
      0x03, 0x02, 0x01, 0x00,
      0x07, 0x09, 0x01, 0x05, 0x77, 0x72, 0x6f, 0x6e, 0x67, 0x00, 0x00,
      0x0a, 0x06, 0x01, 0x04, 0x00, 0x20, 0x00, 0x0b,
    ),
  );
  const instance = await WebAssembly.instantiate(module);
  const incompatible = instance.exports.wrong;
  if (typeof incompatible !== 'function') {
    throw new Error('failed to construct a callable Wasm function with an incompatible signature');
  }
  return incompatible;
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

  const wrongFunctions = { ...watchedFunctions };
  wrongFunctions[contract.names[0]] = await createIncompatibleWasmFunction();
  const wrongFunctionType = await expectLinkFailure(
    'wrong provider Wasm function signature',
    appModule,
    createProviderImportObject(memory, contract.module, wrongFunctions),
    providerCallCount,
  );

  return {
    oldProviderAbi,
    wrongPrecision,
    wrongFunctionType,
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

export function growAndProveMemoryViews(memory, views) {
  const staleBuffer = views.buffer;
  const staleBytes = views.bytes;
  const staleData = views.data;
  const oldByteLength = staleBuffer.byteLength;

  memory.grow(1);
  if (staleBuffer === memory.buffer || memory.buffer.byteLength <= oldByteLength) {
    throw new Error('shared WebAssembly.Memory did not replace and grow its buffer');
  }

  let staleTypedArrayRejected = false;
  try {
    staleBytes.set([1], 0);
  } catch (error) {
    staleTypedArrayRejected = error instanceof TypeError;
  }
  let staleDataViewRejected = false;
  try {
    staleData.getUint8(0);
  } catch (error) {
    staleDataViewRejected = error instanceof TypeError;
  }
  if (
    staleBuffer.byteLength !== 0 ||
    staleBytes.byteLength !== 0 ||
    staleBytes[0] !== undefined ||
    !staleTypedArrayRejected ||
    !staleDataViewRejected
  ) {
    throw new Error('pre-growth typed memory views remained usable after memory.grow');
  }

  if (!views.refresh() || views.buffer !== memory.buffer) {
    throw new Error('typed memory views were not rebound after memory.grow');
  }
  if (!(views.bytes instanceof Uint8Array) || !(views.data instanceof DataView)) {
    throw new Error('refreshed memory views have the wrong JavaScript types');
  }

  const probeOffset = oldByteLength;
  const original = views.data.getUint32(probeOffset, true);
  views.data.setUint32(probeOffset, 0x78563412, true);
  const refreshedViewsReadWrite =
    views.bytes[probeOffset] === 0x12 &&
    views.bytes[probeOffset + 1] === 0x34 &&
    views.bytes[probeOffset + 2] === 0x56 &&
    views.bytes[probeOffset + 3] === 0x78;
  views.data.setUint32(probeOffset, original, true);
  if (!refreshedViewsReadWrite) {
    throw new Error('refreshed Uint8Array and DataView do not share readable, writable memory');
  }

  return {
    memoryGrew: true,
    staleTypedArrayRejected,
    staleDataViewRejected,
    refreshedViewsReadWrite,
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

export async function runProviderPhysicsScenario({ appModule, memory, contract, functions }) {
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
  const memoryProof = growAndProveMemoryViews(memory, memoryViews);
  memoryGrew = true;
  const postGrowthMetric = callAppExport(
    instance,
    memoryViews,
    'boxdd_provider_ray_hit_millimeters',
  );
  if (postGrowthMetric < 0) {
    throw new Error(`provider failed after memory growth with code ${postGrowthMetric}`);
  }
  if (providerGlueCallsAfterGrowth === 0) {
    throw new Error('post-growth physics did not traverse the Emscripten provider exports');
  }
  memoryProof.providerGlueCallsAfterGrowth = providerGlueCallsAfterGrowth;

  const smoke = callAppExport(instance, memoryViews, 'boxdd_provider_smoke');
  if (smoke !== 0) {
    throw new Error(`boxdd provider smoke failed with code ${smoke}`);
  }

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

  const runtimeInit = callAppExport(instance, memoryViews, 'boxdd_runtime_init');
  if (runtimeInit !== 0) {
    throw new Error(`boxdd runtime init failed with code ${runtimeInit}`);
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

  return {
    providerImports: contract.names.length,
    memoryProof,
    linkFailures,
    metrics,
    runtimeBodies,
    runtimeState,
  };
}
