import { createWorkerRequestClient } from './worker-request-client.js';
import type { CompileWorkerRequest, CompileWorkerValue } from './compile-worker-protocol.js';
import { studioMessage } from '../i18n/messages.js';

const client = createWorkerRequestClient<CompileWorkerRequest, CompileWorkerValue>(() => {
	if (typeof window === 'undefined') throw new Error(studioMessage('worker.browserUnavailable'));
	return new Worker(new URL('./compile-worker.ts', import.meta.url), {
		type: 'module',
		name: 'schemra-compiler'
	});
});

export const runCompileWorker = client.run;
