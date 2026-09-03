// LOCAL DIAGNOSTIC ONLY. No hooks are installed unless install() is called.
// Scope workers by the exact FRB 2.13 WorkerPool blob marker, not by all Workers.
(() => {
  let installed = false, poisoned = false, killNext = false, callback;
  let created = 0, stopped = 0, errors = 0, killedDispatch = 0;
  const workers = new Set(), blobs = new WeakSet(), urls = new Set();
  const nativeWorker = globalThis.Worker, nativeBlob = globalThis.Blob;
  const createUrl = URL.createObjectURL.bind(URL);
  function stopAll() {
    for (const worker of workers) { worker.terminate(); stopped++; }
    workers.clear();
  }
  function poison(notify) {
    poisoned = true;
    stopAll();
    if (notify && callback) callback();
  }
  globalThis.scotomaFaultWatch = Object.freeze({
    install(onFault) {
      if (installed) throw new Error('diagnostic_already_started');
      installed = true; callback = onFault;
      globalThis.Blob = class extends nativeBlob {
        constructor(parts, options) {
          super(parts, options);
          if (parts && Array.from(parts).some(p => typeof p === 'string' &&
              p.includes('const FRB_ACTION_PANIC = 3;') && p.includes('receive_transfer_closure'))) {
            blobs.add(this);
          }
        }
      };
      URL.createObjectURL = blob => {
        const url = createUrl(blob);
        if (blobs.has(blob)) urls.add(url);
        return url;
      };
      globalThis.Worker = class extends nativeWorker {
        constructor(url, options) {
          if (urls.has(String(url)) && poisoned) throw new Error('runtime_discarded');
          super(url, options);
          if (!urls.has(String(url))) return;
          workers.add(this); created++;
          this.addEventListener('error', event => {
            // No event.message/error/stack is inspected, logged or forwarded.
            event.preventDefault(); event.stopImmediatePropagation(); errors++;
            poison(true);
          }, {capture: true});
          const post = this.postMessage.bind(this);
          this.postMessage = (...args) => {
            post(...args);
            if (killNext) {
              killNext = false; killedDispatch++;
              // A real outstanding FRB dispatch, then browser Worker.terminate.
              // Silent loss: poison the adapter, but let the Dart deadline fire.
              poison(false);
            }
          };
        }
      };
    },
    armSilentLoss() {
      if (!installed || poisoned || workers.size === 0) throw new Error('no_live_workers');
      killNext = true;
    },
    quarantine() { poison(false); },
    stop() { stopAll(); },
    isPoisoned() { return poisoned; },
    stats() { return JSON.stringify({created, stopped, live: workers.size, errors, killedDispatch, poisoned}); }
  });
})();
