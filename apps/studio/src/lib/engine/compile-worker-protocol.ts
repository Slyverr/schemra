import type { CompileResult, ProjectCompileResult, ProjectSource } from '../schemra.js';
import type { WorkerRequestMessage, WorkerRequestReply } from './worker-request-protocol.js';

export type CompileWorkerRequest =
	| { kind: 'source'; source: string }
	| { kind: 'project'; sources: ProjectSource[] };

export type CompileWorkerValue = CompileResult | ProjectCompileResult;
export type CompileWorkerMessage = WorkerRequestMessage<CompileWorkerRequest>;
export type CompileWorkerReply = WorkerRequestReply<CompileWorkerValue>;
