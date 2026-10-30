<script lang="ts">
	import { Switch } from '../components/ui/switch/index.js';
	import { Label } from '../components/ui/label/index.js';
	import { preferenceRowVariants, settingsClasses } from './settings-classes.js';

	let {
		id,
		label,
		description,
		checked,
		variant = 'card',
		onCheckedChange
	}: {
		id: string;
		label: string;
		description: string;
		checked: boolean;
		variant?: 'card' | 'inline';
		onCheckedChange: (checked: boolean) => void;
	} = $props();
</script>

<Label for={id} class={preferenceRowVariants({ variant })}>
	<span class={settingsClasses.preferenceCopy}>
		<span id={`${id}-label`} class={settingsClasses.preferenceTitle}>{label}</span>
		<span id={`${id}-description`} class={settingsClasses.preferenceDescription}>{description}</span>
	</span>
	<Switch {id} aria-labelledby={`${id}-label`} aria-describedby={`${id}-description`} {checked} {onCheckedChange} />
</Label>
