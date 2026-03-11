/**
 * Project Wizard types
 */

export type PresetId = 'video-analyst' | 'document-reporter' | 'quick-search' | 'custom';
export type ToolId = 'clip' | 'screenshot' | 'transcript' | 'visual' | 'summary';
export type WizardContext = 'onboarding' | 'app';

export interface ProjectData {
	name: string;
	description: string;
	tags: string[];
	files: UploadedFile[];
	selectedPreset: PresetId;
	expandedPreset: PresetId | null;
	enabledTools: ToolId[];
}

export interface UploadedFile {
	id: string;
	name: string;
	size: number;
	progress: number; // 0-100
}

export interface ModelInfo {
	icon: string;
	name: string;
	role: string;
}

export interface AgentPreset {
	id: PresetId;
	name: string;
	badge: 'RECOMMENDED' | 'FAST' | 'ADVANCED' | null;
	tagline: string;
	capabilities: string[];
	models: ModelInfo[];
	downloadSize: string;
	requirements: string;
	isCustom?: boolean;
}

export interface Tool {
	id: ToolId;
	name: string;
	description: string;
}

export interface WizardStep {
	id: string;
	label: string;
}

// Default project data for wizard initialization
export const defaultProjectData: ProjectData = {
	name: '',
	description: '',
	tags: [],
	files: [],
	selectedPreset: 'video-analyst',
	expandedPreset: null,
	enabledTools: ['clip', 'screenshot', 'transcript']
};

// Wizard steps configuration
export const wizardSteps: WizardStep[] = [
	{ id: 'details', label: 'Details' },
	{ id: 'videos', label: 'Videos' },
	{ id: 'models', label: 'Models' },
	{ id: 'tools', label: 'Tools' },
	{ id: 'review', label: 'Review' }
];

// Agent presets configuration
export const agentPresets: AgentPreset[] = [
	{
		id: 'video-analyst',
		name: 'Video Analyst',
		badge: 'RECOMMENDED',
		tagline:
			'Comprehensive video understanding with object detection, scene analysis, and intelligent clip extraction.',
		capabilities: [
			'Find specific moments, objects, and people',
			'Detect and track objects across frames',
			'Analyze body language and gestures',
			'Auto-extract relevant clips to folders',
			"Search by describing what you're looking for"
		],
		models: [
			{ icon: '🧠', name: 'Qwen2.5-VL-3B', role: 'Scene understanding + reasoning' },
			{ icon: '📦', name: 'YOLOv8-n', role: 'Object detection (people, items)' },
			{ icon: '🏃', name: 'RTMPose-m', role: 'Pose estimation + gestures' },
			{ icon: '🔍', name: 'CLIP ViT-B/32', role: 'Visual search embeddings' }
		],
		downloadSize: '4.2 GB',
		requirements: 'M1+ Mac, 8GB RAM'
	},
	{
		id: 'document-reporter',
		name: 'Document & Meeting Reporter',
		badge: null,
		tagline:
			'Extract information from presentations, meetings, and generate formatted reports with your findings.',
		capabilities: [
			'Read slides, whiteboards, and documents in video',
			'Extract tables, charts, and diagrams',
			'Transcribe speech with speaker identification',
			'Generate meeting summaries and action items',
			'Export to PDF reports or slide decks'
		],
		models: [
			{ icon: '📄', name: 'SmolDockling', role: 'Document layout + OCR' },
			{ icon: '🎙️', name: 'Whisper-base.en', role: 'Speech transcription' },
			{ icon: '🧠', name: 'Gemma-3-4B', role: 'Reasoning + summarization' },
			{ icon: '🔍', name: 'CLIP ViT-B/32', role: 'Visual search embeddings' }
		],
		downloadSize: '5.8 GB',
		requirements: 'M1+ Mac, 16GB RAM'
	},
	{
		id: 'quick-search',
		name: 'Quick Search',
		badge: 'FAST',
		tagline: 'Fast visual search without heavy analysis. Perfect for finding specific moments quickly.',
		capabilities: [
			'Search videos by describing scenes',
			'Find similar frames across your library',
			'Basic transcription search',
			'Quick clip extraction'
		],
		models: [
			{ icon: '🔍', name: 'CLIP ViT-B/32', role: 'Visual search embeddings' },
			{ icon: '🎙️', name: 'Whisper-tiny.en', role: 'Fast basic transcription' }
		],
		downloadSize: '500 MB',
		requirements: 'Any Mac'
	},
	{
		id: 'custom',
		name: 'Configure Custom Agent',
		badge: 'ADVANCED',
		tagline:
			'Build your own model stack. Select individual models and configure how they work together.',
		capabilities: [
			'Specific domain expertise (medical, legal, etc.)',
			"Custom detection models you've trained",
			'Specialized workflows not covered by presets',
			'API-based models (GPT-4V, Claude Vision)'
		],
		models: [],
		downloadSize: 'Varies',
		requirements: 'Depends on selection',
		isCustom: true
	}
];

// Available tools configuration
export const availableTools: Tool[] = [
	{
		id: 'clip',
		name: 'Clip Extraction',
		description: 'Extract video segments based on timestamps'
	},
	{
		id: 'screenshot',
		name: 'Screenshot Capture',
		description: 'Capture frames at specific moments'
	},
	{
		id: 'transcript',
		name: 'Transcript Search',
		description: 'Search through video transcripts and captions'
	},
	{
		id: 'visual',
		name: 'Visual Search',
		description: 'Find moments by visual content description'
	},
	{
		id: 'summary',
		name: 'Summary Generation',
		description: 'Generate summaries and key takeaways'
	}
];
