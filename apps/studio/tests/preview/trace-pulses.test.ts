import { describe, expect, test } from 'bun:test';
import { bidirectionalTrace, mergeNearbyTracePulses } from '../../src/lib/preview/trace-pulses';

describe('bidirectional scenario traces', () => {
	test('starts two short segments together at the port', () => {
		const trace = bidirectionalTrace(400, 40, 0);

		expect(trace.dashArray).toBe('10 390');
		expect(trace.forwardOffset).toBe(-40);
		expect(trace.backwardOffset).toBe(-30);
	});

	test('moves both segments away from the port and meets opposite it', () => {
		const trace = bidirectionalTrace(400, 40, 1);

		expect(trace.dashArray).toBe('10 390');
		expect(trace.forwardOffset).toBe(-240);
		expect(trace.backwardOffset).toBe(170);
	});

	test('clamps progress and wraps the origin around the outline', () => {
		const trace = bidirectionalTrace(400, 440, 2);

		expect(trace.dashArray).toBe('10 390');
		expect(trace.forwardOffset).toBe(-240);
		expect(trace.backwardOffset).toBe(170);
	});

	test('returns no dashes for an invalid outline length', () => {
		expect(bidirectionalTrace(0, 0, 0.5)).toEqual({
			dashArray: null,
			forwardOffset: 0,
			backwardOffset: 0
		});
	});
});

describe('trace pulse coalescing', () => {
	test('merges nearby pulses from one port while preserving separated and other-port pulses', () => {
		const pulses = mergeNearbyTracePulses([
			{ id: 'first', originKey: 'node:left', progress: 0.4, pulseOpacity: 0.7 },
			{ id: 'second', originKey: 'node:left', progress: 0.42, pulseOpacity: 0.8 },
			{ id: 'third', originKey: 'node:left', progress: 0.8, pulseOpacity: 1 },
			{ id: 'other-port', originKey: 'node:right', progress: 0.41, pulseOpacity: 0.9 }
		]);

		expect(pulses).toHaveLength(3);
		const merged = pulses.find(({ count }) => count === 2);
		expect(merged?.progress).toBeCloseTo(0.41);
		expect(merged).toMatchObject({ pulseOpacity: 1, count: 2, sourceIds: ['first', 'second'] });
		expect(pulses.some(({ id }) => id === 'third')).toBe(true);
		expect(pulses.some(({ id }) => id === 'other-port')).toBe(true);
	});

	test('limits crowded arrivals at one port to four visible pulses', () => {
		const pulses = mergeNearbyTracePulses(
			Array.from({ length: 7 }, (_, index) => ({
				id: `pulse-${index}`,
				originKey: 'node:left',
				progress: index / 6,
				pulseOpacity: 0.5
			}))
		);

		expect(pulses).toHaveLength(4);
		expect(pulses.reduce((count, pulse) => count + pulse.count, 0)).toBe(7);
	});
});
