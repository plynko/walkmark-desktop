<script lang="ts">
	import Icon from "@iconify/svelte";
	import { getVersion } from "@tauri-apps/api/app";
	import { openUrl } from "@tauri-apps/plugin-opener";
	import { version as buildVersion } from "../../../package.json";
	import appIcon from "../../../src-tauri/icons/128x128.png";
	import { APP_NAME, REPOSITORY_URL } from "$lib/brand";
	import { store } from "$lib/store.svelte";
	import { Button } from "$lib/components/ui/button";
	import { Label } from "$lib/components/ui/label";
	import { Slider } from "$lib/components/ui/slider";
	import { Tabs, TabsContent, TabsList, TabsTrigger } from "$lib/components/ui/tabs";
	import {
		Dialog,
		DialogContent,
		DialogFooter,
		DialogHeader,
		DialogTitle,
	} from "$lib/components/ui/dialog";
	import { ToggleGroup, ToggleGroupItem } from "$lib/components/ui/toggle-group";
	import ModelPicker from "$lib/components/ModelPicker.svelte";
	import RecordingPreferences from "$lib/components/RecordingPreferences.svelte";
	import WritingPreferences from "$lib/components/WritingPreferences.svelte";

	let { open = false, onClose }: { open?: boolean; onClose?: () => void } = $props();

	function handleOpenChange(value: boolean) {
		if (!value) onClose?.();
	}

	let tab = $state("model");
	let version = $state(buildVersion);

	$effect(() => {
		if (open) tab = store.settingsTab;
	});

	$effect(() => {
		if (!open || tab !== "about") return;
		void getVersion()
			.then((value) => { if (value) version = value; })
			.catch(() => {});
	});

	function openRepository(event: MouseEvent) {
		event.preventDefault();
		void openUrl(REPOSITORY_URL).catch((error) => store.reportError(error, "Could not open the repository"));
	}
</script>

<Dialog {open} onOpenChange={handleOpenChange}>
	<DialogContent class="flex h-[min(680px,86vh)] w-full max-w-3xl flex-col gap-0 overflow-hidden p-0">
		<DialogHeader class="shrink-0 border-b border-(--text)/8 px-6 py-5">
			<DialogTitle>Settings</DialogTitle>
		</DialogHeader>

		<Tabs bind:value={tab} class="flex min-h-0 flex-1 flex-row overflow-hidden">
			<TabsList
				class="h-full w-[168px] shrink-0 flex-col items-stretch gap-0.5 overflow-y-auto rounded-none border-r border-(--text)/8 bg-transparent p-3"
			>
				<TabsTrigger value="model" class="w-full justify-start gap-2 rounded-2xl px-3 py-2">
					<Icon icon="lucide:sparkles" class="size-4 shrink-0 opacity-60" />
					Model
				</TabsTrigger>
				<TabsTrigger value="writing" class="w-full justify-start gap-2 rounded-2xl px-3 py-2">
					<Icon icon="lucide:pen-line" class="size-4 shrink-0 opacity-60" />
					Writing
				</TabsTrigger>
				<TabsTrigger value="recording" class="w-full justify-start gap-2 rounded-2xl px-3 py-2">
					<Icon icon="lucide:circle-dot" class="size-4 shrink-0 opacity-60" />
					Recording
				</TabsTrigger>
				<TabsTrigger value="appearance" class="w-full justify-start gap-2 rounded-2xl px-3 py-2">
					<Icon icon="lucide:palette" class="size-4 shrink-0 opacity-60" />
					Appearance
				</TabsTrigger>
				<TabsTrigger value="about" class="w-full justify-start gap-2 rounded-2xl px-3 py-2">
					<Icon icon="lucide:info" class="size-4 shrink-0 opacity-60" aria-hidden="true" />
					About
				</TabsTrigger>
			</TabsList>

			<div class="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden">
				<div class="min-h-0 flex-1 overflow-y-auto px-6 py-5">
					<TabsContent value="model" class="mt-0 space-y-4 pb-2">
						<ModelPicker />
						<div class="space-y-1.5 border-t border-(--text)/8 pt-4">
							<Label>Concurrency — {store.settings.concurrency}</Label>
							<p class="text-xs text-(--text)/56">
								How many steps the model writes at once. Lower this if requests time out.
							</p>
							<Slider
								type="single"
								value={store.settings.concurrency}
								min={1}
								max={8}
								step={1}
								disabled={store.settings.provider === "ollama"}
								onValueChange={(v: number) => void store.updateSettings({ concurrency: v })}
							/>
						</div>
					</TabsContent>

					<TabsContent value="writing" class="mt-0 pb-4">
						<WritingPreferences />
					</TabsContent>

					<TabsContent value="recording" class="mt-0 pb-4">
						<RecordingPreferences />
					</TabsContent>

					<TabsContent value="appearance" class="mt-0 space-y-4 pb-4">
						<div class="space-y-1.5">
							<Label>Theme</Label>
							<ToggleGroup
								type="single"
								value={store.settings.theme}
								onValueChange={(v) =>
									v && void store.updateSettings({ theme: v as typeof store.settings.theme })}
							>
								<ToggleGroupItem value="system">System</ToggleGroupItem>
								<ToggleGroupItem value="light">Light</ToggleGroupItem>
								<ToggleGroupItem value="dark">Dark</ToggleGroupItem>
							</ToggleGroup>
						</div>
					</TabsContent>

					<TabsContent value="about" class="mt-0 pb-4">
						<div class="flex items-center gap-4 pb-5">
							<img src={appIcon} alt="" width="64" height="64" class="size-16 shrink-0" />
							<div class="min-w-0">
								<h2 class="text-xl font-semibold">{APP_NAME}</h2>
								<p class="text-sm text-(--text)/56">Version {version}</p>
							</div>
						</div>
						<div class="border-t border-(--text)/8 pt-4">
							<p class="mb-1 text-xs font-medium text-(--text)/56">Source repository</p>
							<a
								href={REPOSITORY_URL}
								onclick={openRepository}
								class="inline-flex max-w-full items-baseline gap-1.5 rounded-sm text-sm underline decoration-(--text)/30 underline-offset-2 hover:decoration-current focus-visible:outline-2"
							>
								<span class="break-all">{REPOSITORY_URL}</span>
								<Icon icon="lucide:external-link" class="size-3 shrink-0" aria-hidden="true" />
							</a>
						</div>
					</TabsContent>
				</div>
			</div>
		</Tabs>

		<DialogFooter class="shrink-0 border-t border-(--text)/8 px-6 py-4">
			<Button onclick={() => onClose?.()}>Done</Button>
		</DialogFooter>
	</DialogContent>
</Dialog>
