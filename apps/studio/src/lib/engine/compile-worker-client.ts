import { createWorkerRequestClient } from './worker-request-client.js';
import type { CompileWorkerRequest, CompileWorkerValue } from './compile-worker-protocol.js';

const client = createWorkerRequestClient<CompileWorkerRequest, CompileWorkerValue>(() => {
	if (typeof window === 'undefined') throw new Error('Browser workers are unavailable.');
	return new Worker(new URL('./compile-worker.ts', import.meta.url), {
		type: 'module',
		name: 'schemra-compiler'
	});
});

export const runCompileWorker = client.run;
