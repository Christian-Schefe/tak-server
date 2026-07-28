/// <reference lib="webworker" />

import { workerInput, type FullWorkerResponse } from '@/api/engine';
import init, {
  initialize,
  is_settings_supported,
  search_position,
  stop_searching,
} from './tak-wasm-engine/pkg/tak_wasm_engine';
import wasmUrl from './tak-wasm-engine/pkg/tak_wasm_engine_bg.wasm?url';

let initializingPromise: Promise<void> | null = null;

async function assertInit() {
  if (initializingPromise === null) {
    console.log('Initializing engine worker...');
    initializingPromise = init({ module_or_path: wasmUrl }).then(() => {
      initialize();
      console.log('Engine WASM module initialized');
    });
  }
  await initializingPromise;
}

function outputMessage(message: FullWorkerResponse) {
  postMessage(message);
}

addEventListener('message', ({ data }) => {
  void assertInit().then(() => {
    const parsed = workerInput.safeParse(data);
    if (!parsed.success) {
      console.error('Invalid message to worker:', data, parsed.error);
      return;
    }
    const message = parsed.data;

    if (message.type === 'checkSettings') {
      const result = is_settings_supported(JSON.stringify(message.settings));
      outputMessage({ type: 'checkSettings', supported: result });
    } else if (message.type === 'evaluate') {
      search_position(message.key, JSON.stringify(message.game.settings), message.game.tps);
    } else if (message.type === 'initialize') {
      outputMessage({ type: 'initialized' });
    } else {
      stop_searching();
    }
  });
});
