import initializeWasm, { complete_schemra, complete_schemra_with_nodes } from '../generated/schemra_wasm.js';
import type {
	LanguageWorkerMessage,
	LanguageWorkerReply,
	LanguageWorkerRequest,
	LanguageWorkerValue
} from './language-worker-protocol.js';

interface WorkerScope {
	addEventListener(type: 'message', listener: (event: MessageEvent<LanguageWorkerMessage>) => void): void;
	postMessage(message: LanguageWorkerReply): void;
}

interface WorkerFailure extends Error {
	stage: 'initialization' | 'operation';
}

const workerScope = self as unknown as WorkerScope;
let wasmReady: Promise<void> | undefined;

workerScope.addEventListener('message', (event) => {
	const { id, request } = event.data;
	void complete(request).then(
		({ value, compileMs }) =>
			workerScope.postMessage({ id, ok: true, value, worker_compile_ms: compileMs }),
		(error: unknown) => {
			const reason = asError(error);
			workerScope.postMessage({
				id,
				ok: false,
				stage: isWorkerFailure(reason) ? reason.stage : 'operation',
				name: reason.name,
				error: reason.message
			});
		}
	);
});

async function complete(request: LanguageWorkerRequest): Promise<{ value: LanguageWorkerValue; compileMs: number }> {
	try {
		await initializeEngine();
	} catch (error) {
		throw workerFailure(error, 'initialization');
	}

	const startedAt = performance.now();
	try {
		const result = request.include_nodes
			? complete_schemra_with_nodes(
				request.source,
				request.cursor_offset,
				request.explicit,
				JSON.stringify({ node_ids: request.node_ids })
			)
			: complete_schemra(request.source, request.cursor_offset, request.explicit);
		return { value: JSON.parse(result) as LanguageWorkerValue, compileMs: performance.now() - startedAt };
	} catch (error) {
		throw workerFailure(error, 'operation');
	}
}

function initializeEngine(): Promise<void> {
	if (!wasmReady) {
		wasmReady = initializeWasm().then(() => undefined).catch((error: unknown) => {
			wasmReady = undefined;
			throw error;
		});
	}
	return wasmReady;
}

function workerFailure(error: unknown, stage: WorkerFailure['stage']): WorkerFailure {
	const reason = asError(error);
	return Object.assign(new Error(reason.message), { name: reason.name, stage });
}

function isWorkerFailure(error: Error): error is WorkerFailure {
	return 'stage' in error && (error as WorkerFailure).stage !== undefined;
}

function asError(error: unknown): Error {
	return error instanceof Error ? error : new Error(String(error));
}
