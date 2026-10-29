export interface WorkerRequestMessage<Request> {
	id: number;
	request: Request;
}

export type WorkerRequestReply<Value> =
	| { id: number; ok: true; value: Value; worker_compile_ms: number }
	| {
			id: number;
			ok: false;
			stage: 'initialization' | 'operation';
			name: string;
			error: string;
	  };

export interface WorkerRequestTiming {
	route: 'worker' | 'main-thread-fallback';
	elapsedMs: number;
	workerCompileMs?: number;
}

export interface WorkerRequestOptions {
	signal?: AbortSignal;
	/** Queued requests with the same key replace each other. */
	queueKey?: string;
	onTiming?: (timing: WorkerRequestTiming) => void;
}
