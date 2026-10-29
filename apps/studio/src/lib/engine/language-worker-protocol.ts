import type { CompletionItem } from '../schemra.js';
import type { WorkerRequestMessage, WorkerRequestReply } from './worker-request-protocol.js';

export interface LanguageWorkerRequest {
	kind: 'complete';
	source: string;
	cursor_offset: number;
	explicit: boolean;
	node_ids: string[];
	include_nodes: boolean;
}

export type LanguageWorkerValue = { start: number; end: number; items: CompletionItem[] } | null;
export type LanguageWorkerMessage = WorkerRequestMessage<LanguageWorkerRequest>;
export type LanguageWorkerReply = WorkerRequestReply<LanguageWorkerValue>;
