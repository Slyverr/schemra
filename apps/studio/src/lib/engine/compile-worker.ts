import initializeWasm, { compile_project_schemra, compile_schemra } from '../generated/schemra_wasm.js';
import type {
	CompileWorkerMessage,
	CompileWorkerReply,
	CompileWorkerRequest,
	CompileWorkerValue
} from './compile-worker-protocol.js';

interface WorkerScope {
	addEventListener(type: 'message', listener: (event: MessageEvent<CompileWorkerMessage>) => void): void;
	postMessage(message: CompileWorkerReply): void;
}

interface WorkerFailure extends Error {
	stage: 'initialization' | 'operation';
}

const workerScope = self as unknown as WorkerScope;
let wasmReady: Promise<void> | undefined;

workerScope.addEventListener('message', (event) => {
	const { id, request } = event.data;
	void compile(request).then(
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

async function compile(request: CompileWorkerRequest): Promise<{ value: CompileWorkerValue; compileMs: number }> {
	try {
		await initializeEngine();
	} catch (error) {
		throw workerFailure(error, 'initialization');
	}

	const startedAt = performance.now();
	try {
		const value = request.kind === 'source'
			? JSON.parse(compile_schemra(request.source))
			: JSON.parse(compile_project_schemra(JSON.stringify({ sources: request.sources })));
		return { value: value as CompileWorkerValue, compileMs: performance.now() - startedAt };
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
