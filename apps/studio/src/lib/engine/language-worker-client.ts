import { createWorkerRequestClient } from './worker-request-client.js';
import type { LanguageWorkerRequest, LanguageWorkerValue } from './language-worker-protocol.js';

const client = createWorkerRequestClient<LanguageWorkerRequest, LanguageWorkerValue>(() => {
	if (typeof window === 'undefined') throw new Error('Browser workers are unavailable.');
	return new Worker(new URL('./language-worker.ts', import.meta.url), {
		type: 'module',
		name: 'schemra-language-service'
	});
});

export const runLanguageWorker = client.run;
