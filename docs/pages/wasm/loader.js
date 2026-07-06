const statusPanel = document.querySelector("#runtime-status");
const canvas = document.querySelector("#runtime-canvas");
const ctx = canvas.getContext("2d");
const metrics = {
  drop: document.querySelector("#metric-drop"),
  ray: document.querySelector("#metric-ray"),
  cast: document.querySelector("#metric-cast"),
  joint: document.querySelector("#metric-joint"),
};

function setStatus(state, title, detail) {
  statusPanel.dataset.state = state;
  statusPanel.replaceChildren();
  const titleNode = document.createElement("strong");
  titleNode.textContent = title;
  const detailNode = document.createElement("span");
  detailNode.textContent = detail;
  statusPanel.append(titleNode, detailNode);
}

function generatedUrl(path) {
  return new URL(`generated/${path}`, import.meta.url);
}

function providerFunction(provider, name) {
  const exported = provider[`_${name}`] || provider[name];
  if (typeof exported !== "function") {
    throw new Error(`Box2D provider is missing export ${name}`);
  }
  return exported;
}

async function loadRuntime() {
  setStatus("loading", "Loading Box2D provider", "Creating the shared WebAssembly memory.");
  const memory = new WebAssembly.Memory({ initial: 2048, maximum: 8192 });
  const { default: createProvider } = await import(generatedUrl("box2d-sys-v0.js").href);
  const provider = await createProvider({
    wasmMemory: memory,
    locateFile: (path) => generatedUrl(path).href,
    print: (text) => console.log(`[box2d-sys-v0] ${text}`),
    printErr: (text) => console.warn(`[box2d-sys-v0] ${text}`),
  });

  if (provider.wasmMemory && provider.wasmMemory !== memory) {
    throw new Error("Box2D provider did not use the shared WebAssembly.Memory");
  }

  setStatus("loading", "Loading Rust runtime", "Instantiating the boxdd provider-smoke wasm module.");
  const appBytes = await fetch(generatedUrl("boxdd_provider_smoke.wasm")).then((response) => {
    if (!response.ok) throw new Error(`failed to fetch Rust wasm: ${response.status}`);
    return response.arrayBuffer();
  });
  const importObject = { env: { memory }, "box2d-sys-v0": {} };
  const inspectModule = await WebAssembly.compile(appBytes);
  for (const entry of WebAssembly.Module.imports(inspectModule)) {
    if (entry.kind === "function" && entry.module === "box2d-sys-v0") {
      importObject["box2d-sys-v0"][entry.name] = providerFunction(provider, entry.name);
    }
  }
  const instance = await WebAssembly.instantiate(inspectModule, importObject);
  const exports = instance.exports;
  const smoke = exports.boxdd_provider_smoke();
  if (smoke !== 0) throw new Error(`provider smoke failed with code ${smoke}`);
  const init = exports.boxdd_runtime_init();
  if (init !== 0) throw new Error(`runtime init failed with code ${init}`);
  return exports;
}

function setMetric(node, value, suffix) {
  node.textContent = `${value}${suffix}`;
}

function draw(exports) {
  const width = canvas.width;
  const height = canvas.height;
  ctx.clearRect(0, 0, width, height);
  ctx.fillStyle = "#05080c";
  ctx.fillRect(0, 0, width, height);
  ctx.strokeStyle = "#27313a";
  ctx.lineWidth = 1;
  for (let x = 0; x <= width; x += 48) {
    ctx.beginPath();
    ctx.moveTo(x, 0);
    ctx.lineTo(x, height);
    ctx.stroke();
  }
  for (let y = 0; y <= height; y += 48) {
    ctx.beginPath();
    ctx.moveTo(0, y);
    ctx.lineTo(width, y);
    ctx.stroke();
  }

  const scale = 58;
  const worldToCanvas = (x, y) => [width / 2 + x * scale, height - 92 - y * scale];
  ctx.fillStyle = "#2dd4bf";
  ctx.fillRect(0, height - 92 + scale, width, 8);
  const count = exports.boxdd_runtime_body_count();
  for (let i = 0; i < count; i += 1) {
    const shape = exports.boxdd_runtime_body_shape(i);
    const x = exports.boxdd_runtime_body_x_millimeters(i) / 1000;
    const y = exports.boxdd_runtime_body_y_millimeters(i) / 1000;
    const angle = exports.boxdd_runtime_body_angle_milliradians(i) / 1000;
    const [cx, cy] = worldToCanvas(x, y);
    ctx.save();
    ctx.translate(cx, cy);
    ctx.rotate(-angle);
    if (shape === 2) {
      const radius = exports.boxdd_runtime_body_radius_millimeters(i) / 1000 * scale;
      ctx.fillStyle = "#facc15";
      ctx.beginPath();
      ctx.arc(0, 0, radius, 0, Math.PI * 2);
      ctx.fill();
    } else {
      const hw = exports.boxdd_runtime_body_half_width_millimeters(i) / 1000 * scale;
      const hh = exports.boxdd_runtime_body_half_height_millimeters(i) / 1000 * scale;
      ctx.fillStyle = i % 2 === 0 ? "#38bdf8" : "#a78bfa";
      ctx.fillRect(-hw, -hh, hw * 2, hh * 2);
    }
    ctx.restore();
  }
}

loadRuntime()
  .then((exports) => {
    setMetric(metrics.drop, exports.boxdd_provider_drop_millimeters(), " mm");
    setMetric(metrics.ray, exports.boxdd_provider_ray_hit_millimeters(), " mm");
    setMetric(metrics.cast, exports.boxdd_provider_shape_cast_permyriad(), " / 10000");
    setMetric(metrics.joint, exports.boxdd_provider_joint_error_millimeters(), " mm");
    setStatus("running", "Runtime running", "The canvas is stepping a real Box2D world through Rust wasm.");
    const tick = () => {
      for (let i = 0; i < 2; i += 1) exports.boxdd_runtime_step();
      draw(exports);
      requestAnimationFrame(tick);
    };
    tick();
  })
  .catch((error) => {
    console.error(error);
    const message = error instanceof Error ? error.message : String(error);
    setStatus("error", "Runtime failed", message);
  });
