class FakeWorker extends EventTarget {
  constructor(url) {
    super();
    this.url = url;
    this.stopped = false;
  }

  postMessage(_) {}

  terminate() {
    this.stopped = true;
  }
}

globalThis.Worker = FakeWorker;
await import('../web/runtime_supervisor.js');

let faults = 0;
globalThis.scotomaRuntimeSupervisor.install(() => faults++);
const unrelated = new Worker('unrelated-worker.js');
unrelated.dispatchEvent(new Event('error', { cancelable: true }));
if (unrelated.stopped || faults !== 0) {
  throw new Error('unrelated_worker_was_quarantined');
}

const source = 'const FRB_ACTION_PANIC = 3; receive_transfer_closure';
const blob = new Blob([source], { type: 'text/javascript' });
const url = URL.createObjectURL(blob);
const firstWorker = new Worker(url);
const secondWorker = new Worker(url);
firstWorker.dispatchEvent(new Event('messageerror', { cancelable: true }));
secondWorker.dispatchEvent(new Event('error', { cancelable: true }));

let refused = false;
try {
  new Worker(url);
} catch (_) {
  refused = true;
}

if (
  !firstWorker.stopped ||
  !secondWorker.stopped ||
  faults !== 1 ||
  !refused ||
  !globalThis.scotomaRuntimeSupervisor.isPoisoned()
) {
  throw new Error('runtime_supervisor_failed');
}

console.log('RUNTIME_SUPERVISOR_OK');
