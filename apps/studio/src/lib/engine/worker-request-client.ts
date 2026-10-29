import type {
	WorkerRequestMessage,
	WorkerRequestOptions,
	WorkerRequestReply,
	WorkerRequestTiming
} from './worker-request-protocol.js';

interface PendingRequest<Request> {
	id: number;
	request: Request;
	fallback: () => Promise<unknown>;
	resolve: (value: unknown) => void;
	reject: (error: Error) => void;
	signal?: AbortSignal;
	abortListener?: () => void;
	queueKey?: string;
	startedAt: number;
	onTiming?: WorkerRequestOptions['onTiming'];
	settled: boolean;
}

const MAX_QUEUED_REQUESTS = 8;

/** Serialize requests per worker and fall back to WASM on the main thread when needed. */
export function createWorkerRequestClient<Request, Value>(createWorker: () => Worker) {
	let worker: Worker | undefined;
	let workerUnavailable = false;
	let workerFailure: Error | undefined;
	let nextRequestId = 0;
	let active: PendingRequest<Request> | undefined;
	const queued: PendingRequest<Request>[] = [];

	function run<T extends Value>(
		request: Request,
		options: WorkerRequestOptions,
		fallback: () => Promise<T>
	): Promise<T> {
		if (options.signal?.aborted) return Promise.reject(abortError());

		return new Promise<T>((resolve, reject) => {
			const job: PendingRequest<Request> = {
				id: ++nextRequestId,
				request,
				fallback,
				resolve: (value) => resolve(value as T),
				reject,
				signal: options.signal,
				queueKey: options.queueKey?.trim() || undefined,
				startedAt: now(),
				onTiming: options.onTiming,
				settled: false
			};

			if (job.signal) {
				job.abortListener = () => cancel(job);
				job.signal.addEventListener('abort', job.abortListener, { once: true });
			}

			if (active) enqueue(job);
			else start(job);
		});
	}

	function start(job: PendingRequest<Request>): void {
		const instance = getWorker();
		if (instance) send(job, instance);
		else startFallback(job);
	}

	function enqueue(job: PendingRequest<Request>): void {
		if (job.queueKey) {
			const index = queued.findIndex((pending) => pending.queueKey === job.queueKey);
			if (index >= 0) {
				const [superseded] = queued.splice(index, 1);
				if (superseded) finish(superseded, undefined, abortError('Superseded by a newer request.'));
			}
		}

		if (queued.length >= MAX_QUEUED_REQUESTS) {
			const error = new Error(`The worker queue is full (${MAX_QUEUED_REQUESTS} pending requests).`);
			error.name = 'WorkerQueueFullError';
			finish(job, undefined, error);
			return;
		}
		queued.push(job);
	}

	function getWorker(): Worker | undefined {
		if (workerUnavailable || typeof Worker === 'undefined') {
			workerUnavailable = true;
			return undefined;
		}
		if (worker) return worker;

		try {
			worker = createWorker();
			worker.onmessage = (event: MessageEvent<WorkerRequestReply<Value>>) => receive(event.data);
			worker.onerror = (event) => {
				event.preventDefault();
				failWorker(new Error(event.message || 'The worker stopped unexpectedly.'));
			};
			worker.onmessageerror = () => failWorker(new Error('The worker returned an unreadable response.'));
			return worker;
		} catch (error) {
			workerUnavailable = true;
			workerFailure = asError(error);
			return undefined;
		}
	}

	function send(job: PendingRequest<Request>, instance: Worker): void {
		active = job;
		const message: WorkerRequestMessage<Request> = { id: job.id, request: job.request };
		try {
			instance.postMessage(message);
		} catch (error) {
			failWorker(asError(error));
		}
	}

	function receive(reply: WorkerRequestReply<Value>): void {
		if (!active || active.id !== reply.id) return;
		const job = active;

		if (!reply.ok && reply.stage === 'initialization') {
			const error = new Error(`The worker could not initialize WASM: ${reply.error}`);
			error.name = reply.name;
			failWorker(error);
			return;
		}

		active = undefined;
		if (!job.settled) {
			if (reply.ok) finish(job, reply.value, undefined, 'worker', reply.worker_compile_ms);
			else {
				const error = new Error(reply.error);
				error.name = reply.name;
				finish(job, undefined, error);
			}
		}
		drain();
	}

	function drain(): void {
		if (active || queued.length === 0) return;
		const next = queued.shift();
		if (!next) return;
		if (next.settled) {
			drain();
			return;
		}
		start(next);
	}

	function failWorker(error: Error): void {
		workerFailure = error;
		workerUnavailable = true;
		worker?.terminate();
		worker = undefined;
		const pending = [...(active ? [active] : []), ...queued];
		active = undefined;
		queued.length = 0;
		for (const job of pending) {
			if (!job.settled) queued.push(job);
		}
		drain();
	}

	function startFallback(job: PendingRequest<Request>): void {
		active = job;
		void runFallback(job);
	}

	async function runFallback(job: PendingRequest<Request>): Promise<void> {
		try {
			if (job.settled) return;
			if (job.signal?.aborted) {
				cancel(job);
				return;
			}
			const value = await job.fallback();
			if (job.signal?.aborted) cancel(job);
			else finish(job, value, undefined, 'main-thread-fallback');
		} catch (error) {
			const fallbackError = asError(error);
			if (workerFailure) {
				const contextualError = new Error(
					`The worker failed (${workerFailure.message}) and the main-thread fallback also failed: ${fallbackError.message}`
				);
				contextualError.name = fallbackError.name;
				finish(job, undefined, contextualError);
			} else finish(job, undefined, fallbackError);
		} finally {
			if (active === job) {
				active = undefined;
				drain();
			}
		}
	}

	function cancel(job: PendingRequest<Request>): void {
		if (job.settled) return;
		const index = queued.indexOf(job);
		if (index >= 0) queued.splice(index, 1);
		else if (active === job && worker) {
			worker.terminate();
			worker = undefined;
			active = undefined;
		}
		finish(job, undefined, abortError());
		if (index >= 0 || active === undefined) drain();
	}

	function finish(
		job: PendingRequest<Request>,
		value?: unknown,
		error?: Error,
		route?: WorkerRequestTiming['route'],
		workerCompileMs?: number
	): void {
		if (job.settled) return;
		job.settled = true;
		if (job.abortListener) job.signal?.removeEventListener('abort', job.abortListener);
		if (error) job.reject(error);
		else {
			try {
				job.onTiming?.({
					route: route ?? 'worker',
					elapsedMs: now() - job.startedAt,
					...(workerCompileMs === undefined ? {} : { workerCompileMs })
				});
			} catch {
				// Observability hooks must not affect the request result.
			}
			job.resolve(value);
		}
	}

	return { run };
}

function abortError(message = 'Request was canceled.'): Error {
	const error = new Error(message);
	error.name = 'AbortError';
	return error;
}

function asError(error: unknown): Error {
	return error instanceof Error ? error : new Error(String(error));
}

function now(): number {
	return typeof performance === 'undefined' ? Date.now() : performance.now();
}
