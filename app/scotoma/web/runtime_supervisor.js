// Production worker quarantine for the pinned FRB 2.13.0 runtime.
// It handles no credential/message values and never inspects worker errors.
(() => {
  let installed = false;
  let poisoned = false;
  let onFault = null;
  const workers = new Set();
  const bridgeBlobs = new WeakSet();
  const bridgeUrls = new Set();
  const NativeWorker = globalThis.Worker;
  const NativeBlob = globalThis.Blob;
  const createObjectUrl = URL.createObjectURL.bind(URL);

  function quarantine(notify) {
    if (poisoned) return;
    poisoned = true;
    for (const worker of workers) worker.terminate();
    workers.clear();
    if (notify && onFault) {
      try { onFault(); } catch (_) { /* Fixed fail-stop policy. */ }
    }
  }

  function suppressAndQuarantine(event) {
    event.preventDefault();
    event.stopImmediatePropagation();
    quarantine(true);
  }

  globalThis.scotomaRuntimeSupervisor = Object.freeze({
    install(callback) {
      if (installed || typeof callback !== 'function') {
        throw new Error('runtime_supervisor_unavailable');
      }
      installed = true;
      onFault = callback;

      globalThis.Blob = class extends NativeBlob {
        constructor(parts, options) {
          super(parts, options);
          if (parts && Array.from(parts).some(part =>
              typeof part === 'string' &&
              part.includes('const FRB_ACTION_PANIC = 3;') &&
              part.includes('receive_transfer_closure'))) {
            bridgeBlobs.add(this);
          }
        }
      };

      URL.createObjectURL = blob => {
        const url = createObjectUrl(blob);
        if (bridgeBlobs.has(blob)) bridgeUrls.add(url);
        return url;
      };

      globalThis.Worker = class extends NativeWorker {
        constructor(url, options) {
          const bridgeWorker = bridgeUrls.has(String(url));
          if (bridgeWorker && poisoned) {
            throw new Error('runtime_discarded');
          }
          super(url, options);
          if (!bridgeWorker) return;

          workers.add(this);
          this.addEventListener('error', suppressAndQuarantine, {
            capture: true,
          });
          this.addEventListener('messageerror', suppressAndQuarantine, {
            capture: true,
          });
        }
      };
    },

    isPoisoned() { return poisoned; },
  });
})();
