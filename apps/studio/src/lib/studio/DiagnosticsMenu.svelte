<script lang="ts">
	import { Popover } from 'bits-ui';
	import { AlertCircle } from '@lucide/svelte';
	import { Button } from '../components/ui/button/index.js';
	import { studioMessage } from '../i18n/messages.js';
	import type { EditorDiagnostic } from '../editor/source-model.js';

	interface Props {
		activeDiagnostics: readonly EditorDiagnostic[];
		activeFileId: string | null;
		linkedDiagnostics: readonly EditorDiagnostic[];
		linkedFileId: string | null;
		compileError: string | null;
		errorCount: number;
		onNavigate: (fileId: string, diagnostic: EditorDiagnostic) => void;
	}

	let {
		activeDiagnostics,
		activeFileId,
		linkedDiagnostics,
		linkedFileId,
		compileError,
		errorCount,
		onNavigate
	}: Props = $props();

	let open = $state(false);
	const count = $derived(activeDiagnostics.length + linkedDiagnostics.length + Number(Boolean(compileError)));
	const diagnosticLabel = $derived(studioMessage(count === 1 ? 'diagnostics.single' : 'diagnostics.multiple'));
	const menuItemClass = 'flex h-auto w-full items-start justify-start gap-[.55rem] rounded-[.35rem] p-[.38rem] text-left text-[.69rem] leading-[1.4] font-normal whitespace-normal hover:bg-accent hover:text-accent-foreground';

	function navigate(fileId: string | null, diagnostic: EditorDiagnostic) {
		if (!fileId) return;
		open = false;
		onNavigate(fileId, diagnostic);
	}

	function locationLabel(diagnostic: EditorDiagnostic) {
		return diagnostic.span.start.line + ':' + diagnostic.span.start.column;
	}
</script>

{#if count > 0}
	<Popover.Root bind:open>
		<Popover.Trigger>
			{#snippet child({ props })}
				<Button
					{...props}
					variant="ghost"
					size="xs"
					class="h-6 gap-[.3rem] rounded-[.35rem] px-[.35rem] text-[.69rem] font-normal text-muted-foreground hover:bg-muted hover:text-foreground aria-expanded:bg-muted aria-expanded:text-foreground"
					type="button"
					aria-label={studioMessage('diagnostics.button', { count, label: diagnosticLabel })}
					aria-controls="studio-diagnostics-panel"
					title={studioMessage('diagnostics.open')}
				>
					<AlertCircle size={13} class={errorCount > 0 || Boolean(compileError) ? 'size-[13px] text-destructive' : 'size-[13px]'} aria-hidden="true" />
					<span>{count} {diagnosticLabel}</span>
				</Button>
			{/snippet}
		</Popover.Trigger>
		<Popover.Portal>
			<Popover.Content
				id="studio-diagnostics-panel"
				side="bottom"
				align="start"
				sideOffset={8}
				collisionPadding={8}
				class="!z-[1000] grid max-h-[min(24rem,calc(100dvh-8rem))] w-[min(21rem,calc(100vw-2rem))] gap-[.55rem] overflow-auto rounded-[.6rem] border border-border bg-popover p-[.65rem] text-popover-foreground shadow-[0_.65rem_1.8rem_rgb(7_21_41_/_16%)]"
				role="dialog"
				aria-label={studioMessage('diagnostics.title')}
			>
				<h2 class="m-0 text-[.72rem] font-[650] text-foreground">{studioMessage('diagnostics.title')}</h2>
				{#if compileError}<p class="m-0 text-[.69rem] leading-[1.45] text-destructive" role="alert">{compileError}</p>{/if}
				{#if activeDiagnostics.length > 0}
					<section class="grid gap-[.3rem]" aria-label={studioMessage('diagnostics.currentFile')}>
						<h3 class="m-0 text-[.65rem] font-[650] text-muted-foreground">{studioMessage('diagnostics.currentFile')}</h3>
						<ul class="m-0 grid list-none gap-[.15rem] p-0">
							{#each activeDiagnostics as diagnostic, index (diagnostic.code + '-' + index)}
								<li>
									<Button variant="ghost" class={menuItemClass} type="button" onclick={() => navigate(activeFileId, diagnostic)}>
										<span class="shrink-0 font-mono text-[.62rem] text-muted-foreground">{locationLabel(diagnostic)}</span>
										<span class="grid min-w-0 gap-[.16rem] text-left">
											<span>{diagnostic.message}</span>
											{#if diagnostic.hint}<span class="text-muted-foreground">{diagnostic.hint}</span>{/if}
										</span>
									</Button>
								</li>
							{/each}
						</ul>
					</section>
				{/if}
				{#if linkedDiagnostics.length > 0 && linkedFileId}
					<section class="grid gap-[.3rem]" aria-label={studioMessage('diagnostics.linkedVisual')}>
						<h3 class="m-0 text-[.65rem] font-[650] text-muted-foreground">{studioMessage('diagnostics.linkedVisual')}</h3>
						<ul class="m-0 grid list-none gap-[.15rem] p-0">
							{#each linkedDiagnostics as diagnostic, index (diagnostic.code + '-' + index)}
								<li>
									<Button variant="ghost" class={menuItemClass} type="button" onclick={() => navigate(linkedFileId, diagnostic)}>
										<span class="shrink-0 font-mono text-[.62rem] text-muted-foreground">{locationLabel(diagnostic)}</span>
										<span class="grid min-w-0 gap-[.16rem] text-left">
											<span>{diagnostic.message}</span>
											{#if diagnostic.hint}<span class="text-muted-foreground">{diagnostic.hint}</span>{/if}
										</span>
									</Button>
								</li>
							{/each}
						</ul>
					</section>
				{/if}
			</Popover.Content>
		</Popover.Portal>
	</Popover.Root>
{/if}
