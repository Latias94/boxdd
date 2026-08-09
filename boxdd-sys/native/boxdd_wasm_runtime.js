Module["boxddRefreshMemoryViews"] = () => {
  if (Module["HEAPU8"].buffer !== wasmMemory.buffer) {
    updateMemoryViews();
  }
  return Module["HEAPU8"];
};
