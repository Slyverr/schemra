<script lang="ts">
	import { onDestroy, tick } from 'svelte';
	import {
		diagnosticQuickFixOptions,
		type RefactorDiagnostic,
		type RefactorSymbol
	} from './editor-refactors.js';
	import type { EditorDiagnostic } from './source-model.js';
	import { studioMessage } from '../i18n/messages.js';
	import { Button } from '../components/ui/button/index.js';
	import { editorClasses, tooltipSeverityClasses } from './editor-classes.js';
	import { cn } from '#lib/utils.js';

	interface TooltipState {
		key: string;
		diagnostic: EditorDiagnostic;
		left: number;
		top: number;
		maxWidth: number;
		maxHeight: number;
	}

	interface Props {
		root?: HTMLDivElement;
		source: string;
		refactorDiagnostics?: readonly RefactorDiagnostic[];
		refactorSymbols?: readonly RefactorSymbol[];
		quickFixDisabled?: boolean;
		onQuickFix?: (diagnostic: RefactorDiagnostic, replacement: string) => void;
	}

	const diagnosticHoverDelayMs = 400;
	const props: Props = $props();
	let tooltipElement = $state<HTMLDivElement>();
	let tooltip = $state<TooltipState | null>(null);
	let tooltipTimer: ReturnType<typeof setTimeout> | undefined;
	let pendingTooltip: TooltipState | null = null;

	const tooltipQuickFixes = $derived.by(() => {
		if (!tooltip) return { diagnostic: null, options: [] };
		const diagnostic = refactorDiagnosticFor(tooltip.diagnostic);
		return {
			diagnostic,
			options: diagnostic
				? diagnosticQuickFixOptions(props.source, props.refactorSymbols ?? [], diagnostic)
				: []
		};
	});

	$effect(() => {
		const root = props.root;
		if (!root || typeof ResizeObserver === 'undefined') return;
		const observer = new ResizeObserver(() => {
			if (tooltip) scheduleTooltipPlacement(tooltip.key);
		});
		observer.observe(root);
		return () => observer.disconnect();
	});

	onDestroy(clear);

	export function show(event: PointerEvent, diagnostic: EditorDiagnostic | null) {
		if (!diagnostic || !props.root) {
			clear();
			return;
		}
		const bounds = props.root.getBoundingClientRect();
		const key = diagnosticKey(diagnostic);
		const target: TooltipState = {
			key,
			diagnostic,
			left: event.clientX - bounds.left + 12,
			top: event.clientY - bounds.top + 14,
			maxWidth: Math.max(120, props.root.clientWidth - 16),
			maxHeight: Math.max(80, props.root.clientHeight - 16)
		};
		if (tooltip?.key === key) {
			tooltip = target;
			scheduleTooltipPlacement(key);
			return;
		}
		if (pendingTooltip?.key === key) {
			pendingTooltip = target;
			return;
		}
		clearTimeout(tooltipTimer);
		tooltip = null;
		pendingTooltip = target;
		tooltipTimer = setTimeout(() => {
			tooltip = pendingTooltip;
			pendingTooltip = null;
			tooltipTimer = undefined;
			if (tooltip?.key === key) scheduleTooltipPlacement(key);
		}, diagnosticHoverDelayMs);
	}

	export function handleLeave(event: PointerEvent) {
		if ((event.relatedTarget as Element | null)?.closest('.editor-diagnostic-tooltip')) return;
		clear();
	}

	export function clear() {
		clearTimeout(tooltipTimer);
		tooltipTimer = undefined;
		pendingTooltip = null;
		tooltip = null;
	}

	function diagnosticKey(diagnostic: EditorDiagnostic) {
		return `${diagnostic.code}:${diagnostic.span.start.line}:${diagnostic.span.start.column}:${diagnostic.span.end.line}:${diagnostic.span.end.column}`;
	}

	function refactorDiagnosticFor(diagnostic: EditorDiagnostic) {
		return props.refactorDiagnostics?.find((candidate) =>
			candidate.code === diagnostic.code &&
			candidate.message === diagnostic.message &&
			candidate.span.start.line === diagnostic.span.start.line &&
			candidate.span.start.column === diagnostic.span.start.column &&
			candidate.span.end.line === diagnostic.span.end.line &&
			candidate.span.end.column === diagnostic.span.end.column
		) ?? null;
	}

	function applyQuickFix(diagnostic: RefactorDiagnostic, replacement: string) {
		clear();
		props.onQuickFix?.(diagnostic, replacement);
	}

	function scheduleTooltipPlacement(key: string) {
		void tick().then(() => {
			if (!tooltip || tooltip.key !== key || !tooltipElement || !props.root) return;
			const elementRect = tooltipElement.getBoundingClientRect();
			const left = Math.max(8, Math.min(tooltip.left, props.root.clientWidth - elementRect.width - 8));
			const top = Math.max(8, Math.min(tooltip.top, props.root.clientHeight - elementRect.height - 8));
			if (left !== tooltip.left || top !== tooltip.top) tooltip = { ...tooltip, left, top };
		});
	}
</script>

{#if tooltip}
	<div
		bind:this={tooltipElement}
		class={cn(
			editorClasses.tooltip,
			tooltip.diagnostic.severity === 'error' && editorClasses.tooltipError,
			`diagnostic-${tooltip.diagnostic.severity}`
		)}
		role="status"
		style:left={`${tooltip.left}px`}
		style:top={`${tooltip.top}px`}
		style:max-width={`${tooltip.maxWidth}px`}
		style:max-height={`${tooltip.maxHeight}px`}
		onpointerleave={handleLeave}
	>
		<strong class={cn(editorClasses.tooltipSeverity, tooltipSeverityClasses[tooltip.diagnostic.severity])}>
			{tooltip.diagnostic.severity}
		</strong>: {tooltip.diagnostic.message}
		{#if tooltip.diagnostic.hint}
			<div class={editorClasses.tooltipHint}>{tooltip.diagnostic.hint}</div>
		{/if}
		{#if tooltipQuickFixes.options.length > 0 && props.onQuickFix && tooltipQuickFixes.diagnostic}
			<div class={editorClasses.quickFixes}>
				<span>{studioMessage('editor.quickFixReplace')}</span>
				{#each tooltipQuickFixes.options as quickFix (`${tooltipQuickFixes.diagnostic.code}-${quickFix.replacement}`)}
					<Button
						variant="outline"
						size="xs"
						class={editorClasses.quickFixButton}
						type="button"
						disabled={props.quickFixDisabled}
						onpointerdown={(event) => event.preventDefault()}
						onclick={(event) => {
							event.stopPropagation();
							if (tooltipQuickFixes.diagnostic) applyQuickFix(tooltipQuickFixes.diagnostic, quickFix.replacement);
						}}
					>{quickFix.replacement}</Button>
				{/each}
			</div>
		{/if}
	</div>
{/if}
