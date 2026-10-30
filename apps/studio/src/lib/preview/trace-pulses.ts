export interface TracePulseState {
	id: string;
	originKey: string;
	progress: number;
	pulseOpacity: number;
}

export interface MergedTracePulse extends TracePulseState {
	count: number;
	sourceIds: string[];
}

export interface BidirectionalTrace {
	dashArray: string | null;
	forwardOffset: number;
	backwardOffset: number;
}

const TRACE_PULSE_LENGTH = 0.025;
const TRACE_TRAVEL_DISTANCE = 0.5;
const TRACE_MERGE_DISTANCE = TRACE_PULSE_LENGTH * TRACE_TRAVEL_DISTANCE * 2;
const MAX_DISTINCT_TRACE_PULSES = 4;

function averageProgress(cluster: TracePulseState[]) {
	return cluster.reduce((sum, pulse) => sum + pulse.progress, 0) / cluster.length;
}

/** Combine crowded trace pulses from one node port while keeping separated arrivals distinct. */
export function mergeNearbyTracePulses(pulses: TracePulseState[]): MergedTracePulse[] {
	const byOrigin = new Map<string, TracePulseState[]>();
	for (const pulse of pulses) {
		const bucket = byOrigin.get(pulse.originKey) ?? [];
		bucket.push(pulse);
		byOrigin.set(pulse.originKey, bucket);
	}

	const result: MergedTracePulse[] = [];
	for (const [originKey, group] of byOrigin) {
		group.sort((a, b) => a.progress - b.progress || a.id.localeCompare(b.id));
		const clusters: TracePulseState[][] = [];
		for (const pulse of group) {
			const previous = clusters.at(-1);
			if (previous && pulse.progress - previous.at(-1)!.progress <= TRACE_MERGE_DISTANCE) {
				previous.push(pulse);
			} else {
				clusters.push([pulse]);
			}
		}

		while (clusters.length > MAX_DISTINCT_TRACE_PULSES) {
			let nearestIndex = 0;
			let nearestDistance = Number.POSITIVE_INFINITY;
			for (let index = 0; index < clusters.length - 1; index += 1) {
				const distance = clusters[index + 1]![0]!.progress - clusters[index]!.at(-1)!.progress;
				if (distance < nearestDistance) {
					nearestIndex = index;
					nearestDistance = distance;
				}
			}
			clusters.splice(nearestIndex, 2, [...clusters[nearestIndex]!, ...clusters[nearestIndex + 1]!]);
			clusters.sort((a, b) => averageProgress(a) - averageProgress(b));
		}

		for (const cluster of clusters) {
			const count = cluster.length;
			result.push({
				id: count === 1 ? cluster[0]!.id : `${originKey}:${cluster.map(({ id }) => id).join('+')}`,
				originKey,
				progress: averageProgress(cluster),
				pulseOpacity: Math.min(1, cluster.reduce((sum, pulse) => sum + pulse.pulseOpacity, 0)),
				count,
				sourceIds: cluster.map(({ id }) => id)
			});
		}
	}

	return result;
}

/** Build two short segments that start at a node port and travel around its outline in opposite directions. */
export function bidirectionalTrace(
	pathLength: number,
	originLength: number,
	progress: number
): BidirectionalTrace {
	const length = Math.max(0, Number.isFinite(pathLength) ? pathLength : 0);
	const amount = Number.isFinite(progress) ? Math.max(0, Math.min(1, progress)) : 0;
	const origin = length > 0 && Number.isFinite(originLength)
		? ((originLength % length) + length) % length
		: 0;
	if (length === 0) return { dashArray: null, forwardOffset: 0, backwardOffset: 0 };

	const segmentLength = length * TRACE_PULSE_LENGTH;
	const travelDistance = length * TRACE_TRAVEL_DISTANCE * amount;
	return {
		dashArray: `${segmentLength} ${Math.max(0.001, length - segmentLength)}`,
		forwardOffset: -(origin + travelDistance),
		backwardOffset: segmentLength - origin + travelDistance
	};
}
