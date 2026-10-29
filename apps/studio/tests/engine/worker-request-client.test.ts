import { describe, expect, it } from 'bun:test';
import { createWorkerRequestClient } from '../../src/lib/engine/worker-request-client.js';
import type {
	WorkerRequestMessage,
	WorkerRequestReply,
	WorkerRequestTiming
} from '../../src/lib/engine/worker-request-protocol.js';

type Request = { value: number };
type Value = { value: number };

class MockWorker {
	onmessage: ((event: MessageEvent<WorkerRequestReply<Value>>) => void) | null = null;
	onerror: ((event: ErrorEvent) => void) | null = null;
	onmessageerror: (() => void) | null = null;
	messages: WorkerRequestMessage<Request>[] = [];
	terminated = false;

	postMessage(message: WorkerRequestMessage<Request>) {
		this.messages.push(message);
	}

	terminate() {
		this.terminated = true;
	}

	reply(index: number, value: Value) {
		const message = this.messages[index];
		if (!message) throw new Error(`No worker request exists at index ${index}.`);
		this.onmessage?.({ data: { id: message.id, ok: true, value, worker_compile_ms: 2 } } as MessageEvent<WorkerRequestReply<Value>>);
	}

	fail(message = 'Worker crashed.') {
		this.onerror?.({ message, preventDefault() {} } as ErrorEvent);
	}
}

function createClient(worker: MockWorker) {
	return createWorkerRequestClient<Request, Value>(() => worker as unknown as Worker);
}

describe('worker request client', () => {
	it('resolves worker replies and reports worker timing', async () => {
		const worker = new MockWorker();
		const client = createClient(worker);
		const timings: WorkerRequestTiming[] = [];
		const pending = client.run({ value: 4 }, { onTiming: (timing) => timings.push(timing) }, async () => ({ value: -1 }));

		expect(worker.messages).toEqual([{ id: 1, request: { value: 4 } }]);
		worker.reply(0, { value: 8 });

		expect(await pending).toEqual({ value: 8 });
		expect(timings).toHaveLength(1);
		expect(timings[0]?.route).toBe('worker');
		expect(timings[0]?.workerCompileMs).toBe(2);
	});

	it('coalesces queued latest-wins requests with the same key', async () => {
		const worker = new MockWorker();
		const client = createClient(worker);
		const active = client.run({ value: 1 }, {}, async () => ({ value: -1 }));
		const superseded = client.run({ value: 2 }, { queueKey: 'preview' }, async () => ({ value: -2 }));
		const latest = client.run({ value: 3 }, { queueKey: 'preview' }, async () => ({ value: -3 }));

		await expect(superseded).rejects.toMatchObject({ name: 'AbortError' });
		expect(worker.messages).toHaveLength(1);
		worker.reply(0, { value: 10 });
		await expect(active).resolves.toEqual({ value: 10 });
		expect(worker.messages[1]?.request).toEqual({ value: 3 });
		worker.reply(1, { value: 30 });
		await expect(latest).resolves.toEqual({ value: 30 });
	});

	it('terminates canceled active worker work before starting the next request', async () => {
		const firstWorker = new MockWorker();
		const nextWorker = new MockWorker();
		const workers = [firstWorker, nextWorker];
		const client = createWorkerRequestClient<Request, Value>(() => {
			const worker = workers.shift();
			if (!worker) throw new Error('No worker remains.');
			return worker as unknown as Worker;
		});
		const controller = new AbortController();
		const active = client.run({ value: 1 }, { signal: controller.signal }, async () => ({ value: -1 }));
		const queued = client.run({ value: 2 }, {}, async () => ({ value: -2 }));

		controller.abort();

		await expect(active).rejects.toMatchObject({ name: 'AbortError' });
		expect(firstWorker.terminated).toBe(true);
		expect(nextWorker.messages).toHaveLength(1);
		nextWorker.reply(0, { value: 20 });
		await expect(queued).resolves.toEqual({ value: 20 });
	});

	it('replays pending requests through the fallback after a worker failure', async () => {
		const worker = new MockWorker();
		const client = createClient(worker);
		const timings: WorkerRequestTiming[] = [];
		const first = client.run({ value: 1 }, { onTiming: (timing) => timings.push(timing) }, async () => ({ value: 11 }));
		const second = client.run({ value: 2 }, { onTiming: (timing) => timings.push(timing) }, async () => ({ value: 22 }));

		worker.fail();
		expect(worker.terminated).toBe(true);
		expect(await Promise.all([first, second])).toEqual([{ value: 11 }, { value: 22 }]);
		expect(timings.map(({ route }) => route)).toEqual(['main-thread-fallback', 'main-thread-fallback']);
	});

	it('rejects canceled and over-capacity queued requests without leaking active work', async () => {
		const worker = new MockWorker();
		const client = createClient(worker);
		const activeController = new AbortController();
		const active = client.run({ value: 0 }, { signal: activeController.signal }, async () => ({ value: -1 }));
		const queued = Array.from({ length: 8 }, (_, index) => {
			const controller = new AbortController();
			const promise = client.run({ value: index + 1 }, { signal: controller.signal, queueKey: `q${index}` }, async () => ({ value: -1 }));
			return { controller, promise };
		});
		const overflow = client.run({ value: 99 }, {}, async () => ({ value: -1 }));

		await expect(overflow).rejects.toMatchObject({ name: 'WorkerQueueFullError' });
		for (const item of queued) item.controller.abort();
		await Promise.all(queued.map(({ promise }) => expect(promise).rejects.toMatchObject({ name: 'AbortError' })));
		activeController.abort();
		await expect(active).rejects.toMatchObject({ name: 'AbortError' });
		worker.reply(0, { value: 0 });
		expect(worker.messages).toHaveLength(1);
	});

	it('uses the main-thread fallback if worker creation is unavailable', async () => {
		const client = createWorkerRequestClient<Request, Value>(() => {
			throw new Error('Not supported.');
		});
		const timings: WorkerRequestTiming[] = [];

		await expect(client.run({ value: 5 }, { onTiming: (timing) => timings.push(timing) }, async () => ({ value: 15 })))
			.resolves.toEqual({ value: 15 });
		expect(timings.map(({ route }) => route)).toEqual(['main-thread-fallback']);
	});
});
