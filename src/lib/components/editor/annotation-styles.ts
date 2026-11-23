import type { Annotation, AnnotationKind, AnnotationStroke } from '$lib/types';

export const DEFAULT_HIGHLIGHT = '#6366f1';
export const DEFAULT_REDACT = '#18181b';
export const DEFAULT_TINT = '#f59e0b';
export const DEFAULT_ARROW = '#ef4444';

export const HIGHLIGHT_COLORS = [
	{ hex: '#6366f1', label: 'Indigo' },
	{ hex: '#ef4444', label: 'Red' },
	{ hex: '#f59e0b', label: 'Amber' },
	{ hex: '#22c55e', label: 'Green' },
	{ hex: '#0ea5e9', label: 'Sky' },
	{ hex: '#ffffff', label: 'White' }
] as const;

export const REDACT_COLORS = [
	{ hex: '#18181b', label: 'Black' },
	{ hex: '#ffffff', label: 'White' },
	{ hex: '#71717a', label: 'Gray' },
	{ hex: '#ef4444', label: 'Red' }
] as const;

export const STROKE_OPTIONS: { value: AnnotationStroke; label: string }[] = [
	{ value: 'thin', label: 'Thin' },
	{ value: 'medium', label: 'Medium' },
	{ value: 'thick', label: 'Thick' }
];

export function defaultColor(kind: AnnotationKind): string | undefined {
	if (kind === 'highlight') return DEFAULT_HIGHLIGHT;
	if (kind === 'redact') return DEFAULT_REDACT;
	if (kind === 'tint') return DEFAULT_TINT;
	if (kind === 'arrow') return DEFAULT_ARROW;
	return undefined;
}

export function resolveColor(annotation: Annotation): string | undefined {
	return annotation.color ?? defaultColor(annotation.kind);
}

export function annotationClasses(annotation: Annotation, selected = false): string {
	const color = resolveColor(annotation);
	const parts = ['group absolute rounded-[3px] transition-shadow'];

	if (annotation.kind === 'blur') {
		parts.push('bg-black/5 backdrop-blur-[7px] backdrop-saturate-50');
	} else if (annotation.kind === 'redact' && color) {
		parts.push('border border-black/10');
	} else if (annotation.kind === 'highlight') {
		const weight =
			annotation.stroke === 'thin'
				? 'border'
				: annotation.stroke === 'thick'
					? 'border-[3px]'
					: 'border-2';
		parts.push(weight);
	}

	if (selected) {
		parts.push('z-10 ring-2 ring-white ring-offset-2 ring-offset-black/70');
	}

	return parts.join(' ');
}

export function annotationStyle(annotation: Annotation): string | undefined {
	const color = resolveColor(annotation);
	if (!color) return undefined;

	if (annotation.kind === 'redact') {
		return `background:${color}`;
	}
	if (annotation.kind === 'highlight') {
		return `border-color:${color};background:${color}22`;
	}
	if (annotation.kind === 'tint') {
		return `background:${color}55`;
	}
	return undefined;
}

export function draftStyle(tool: AnnotationKind, color?: string): string | undefined {
	if (tool === 'highlight' && color) {
		return `border-color:${color};background:${color}33`;
	}
	if (tool === 'redact' && color) {
		return `border-color:${color};background:${color}88`;
	}
	if (tool === 'tint') {
		const fill = color ?? DEFAULT_TINT;
		return `border-color:${fill};background:${fill}55`;
	}
	return undefined;
}
